package com.example.hylarana.app

import android.util.Log
import com.github.mycrl.hylarana.MediaStreamDescription
import com.github.mycrl.hylarana.TransportOptions
import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.JsonElement as Value

/**
 * A call wrapper for communicating with the Webview, used to respond to requests from the Webview
 * and send events to the Webview.
 *
 * Example of usage:
 * ```
 * val bridge = Bridge()
 *
 * bridge.setHandler { message ->
 *     // Sends this message to the Webview.
 * }
 *
 * // The message sent out by the Webview is passed to the bridge.
 * bridge.sendMessage(message)
 *
 * bridge.on<Unit, List<Unit>>(Bridge.Methods.GET_DEVICES) { _ ->
 *     listOf()
 * }
 *
 * bridge.emit(Bridge.Events.READY)
 * ```
 */
class Bridge {
    var listeners: HashMap<String, (Value?) -> Value?> = HashMap();
    private var handle: ((String) -> Unit)? = null

    /**
     * Set up a callback whose message needs to be passed to the Webview.
     *
     * Example of usage:
     * ```
     * val bridge = Bridge()
     *
     * bridge.setHandler { message ->
     *     // Sends this message to the Webview.
     * }
     *
     * // The message sent out by the Webview is passed to the bridge.
     * bridge.sendMessage(message)
     *
     * bridge.on<Unit, List<Unit>>(Bridge.Methods.GET_DEVICES) { _ ->
     *     listOf()
     * }
     *
     * bridge.emit(Bridge.Events.READY)
     * ```
     */
    fun setHandler(handle: (String) -> Unit) {
        this.handle = handle
    }

    /**
     * Messages received from the Webview are passed to bridge.
     *
     * Example of usage:
     * ```
     * val bridge = Bridge()
     *
     * bridge.setHandler { message ->
     *     // Sends this message to the Webview.
     * }
     *
     * // The message sent out by the Webview is passed to the bridge.
     * bridge.sendMessage(message)
     *
     * bridge.on<Unit, List<Unit>>(Bridge.Methods.GET_DEVICES) { _ ->
     *     listOf()
     * }
     *
     * bridge.emit(Bridge.Events.READY)
     * ```
     */
    fun sendMessage(message: String) {
        Log.i("hylarana", "webview message transport on message: $message")

        when (val payload = Json.decodeFromString<Payload>(message)) {
            is Payload.IRequest -> {
                var res: Result? = null
                val (method, sequence, content) = payload.content

                listeners[method]?.let { it ->
                    try {
                        res = Result.Ok(it(content))
                    } catch (e: Error) {
                        Log.e("hylarana", "$e")

                        res = Result.Err(e.message)
                    }
                }

                res?.let {
                    handle?.let { handle ->
                        handle(
                            Json.encodeToString(
                                Payload.IResponse(
                                    content = Response(
                                        sequence,
                                        content = it
                                    )
                                )
                            )
                        )
                    }
                }
            }

            else -> {}
        }
    }

    /**
     * Listens for requests sent by the Webview and uses a handler to process the request and return.
     *
     * Example of usage:
     * ```
     * val bridge = Bridge()
     *
     * bridge.setHandler { message ->
     *     // Sends this message to the Webview.
     * }
     *
     * // The message sent out by the Webview is passed to the bridge.
     * bridge.sendMessage(message)
     *
     * bridge.on<Unit, List<Unit>>(Bridge.Methods.GET_DEVICES) { _ ->
     *     listOf()
     * }
     *
     * bridge.emit(Bridge.Events.READY)
     * ```
     */
    inline fun <reified Q, reified S> on(method: Method, crossinline handle: (Q) -> S) {
        listeners[method.type] = { req ->
            Json.encodeToJsonElement<S>(handle(if (req != null) Json.decodeFromJsonElement<Q>(req) else Unit as Q))
        }
    }

    /**
     * Sends an event to the Webview.
     *
     * Example of usage:
     * ```
     * val bridge = Bridge()
     *
     * bridge.setHandler { message ->
     *     // Sends this message to the Webview.
     * }
     *
     * // The message sent out by the Webview is passed to the bridge.
     * bridge.sendMessage(message)
     *
     * bridge.on<Unit, List<Unit>>(Bridge.Methods.GET_DEVICES) { _ ->
     *     listOf()
     * }
     *
     * bridge.emit(Bridge.Events.READY)
     * ```
     */
    fun emit(event: Event) {
        handle?.let { it(Json.encodeToString(Payload.IEvents(content = Events(method = event.type)))) }
    }

    fun release() {
        listeners.clear()
        handle = null
    }

    enum class Method(val type: String) {
        SET_NAME("SetName"),
        GET_DEVICES("GetDevices"),
        GET_CAPTURE_SOURCES("GetCaptureSources"),
        CREATE_SENDER("CreateSender"),
        CLOSE_SENDER("CloseSender"),
        CREATE_RECEIVER("CreateReceiver"),
        CLOSE_RECEIVER("CloseReceiver"),
        GET_STATUS("GetStatus"),
        GET_SETTINGS("GetSettings"),
        SET_SETTINGS("SetSettings"),
    }

    enum class Event(val type: String) {
        STATUS_CHANGE("StatusChangeNotify"),
        DEVICE_CHANGE("DevicesChangeNotify"),
        READY("ReadyNotify"),
    }

    /**
     * This is a response structure with two states, success and failure, and if it fails, an
     * internal error message.
     *
     * ```
     * try {
     *
     * } catch (e: Error) {
     *     Result.Err(e.message)
     * }
     * ```
     */
    @Serializable
    @JsonClassDiscriminator("ty")
    @OptIn(ExperimentalSerializationApi::class)
    sealed class Result {
        @Serializable
        @SerialName("Ok")
        data class Ok(val content: Value?) : Result()

        @Serializable
        @SerialName("Err")
        data class Err(val content: String?) : Result()
    }

    /**
     * There are loads of communication in Webview with events, requests and responses. Requests
     * and responses have sequence numbers, which are generated in the request and carry the same
     * sequence number in the response.
     *
     * ```
     * Events("ReadyNotify")
     * ```
     */
    @Serializable
    data class Events(val method: String)

    /**
     * There are loads of communication in Webview with events, requests and responses. Requests
     * and responses have sequence numbers, which are generated in the request and carry the same
     * sequence number in the response.
     *
     * ```
     * Response(0, Result.Ok(Json.encodeToJsonElement(Unit)))
     * ```
     */
    @Serializable
    data class Response(val sequence: Int, val content: Result)

    /**
     * There are loads of communication in Webview with events, requests and responses. Requests
     * and responses have sequence numbers, which are generated in the request and carry the same
     * sequence number in the response.
     *
     * ```
     * Request("GetDevices", 0, Json.encodeToJsonElement(Unit))
     * ```
     */
    @Serializable
    data class Request(val method: String, val sequence: Int, val content: Value?)

    /**
     * There are loads of communication in Webview with events, requests and responses. Requests
     * and responses have sequence numbers, which are generated in the request and carry the same
     * sequence number in the response.
     *
     * ```
     * Payload.IEvents(content = Events("ReadyNotify"))
     *
     * Payload.IResponse(content = Response(0, Result.Ok(Json.encodeToJsonElement(Unit))))
     *
     * Payload.IRequest(content = Request("GetDevices", 0, Json.encodeToJsonElement(Unit)))
     * ```
     */
    // @formatter:off
    // noinspection KotlinStyle
    @Serializable
    @JsonClassDiscriminator("ty")
    @OptIn(ExperimentalSerializationApi::class)
    sealed class Payload {
        @Serializable
        @SerialName("Events")
        data class IEvents(@EncodeDefault val ty: String = "Events", val content: Events) : Payload()

        @Serializable
        @SerialName("Response")
        data class IResponse(@EncodeDefault val ty: String = "Response", val content: Response) : Payload()

        @Serializable
        @SerialName("Request")
        data class IRequest(@EncodeDefault val ty: String = "Request", val content: Request) : Payload()
    }

    /**
     * Video source or Audio source.
     *
     * ```
     * Source(
     *     id = "default",
     *     index = 0,
     *     isDefault = true,
     *     kind = "Screen",
     *     name = "Main display"
     * )
     * ```
     */
    @Serializable
    data class Source(
        /**
         * Device ID, usually the symbolic link to the device or the address of the
         * device file handle.
         */
        val id: String,
        /**
         * Sequence number, which can normally be ignored, in most cases this field
         * has no real meaning and simply indicates the order in which the device
         * was acquired internally.
         */
        val index: Int,
        /**
         * Whether or not it is the default device, normally used to indicate
         * whether or not it is the master device.
         */
        @SerialName("is_default")
        val isDefault: Boolean,
        val kind: String,
        val name: String,
    )

    @Serializable
    data class SenderOptions(
        val transport: TransportOptions,
        val media: SenderMediaOptions
    )

    @Serializable
    data class SenderMediaStreamOptions<T>(
        val source: Source?,
        val options: T,
    )

    @Serializable
    data class SenderMediaOptions(
        val video: SenderMediaStreamOptions<SenderVideoMediaStreamOptions>?,
        val audio: SenderMediaStreamOptions<SenderAudioMediaStreamOptions>?,
    )

    @Serializable
    data class SenderVideoMediaStreamOptions(
        /**
         * Video encoder, X264 is a software encoder with the best compatibility.
         */
        val codec: String,
        /**
         * The refresh rate of the video is usually 24 / 30 / 60.
         */
        @SerialName("frame_rate")
        val frameRate: Int,
        /**
         * The width and height of the video on the sender side.
         */
        val width: Int,
        val height: Int,
        /**
         * The bit rate of the video stream, in bit/s.
         */
        @SerialName("bit_rate")
        val bitRate: Int,
        /**
         * It is recommended that the key frame interval be consistent with the video frame rate,
         * which helps reduce the size of the video stream.
         */
        @SerialName("key_frame_interval")
        val keyFrameInterval: Int,
    )

    @Serializable
    data class SenderAudioMediaStreamOptions(
        /**
         * The audio sampling rate is recommended to be 48Khz.
         */
        @SerialName("sample_rate")
        val sampleRate: Int,
        /**
         * The bit rate of the audio stream, in bit/s.
         */
        @SerialName("bit_rate")
        val bitRate: Int,
    )

    @Serializable
    enum class Status {
        Sending,
        Receiving,
        Idle,
    }

    @Serializable
    data class CreateSenderParams(
        val targets: List<String>,
        val options: SenderOptions,
    )

    @Serializable
    data class CreateReceiverParams(
        val codec: String,
        val backend: String,
        val description: MediaStreamDescription,
    )
}