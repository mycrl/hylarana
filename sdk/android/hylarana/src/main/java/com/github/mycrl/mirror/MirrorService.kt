package com.github.mycrl.hylarana

import android.media.AudioRecord
import android.media.AudioTrack
import android.util.Log
import android.view.Surface
import kotlin.Exception

interface HylaranaAdapterConfigure {
    val video: Video.VideoEncoder.VideoEncoderConfigure
    val audio: Audio.AudioEncoder.AudioEncoderConfigure
}

abstract class HylaranaReceiver {

    /**
     *  You need to provide a surface to the receiver, which will decode and render the received
     *  video stream to this surface.
     */
    abstract val surface: Surface

    /**
     * You need to provide an audio track to the receiver, which will decode the received audio
     * stream and play it using this audio track.
     */
    abstract val track: AudioTrack?

    /**
     * You can choose to implement this function, and the underlying transport layer will give you a c
     * opy of the audio and video data, with the `kind` parameter indicating the type of packet.
     */
    open fun sink(buf: ByteArray, kind: Int) {}

    /**
     * Called when the receiver is closed, the likely reason is because the underlying transport
     * layer has been disconnected, perhaps because the sender has been closed or the network is
     * disconnected.
     */
    open fun released() {}

    /**
     * Called when the receiver is created, this will pass you a wrapper for the underlying adapter,
     * and you can actively release this receiver by calling the release method of the adapter.
     */
    open fun onStart(adapter: ReceiverAdapterWrapper) {}

    /**
     * For the receiving side, this function will be called back when the sending side is created.
     */
    open fun onLine() {}
}

/**
 * Create a hylarana service, note that observer can be null, when observer is null, it will not
 * automatically respond to any sender push.
 */
class HylaranaService(
    server: String,
    multicast: String,
    mtu: Int,
) {
    private val hylarana: Hylarana = Hylarana(server, multicast, mtu)

    /**
     * Release this hylarana instance.
     */
    fun release() {
        hylarana.release()
    }

    /**
     * Creates an instance of a sender with an unlimited `id` parameter, this id is passed to all
     * receivers and is mainly used to provide receivers with identification of this sender.
     */
    fun createSender(
        id: Int,
        configure: HylaranaAdapterConfigure,
        record: AudioRecord?
    ): HylaranaSender {
        return HylaranaSender(
            hylarana.createSender(id),
            configure,
            record,
        )
    }

    /**
     * Creating a receiver and connecting to a specific sender results in a more proactive connection
     * than auto-discovery, and the handshake will take less time.
     *
     * `port` The port number from the created sender.
     */
    fun createReceiver(
        id: Int,
        configure: HylaranaAdapterConfigure,
        observer: HylaranaReceiver
    ) {
        var adapter: ReceiverAdapterWrapper? = null
        adapter = hylarana.createReceiver(id, object : ReceiverAdapter() {
            private var isReleased: Boolean = false
            private val videoDecoder = Video.VideoDecoder(
                observer.surface,
                object : Video.VideoDecoder.VideoDecoderConfigure {
                    override val height = configure.video.height
                    override val width = configure.video.width
                })

            private val audioDecoder = if (observer.track != null) {
                Audio.AudioDecoder(
                    observer.track!!,
                    object : Audio.AudioDecoder.AudioDecoderConfigure {
                        override val sampleRate = configure.audio.sampleRate
                        override val channels = configure.audio.channels
                    })
            } else {
                null
            }

            init {
                videoDecoder.start()
                audioDecoder?.start()
                observer.onStart(ReceiverAdapterWrapper { close() })
            }

            override fun sink(kind: Int, flags: Int, timestamp: Long, buf: ByteArray): Boolean {
                try {
                    if (isReleased) {
                        return false
                    }

                    when (kind) {
                        StreamKind.VIDEO -> {
                            if (videoDecoder.isRunning) {
                                videoDecoder.sink(buf, flags, timestamp)
                            }
                        }

                        StreamKind.AUDIO -> {
                            if (audioDecoder != null && audioDecoder.isRunning) {
                                audioDecoder.sink(buf, flags, timestamp)
                            }
                        }
                    }

                    observer.sink(buf, kind)
                    return true
                } catch (e: Exception) {
                    Log.e(
                        "com.github.mycrl.hylarana",
                        "Hylarana ReceiverAdapter sink exception",
                        e
                    )

                    return false
                }
            }

            override fun online() {
                observer.onLine()
            }

            override fun close() {
                try {
                    if (!isReleased) {
                        isReleased = true
                        adapter?.release()
                        audioDecoder?.release()
                        videoDecoder.release()
                        observer.released()
                    }
                } catch (e: Exception) {
                    Log.e(
                        "com.github.mycrl.hylarana",
                        "Hylarana ReceiverAdapter close exception",
                        e
                    )
                }
            }
        })
    }
}

class HylaranaSender(
    private val sender: SenderAdapterWrapper,
    configure: HylaranaAdapterConfigure,
    record: AudioRecord?,
) {
    private val videoEncoder: Video.VideoEncoder =
        Video.VideoEncoder(configure.video, object : ByteArraySinker() {
            override fun sink(info: StreamBufferInfo, buf: ByteArray) {
                sender.send(info, buf)
            }
        })

    private val audioEncoder: Audio.AudioEncoder =
        Audio.AudioEncoder(record, configure.audio, object : ByteArraySinker() {
            override fun sink(info: StreamBufferInfo, buf: ByteArray) {
                sender.send(info, buf)
            }
        })

    init {
        videoEncoder.start()
        audioEncoder.start()
    }

    /**
     * Get whether the sender uses multicast transmission
     */
    fun getMulticast() : Boolean {
        return sender.getMulticast()
    }

    /**
     * Set whether the sender uses multicast transmission
     */
    fun setMulticast(isMulticast: Boolean) {
        sender.setMulticast(isMulticast)
    }

    /**
     * Get the surface inside the sender, you need to render the texture to this surface to pass the
     * screen to other receivers.
     */
    fun getSurface(): Surface? {
        return videoEncoder.getSurface()
    }

    /**
     * Push a single frame of data into the video encoder, note that the frame type needs to be the
     * same as the encoder configuration and you need to be aware of the input frame rate.
     */
    fun pushVideoFrame(frame: ByteArray) {
        videoEncoder.sink(frame)
    }

    fun pushAudioChunk(chunk: ByteArray) {
        audioEncoder.sink(chunk)
    }

    /**
     * Close and release this sender.
     */
    fun release() {
        audioEncoder.release()
        videoEncoder.release()
        sender.release()
    }
}
