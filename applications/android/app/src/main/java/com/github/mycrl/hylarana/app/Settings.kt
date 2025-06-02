package com.github.mycrl.hylarana.app

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
            value = Model.default(context)
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
    ) {
        companion object {
            fun default(context: Context): Model {
                return Model(
                    network = Network.default(),
                    system = System.default(),
                    codec = Codec.default(),
                    audio = Audio.default(),
                    video = Video.default(context),
                );
            }
        }
    }

    @Serializable
    data class System(
        var name: String,
        var language: String,
    ) {
        companion object {
            fun default(): System {
                return System(
                    name = Faker().name.name(),
                    language = "english",
                );
            }
        }
    }

    @Serializable
    data class Network(
        /**
         * Bound NIC interfaces, 0.0.0.0 means all NICs are bound.
         */
        var bind: String,
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
    ) {
        companion object {
            fun default(): Network {
                return Network(
                    bind = "0.0.0.0:43165",
                    mtu = 1500,
                    maxBandwidth = -1,
                    latency = 20,
                    fc = 32,
                    timeout = 2000,
                    fec = "fec,layout:staircase,rows:2,cols:10,arq:onreq",
                );
            }
        }
    }

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
    ) {
        companion object {
            fun default(): Codec {
                return Codec(
                    encoder = "",
                    decoder = "",
                );
            }
        }
    }

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
    ) {
        companion object {
            fun default(context: Context): Video {
                return Video(
                    width = context.resources.displayMetrics.widthPixels,
                    height = context.resources.displayMetrics.heightPixels,
                    frameRate = 30,
                    bitRate = 5000000,
                    keyFrameInterval = 30,
                );
            }
        }
    }

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
    ) {
        companion object {
            fun default(): Audio {
                return Audio(sampleRate = 48000, bitRate = 64000);
            }
        }
    }
}
