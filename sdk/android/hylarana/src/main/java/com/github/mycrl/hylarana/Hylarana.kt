package com.github.mycrl.hylarana

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Data Stream Receiver Adapter
 *
 * Used to receive data streams from the network.
 */
internal abstract class HylaranaReceiverAdapterObserver {
    /**
     * Triggered when data arrives in the network.
     *
     * Note: If the buffer is empty, the current network connection has been closed or suddenly interrupted.
     */
    abstract fun sink(kind: Int, flags: Int, timestamp: Long, buf: ByteArray): Boolean

    /**
     * stream is closed.
     */
    abstract fun close()
}

/**
 * STREAM_TYPE_VIDEO | STREAM_TYPE_AUDIO
 */
data class StreamBufferInfo(val type: Int) {
    var flags: Int = 0
    var timestamp: Long = 0
}

/**
 * transport strategy
 */
@Serializable
data class TransportStrategy(
    /**
     * STRATEGY_DIRECT | STRATEGY_RELAY | STRATEGY_MULTICAST
     */
    @SerialName("t")
    val type: String,
    /**
     * socket address
     */
    @SerialName("v")
    var addr: String
)

class VideoFormat {
    companion object {
        const val BGRA: Int = 0
        const val RGBA: Int = 1
        const val NV12: Int = 2
        const val I420: Int = 3
    }
}

@Serializable
data class TransportOptions(
    @SerialName("s")
    val strategy: TransportStrategy,
    /**
     * see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
     */
    @SerialName("m")
    val mtu: Int
)

@Serializable
data class Size(
    @SerialName("w")
    val width: Int,
    @SerialName("h")
    val height: Int,
)

@Serializable
data class MediaVideoStreamDescription(
    @SerialName("f")
    val format: Int,
    @SerialName("s")
    val size: Size,
    val fps: Int,
    @SerialName("br")
    val bitRate: Int,
)

@Serializable
data class MediaAudioStreamDescription(
    @SerialName("sr")
    val sampleRate: Int,
    @SerialName("cs")
    val channels: Int,
    @SerialName("br")
    val bitRate: Int,
)

@Serializable
data class MediaStreamDescription(
    @SerialName("i")
    val id: String,
    @SerialName("t")
    val transport: TransportOptions,
    @SerialName("v")
    val video: MediaVideoStreamDescription?,
    @SerialName("a")
    val audio: MediaAudioStreamDescription?,
)

class HylaranaSenderAdapter(
    private val id: String,
    private val sendHandle: (StreamBufferInfo, ByteArray) -> Boolean,
    private val releaseHandle: () -> Unit,
) {
    /**
     * get sender stream id.
     */
    fun getId(): String {
        return id
    }

    /**
     * send stream buffer to sender.
     */
    fun send(info: StreamBufferInfo, buf: ByteArray): Boolean {
        return sendHandle(info, buf)
    }

    /**
     * Close and release this sender.
     */
    fun release() {
        releaseHandle()
    }
}

class HylaranaReceiverAdapter(private val releaseHandle: () -> Unit) {
    /**
     * Close and release this receiver.
     */
    fun release() {
        releaseHandle()
    }
}

internal class Hylarana {
    companion object {
        init {
            System.loadLibrary("hylarana")
        }
    }

    fun createSender(
        options: TransportOptions
    ): HylaranaSenderAdapter {
        var sender = createTransportSender(Json.encodeToString(options))
        if (sender == 0L) {
            throw Exception("failed to create transport sender")
        }

        val id = getTransportSenderId(sender)
        return HylaranaSenderAdapter(
            id,
            { info, buf ->
                if (sender != 0L) {
                    if (!sendStreamBufferToTransportSender(sender, info, buf)) {
                        sender = 0L

                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            },
            {
                run {
                    if (sender != 0L) {
                        val ptr = sender
                        sender = 0L

                        releaseTransportSender(ptr)
                    }
                }
            },
        )
    }

    fun createReceiver(
        id: String, options: TransportOptions, observer: HylaranaReceiverAdapterObserver
    ): HylaranaReceiverAdapter {
        var receiver = createTransportReceiver(id, Json.encodeToString(options), observer)
        if (receiver == 0L) {
            throw Exception("failed to create transport receiver")
        }

        return HylaranaReceiverAdapter {
            run {
                if (receiver != 0L) {
                    val ptr = receiver
                    receiver = 0L

                    releaseTransportReceiver(ptr)
                }
            }
        }
    }

    /**
     * Creates the sender, the return value indicates whether the creation
     * was successful or not.
     */
    private external fun createTransportSender(
        options: String,
    ): Long

    /**
     * get transport sender id.
     */
    private external fun getTransportSenderId(
        sender: Long
    ): String

    /**
     * Sends the packet to the sender instance.
     */
    private external fun sendStreamBufferToTransportSender(
        sender: Long,
        info: StreamBufferInfo,
        buf: ByteArray,
    ): Boolean

    /**
     * release transport sender.
     */
    private external fun releaseTransportSender(sender: Long)

    /**
     * Creates the receiver, the return value indicates whether the creation
     * was successful or not.
     */
    private external fun createTransportReceiver(
        id: String,
        options: String,
        observer: HylaranaReceiverAdapterObserver,
    ): Long

    /**
     * release transport receiver.
     */
    private external fun releaseTransportReceiver(sender: Long)
}