package com.github.mycrl.hylarana

import android.R.attr.mimeType
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.os.Build
import android.os.Process
import android.util.Log
import android.view.Surface
import java.nio.ByteBuffer
import java.util.Locale


abstract class ByteArraySinker {
    abstract fun sink(info: StreamBufferInfo, buf: ByteArray)
}

class Video {
    class VideoEncoder(
        configure: VideoEncoderConfigure,
        private val sinker: ByteArraySinker
    ) {
        private var isRunning: Boolean = false

        private val codec: MediaCodec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
        private val bufferInfo = MediaCodec.BufferInfo()
        private var surface: Surface? = null
        private var worker: Thread

        init {
            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, configure.width, configure.height)
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR)
            format.setInteger(MediaFormat.KEY_PROFILE, MediaCodecInfo.CodecProfileLevel.AVCProfileBaseline)
            format.setFloat(MediaFormat.KEY_MAX_FPS_TO_ENCODER, configure.frameRate.toFloat())
            format.setInteger(MediaFormat.KEY_LATENCY, configure.frameRate / 10)
            format.setInteger(MediaFormat.KEY_OPERATING_RATE, configure.frameRate)
            format.setInteger(MediaFormat.KEY_CAPTURE_RATE, configure.frameRate)
            format.setInteger(MediaFormat.KEY_FRAME_RATE, configure.frameRate)
            format.setInteger(MediaFormat.KEY_COLOR_FORMAT, configure.format)
            format.setInteger(MediaFormat.KEY_COLOR_RANGE, MediaFormat.COLOR_RANGE_LIMITED)
            format.setInteger(MediaFormat.KEY_BIT_RATE, configure.bitRate)
            format.setFloat(MediaFormat.KEY_I_FRAME_INTERVAL, 0.4F)
            format.setInteger(MediaFormat.KEY_MAX_B_FRAMES, 0)
            format.setInteger(
                MediaFormat.KEY_LEVEL, if (configure.width <= 1280 && configure.height <= 720) {
                    MediaCodecInfo.CodecProfileLevel.AVCLevel31
                } else if (configure.width <= 2048 && configure.height <= 1024) {
                    MediaCodecInfo.CodecProfileLevel.AVCLevel4
                } else {
                    MediaCodecInfo.CodecProfileLevel.AVCLevel5
                }
            )

            if (codec.name.indexOf(".rk.") >= 0) {
                format.setInteger(MediaFormat.KEY_COMPLEXITY, 0)
                format.setInteger(MediaFormat.KEY_PRIORITY, 0)
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                format.setInteger(MediaFormat.KEY_ALLOW_FRAME_DROP, 1)
            }

            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            surface =
                if (configure.format == MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface) {
                    codec.createInputSurface()
                } else {
                    null
                }

            worker = Thread {
                Process.setThreadPriority(Process.THREAD_PRIORITY_VIDEO)

                val buffer = ByteArray(2 * 1024 * 1024)
                val streamBufferInfo = StreamBufferInfo(StreamType.VIDEO)

                while (isRunning) {
                    try {
                        val index = codec.dequeueOutputBuffer(bufferInfo, -1)
                        if (index >= 0) {
                            val outputBuffer = codec.getOutputBuffer(index)
                            if (outputBuffer != null && bufferInfo.size > 0) {
                                streamBufferInfo.flags = bufferInfo.flags
                                streamBufferInfo.timestamp = bufferInfo.presentationTimeUs
                                outputBuffer.get(buffer, 0, bufferInfo.size)

                                sinker.sink(
                                    streamBufferInfo,
                                    buffer.sliceArray(bufferInfo.offset until bufferInfo.size),
                                )
                            }

                            codec.releaseOutputBuffer(index, false)
                        }
                    } catch (e: Exception) {
                        Log.w("com.github.mycrl.hylarana", "VideoEncoder worker exception", e)

                        release()
                    }
                }
            }
        }

        fun sink(buf: ByteArray) {
            val index = codec.dequeueInputBuffer(-1)
            if (index >= 0) {
                codec.getInputBuffer(index)?.clear()
                codec.getInputBuffer(index)?.put(buf)
                codec.queueInputBuffer(index, 0, buf.size, 0, 0)
            }
        }

        fun getSurface(): Surface? {
            return surface
        }

        fun start() {
            if (!isRunning) {
                isRunning = true

                codec.start()
                worker.start()
            }
        }

        fun release() {
            if (isRunning) {
                isRunning = false

                codec.stop()
                codec.release()
            }
        }

        interface VideoEncoderConfigure {

            /**
             * [MediaCodecInfo.CodecCapabilities](https://developer.android.com/reference/android/media/MediaCodecInfo.CodecCapabilities)
             */
            val format: Int
            var width: Int
            var height: Int

            /**
             * [MediaFormat#KEY_BIT_RATE](https://developer.android.com/reference/android/media/MediaFormat#KEY_BIT_RATE)
             */
            val bitRate: Int

            /**
             * [MediaFormat#KEY_FRAME_RATE](https://developer.android.com/reference/android/media/MediaFormat#KEY_FRAME_RATE)
             */
            val frameRate: Int
        }
    }

    class VideoDecoder(surface: Surface) {
        var isRunning: Boolean = false

        private lateinit var codec: MediaCodec
        private val bufferInfo = MediaCodec.BufferInfo()
        private var worker: Thread

        init {

            var codecName: String? = null
            run {
                val codecList = MediaCodecList(MediaCodecList.REGULAR_CODECS)
                val codecInfos = codecList.codecInfos

                for (codecInfo in codecInfos) {
                    if (!codecInfo.isEncoder && codecInfo.isHardwareAccelerated) {
                        for (type in codecInfo.supportedTypes) {
                            if (type == "video/avc" && codecInfo.name.indexOf("low_latency") > 0) {
                                codecName = codecInfo.name
                            }
                        }
                    }
                }
            }

            codec = if (codecName != null) {
                MediaCodec.createByCodecName(codecName!!)
            } else {
                MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
            }

            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, 2560, 1660)
            format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                if (codec.name.indexOf(".rk.") < 0 && codec.name.indexOf(".hisi.") < 0) {
                    format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
                }
            }

            codec.configure(format, surface, null, 0)
            worker = Thread {
                Process.setThreadPriority(Process.THREAD_PRIORITY_VIDEO)

                while (isRunning) {
                    try {
                        val index = codec.dequeueOutputBuffer(bufferInfo, -1)
                        if (index >= 0) {
                            codec.releaseOutputBuffer(index, true)
                        }
                    } catch (e: Exception) {
                        Log.w("com.github.mycrl.hylarana", "VideoDecoder worker exception", e)

                        release()
                    }
                }
            }
        }

        fun sink(buf: ByteArray, flags: Int, timestamp: Long) {
            try {
                val index = codec.dequeueInputBuffer(-1)
                if (index >= 0) {
                    codec.getInputBuffer(index)?.clear()
                    codec.getInputBuffer(index)?.put(buf)
                    codec.queueInputBuffer(index, 0, buf.size, timestamp, flags)
                }
            } catch (e: Exception) {
                Log.w("com.github.mycrl.hylarana", "VideoDecoder sink exception", e)

                release()
            }
        }

        fun start() {
            if (!isRunning) {
                isRunning = true

                codec.start()
                worker.start()
            }
        }

        fun release() {
            if (isRunning) {
                isRunning = false

                codec.stop()
                codec.release()
            }
        }
    }
}

class Audio {
    interface AudioCodecConfigure {

        /**
         * [AudioFormat#ENCODING_PCM_16BIT](https://developer.android.com/reference/android/media/AudioFormat#ENCODING_PCM_16BIT)
         */
        val sampleBits: Int

        /**
         * [AudioFormat#SAMPLE_RATE_UNSPECIFIED](https://developer.android.com/reference/android/media/AudioFormat#SAMPLE_RATE_UNSPECIFIED)
         */
        val sampleRate: Int

        /**
         * Number of audio channels, such as mono or stereo (dual channel)
         */
        val channels: Int

        /**
         * [MediaFormat#KEY_BIT_RATE](https://developer.android.com/reference/android/media/MediaFormat#KEY_BIT_RATE)
         */
        val bitRate: Int
    }

    companion object {
        fun getAudioCodecConfigure(): AudioCodecConfigure {
            return object: AudioCodecConfigure {
                override val sampleBits = AudioFormat.ENCODING_PCM_16BIT
                override val sampleRate = 48000
                override val bitRate = 64000
                override val channels = 1
            }
        }
    }

    class AudioDecoder(private val track: AudioTrack) {
        var isRunning: Boolean = false

        private val bufferInfo = MediaCodec.BufferInfo()
        private var codec: MediaCodec
        private var worker: Thread

        init {
            val format = MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_OPUS, 48000, 1)
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR)
            format.setInteger(MediaFormat.KEY_PCM_ENCODING, AudioFormat.ENCODING_PCM_16BIT)

            codec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_AUDIO_OPUS)
            codec.configure(format, null, null, 0)

            worker = Thread {
                Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)

                val buf = ByteArray(1024 * 1024)

                while (isRunning) {
                    try {
                        val index = codec.dequeueOutputBuffer(bufferInfo, -1)
                        if (index >= 0) {
                            val outputBuffer = codec.getOutputBuffer(index)
                            if (outputBuffer != null && bufferInfo.size > 0) {
                                outputBuffer.get(buf, 0, bufferInfo.size)
                                track.write(buf, 0, bufferInfo.size)
                            }

                            codec.releaseOutputBuffer(index, false)
                        }
                    } catch (e: Exception) {
                        Log.w("com.github.mycrl.hylarana", "AudioDecoder worker exception", e)

                        release()
                    }
                }
            }
        }

        fun sink(buf: ByteArray, flags: Int, timestamp: Long) {
            val index = codec.dequeueInputBuffer(1000)
            if (index >= 0) {
                codec.getInputBuffer(index)?.clear()
                codec.getInputBuffer(index)?.put(buf)
                codec.queueInputBuffer(index, 0, buf.size, timestamp, flags)
            }
        }

        fun start() {
            if (!isRunning) {
                isRunning = true

                codec.start()
                worker.start()
                track.play()
            }
        }

        fun release() {
            if (isRunning) {
                isRunning = false

                track.stop()
                track.release()
                codec.stop()
                codec.release()
            }
        }
    }

    class AudioEncoder(
        private val record: AudioRecord?,
        private val sinker: ByteArraySinker
    ) {
        private var isRunning: Boolean = false

        private val bufferInfo = MediaCodec.BufferInfo()
        private var codec: MediaCodec
        private var worker: Thread
        private var recorder: Thread? = null

        private val minBufferSize = AudioRecord.getMinBufferSize(
            48000,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )

        init {
            val format = MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_OPUS, 48000, 1)
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR)
            format.setInteger(MediaFormat.KEY_PCM_ENCODING, AudioFormat.ENCODING_PCM_16BIT)
            format.setInteger(MediaFormat.KEY_CHANNEL_COUNT, 1)
            format.setInteger(MediaFormat.KEY_BIT_RATE, 64000)
            format.setInteger(MediaFormat.KEY_DURATION, 100000)
            format.setInteger(MediaFormat.KEY_COMPLEXITY, 0)

            codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_AUDIO_OPUS)
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)

            worker = Thread {
                Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)

                val buffer = ByteArray(1024 * 1024)
                val streamBufferInfo = StreamBufferInfo(StreamType.AUDIO)

                while (isRunning) {
                    try {
                        val index = codec.dequeueOutputBuffer(bufferInfo, -1)
                        if (index >= 0) {
                            val outputBuffer = codec.getOutputBuffer(index)
                            if (outputBuffer != null && bufferInfo.size > 0) {
                                streamBufferInfo.flags = bufferInfo.flags
                                streamBufferInfo.timestamp = bufferInfo.presentationTimeUs
                                outputBuffer.get(buffer, 0, bufferInfo.size)

                                sinker.sink(
                                    streamBufferInfo,
                                    buffer.sliceArray(bufferInfo.offset until bufferInfo.size),
                                )
                            }

                            codec.releaseOutputBuffer(index, false)
                        }
                    } catch (e: Exception) {
                        Log.w("com.github.mycrl.hylarana", "AudioEncoder worker exception", e)

                        release()
                    }
                }
            }

            if (record != null) {
                recorder = Thread {
                    Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)

                    while (isRunning) {
                        try {
                            val buf = ByteBuffer.allocateDirect(minBufferSize)
                            val size = record.read(buf, buf.capacity(), AudioRecord.READ_BLOCKING)
                            if (size > 0) {
                                val index = codec.dequeueInputBuffer(-1)
                                if (index >= 0) {
                                    codec.getInputBuffer(index)?.put(buf)
                                    codec.queueInputBuffer(index, 0, size, 0, 0)
                                }
                            }
                        } catch (e: Exception) {
                            Log.w("com.github.mycrl.hylarana", "AudioDecoder record exception", e)

                            release()
                        }
                    }
                }
            }
        }

        fun sink(buf: ByteArray) {
            val index = codec.dequeueInputBuffer(-1)
            if (index >= 0) {
                codec.getInputBuffer(index)?.clear()
                codec.getInputBuffer(index)?.put(buf)
                codec.queueInputBuffer(index, 0, buf.size, 0, 0)
            }
        }

        fun start() {
            if (!isRunning) {
                isRunning = true

                codec.start()
                worker.start()
                recorder?.start()
                record?.startRecording()
            }
        }

        fun release() {
            if (isRunning) {
                isRunning = false

                record?.stop()
                codec.stop()
                codec.release()
            }
        }
    }
}
