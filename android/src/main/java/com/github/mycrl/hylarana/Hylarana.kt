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
    abstract fun sink(/*StreamType*/ kind: Int,
                      flags: Int,
                      timestamp: Long,
                      bytes: ByteArray
    ): Boolean

    /**
     * stream is closed.
     */
    abstract fun close()
}

enum class StreamType(val flag: Int) {
    VIDEO(0),
    AUDIO(1),
}

enum class VideoFormat(val flag: Int) {
    BGRA(0),
    RGBA(1),
    NV12(2),
    I420(3),
}

@Serializable
data class TransportOptions(
    /**
     * Maximum Transmission Unit size
     */
    val mtu: Int,
    /**
     * Maximum bandwidth in bytes per second
     */
    @SerialName("max_bandwidth")
    val maxBandwidth: Long,
    /**
     * Latency in milliseconds
     */
    val latency: Int,
    /**
     * Connection timeout in milliseconds
     */
    val timeout: Int,
    /**
     * Forward Error Correction configuration
     */
    val fec: String,
    /**
     * Flow control window size
     */
    val fc: Int,
)

@Serializable
data class Size(
    val width: Int,
    val height: Int,
)

@Serializable
data class MediaVideoStreamDescription(
    /**
     * VideoFormat
     */
    val format: Int,
    val size: Size,
    val fps: Int,
    @SerialName("bit_rate") val bitRate: Int,
)

@Serializable
data class MediaAudioStreamDescription(
    @SerialName("sample_rate") val sampleRate: Int,
    val channels: Int,
    @SerialName("bit_rate") val bitRate: Int,
)

@Serializable
data class MediaStreamDescription(
    val video: MediaVideoStreamDescription?,
    val audio: MediaAudioStreamDescription?,
)

class HylaranaSenderAdapter(
    private val getPortHandle: () -> Int,
    private val sendHandle: (Int, Int, Long, ByteArray) -> Boolean,
    private val releaseHandle: () -> Unit,
) {
    fun getPort(): Int {
        return getPortHandle()
    }

    /**
     * send stream buffer to sender.
     */
    fun send(
        kind: Int,
        flags: Int,
        timestamp: Long, bytes: ByteArray
    ): Boolean {
        return sendHandle(kind, flags, timestamp, bytes)
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
        bind: String,
        options: TransportOptions
    ): HylaranaSenderAdapter {
        var sender = senderCreate(bind, Json.encodeToString(options))
        if (sender == 0L) {
            throw Exception("failed to create transport sender")
        }

        return HylaranaSenderAdapter(
            {
                if (sender != 0L) senderGetPort(sender) else 0
            },
            { kind, flags, timestamp, bytes ->
                if (sender != 0L) senderWrite(
                    sender,
                    kind,
                    flags,
                    timestamp,
                    bytes
                ) else false
            },
            {
                if (sender != 0L) {
                    val ptr = sender
                    sender = 0L

                    senderRelease(ptr)
                }
            },
        )
    }

    fun createReceiver(
        addr: String,
        options: TransportOptions, observer: HylaranaReceiverAdapterObserver
    ): HylaranaReceiverAdapter {
        var receiver = receiverCreate(addr, Json.encodeToString(options), observer)
        if (receiver == 0L) {
            throw Exception("failed to create transport receiver")
        }

        return HylaranaReceiverAdapter {
            if (receiver != 0L) {
                val ptr = receiver
                receiver = 0L

                receiverRelease(ptr)
            }
        }
    }

    /**
     * Creates the sender, the return value indicates whether the creation
     * was successful or not.
     */
    private external fun senderCreate(
        bind: String,
        options: String,
    ): Long

    /**
     * get transport sender pkt lose reate.
     */
    private external fun senderGetPktLoseRate(
        sender: Long
    ): Long

    /**
     * get transport sender port.
     */
    private external fun senderGetPort(
        sender: Long
    ): Int

    /**
     * Sends the packet to the sender instance.
     */
    private external fun senderWrite(
        sender: Long,
        kind: Int,
        flags: Int,
        timestamp: Long,
        bytes: ByteArray,
    ): Boolean

    /**
     * release transport sender.
     */
    private external fun senderRelease(sender: Long)

    /**
     * Creates the receiver, the return value indicates whether the creation
     * was successful or not.
     */
    private external fun receiverCreate(
        addr: String,
        options: String,
        observer: HylaranaReceiverAdapterObserver,
    ): Long

    /**
     * release transport receiver.
     */
    private external fun receiverRelease(sender: Long)
}