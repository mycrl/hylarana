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

enum class TransportStrategyType(val type: String) {
    DIRECT("Direct"),
    RELAY("Relay"),
    MULTICAST("Multicast"),
}

enum class VideoFormat(val flag: Int) {
    BGRA(0),
    RGBA(1),
    NV12(2),
    I420(3),
}

data class StreamBufferInfo(/* StreamType */ val type: Int) {
    var flags: Int = 0
    var timestamp: Long = 0
}

/**
 * transport strategy
 */
@Serializable
data class TransportStrategy(
    /**
     * TransportStrategyType
     */
    val ty: String,
    /**
     * socket address
     */
    var address: String
)

@Serializable
data class TransportOptions(
    val strategy: TransportStrategy,
    /**
     * see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
     */
    val mtu: Int
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
    val id: String,
    val transport: TransportOptions,
    val video: MediaVideoStreamDescription?,
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
    fun send(info: StreamBufferInfo, bytes: ByteArray): Boolean {
        return sendHandle(info, bytes)
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
        var ptr = createTransportSender(Json.encodeToString(options))
        if (ptr == 0L) {
            throw Exception("failed to create transport sender")
        }

        val id = getTransportSenderId(ptr)
        return HylaranaSenderAdapter(
            id,
            { info, bytes ->
                if (ptr != 0L) sendStreamBufferToTransportSender(ptr, info, bytes) else false
            },
            {
                if (ptr != 0L) {
                    val copyPtr = ptr
                    ptr = 0L

                    releaseTransportSender(copyPtr)
                }
            },
        )
    }

    fun createReceiver(
        id: String, options: TransportOptions, observer: HylaranaReceiverAdapterObserver
    ): HylaranaReceiverAdapter {
        var ptr = createTransportReceiver(id, Json.encodeToString(options), observer)
        if (ptr == 0L) {
            throw Exception("failed to create transport receiver")
        }

        return HylaranaReceiverAdapter {
            if (ptr != 0L) {
                val copyPtr = ptr
                ptr = 0L

                releaseTransportReceiver(copyPtr)
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
        bytes: ByteArray,
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