package com.example.hylarana.app

import android.content.Context
import io.github.serpro69.kfaker.Faker
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File

/**
 * The settings information storage class used by Webview.
 *
 * ```
 * val storage = SettingsStorage(context)
 *
 * val settings = storage.value
 * storage.value = settings
 *
 * val settingsRaw = storage.raw
 * storage.raw = settingsRaw
 * ```
 */
class Settings(private val context: Context) {
    private val file = File(context.filesDir, "settings.json")
    private var model: Model

    var value: Model
        get() = model
        set(value) {
            model = value
            raw = Json.encodeToString(value)
        }

    /**
     * Get the original configuration item text.
     *
     * ```
     * val storage = SettingsStorage(context)
     *
     * val settings = storage.value
     * storage.value = settings
     *
     * val settingsRaw = storage.raw
     * storage.raw = settingsRaw
     * ```
     */
    var raw: String
        get() = file.readText()
        set(value) {
            file.writeText(value)
        }

    /**
     * Checks if the configuration file exists, and if it does not, creates a configuration file
     * containing the default configuration items.
     */
    init {
        if (!file.exists()) {
            val settings = Json.decodeFromString<Model>(
                context.resources.openRawResource(R.raw.settings).bufferedReader()
                    .use { it.readText() })

            settings.system.name = Faker().name.name()
            settings.video.width = context.resources.displayMetrics.widthPixels
            settings.video.height = context.resources.displayMetrics.heightPixels
            settings.video.frameRate = 30
            value = settings
        }

        model = Json.decodeFromString(raw)
    }

    @Serializable
    data class Model(
        var network: Network,
        var system: System,
        var codec: Codec,
        var video: Video,
        var audio: Audio
    )

    @Serializable
    data class System(
        var name: String,
        var language: String,
        /**
         * Direct3D11: Backend implemented using D3D11, which is supported on an older device and
         * platform and has better performance performance and memory footprint, but only on windows.
         *
         * WebGPU: Cross-platform graphics backends implemented using WebGPUs are supported on a
         * number of common platforms or devices.
         */
        var backend: String
    )

    @Serializable
    data class Network(
        /**
         * Bound NIC interfaces, 0.0.0.0 means all NICs are bound.
         */
        @SerialName("interface")
        var inter: String,
        /**
         * The IP address used for multicast, the default is 239.0.0.1.
         */
        var multicast: String,
        /**
         * The address of the forwarding server, such as 192.168.1.100:8080.
         */
        var server: String?,
        var port: Int,
        /**
         * In computer networking, the maximum transmission unit (MTU) is the size of the largest
         * protocol data unit (PDU) that can be communicated in a single network layer transaction.
         */
        var mtu: Int
    )

    @Serializable
    data class Codec(
        /**
         * Video encoder, X264 is a software encoder with the best compatibility.
         */
        var encoder: String,
        /**
         * Video decoder, H264 is a software decoder with the best compatibility.
         */
        var decoder: String
    )

    @Serializable
    data class Video(
        /**
         * The width and height of the video on the sender side.
         */
        var width: Int,
        var height: Int,
        /**
         * The refresh rate of the video is usually 24 / 30 / 60.
         */
        @SerialName("frame_rate")
        var frameRate: Int,
        /**
         * The bit rate of the video stream, in bit/s.
         */
        @SerialName("bit_rate")
        var bitRate: Int,
        /**
         * It is recommended that the key frame interval be consistent with the video frame rate,
         * which helps reduce the size of the video stream.
         */
        @SerialName("key_frame_interval")
        var keyFrameInterval: Int
    )

    @Serializable
    data class Audio(
        /**
         * The audio sampling rate is recommended to be 48Khz.
         */
        @SerialName("sample_rate")
        var sampleRate: Int,
        /**
         * The bit rate of the audio stream, in bit/s.
         */
        @SerialName("bit_rate")
        var bitRate: Int
    )
}