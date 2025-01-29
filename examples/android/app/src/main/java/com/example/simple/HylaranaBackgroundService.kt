package com.example.simple

import android.annotation.SuppressLint
import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.graphics.BitmapFactory
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioPlaybackCaptureConfiguration
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaCodecInfo
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Binder
import android.os.IBinder
import android.util.DisplayMetrics
import android.util.Log
import android.view.Surface
import com.github.mycrl.hylarana.Audio
import com.github.mycrl.hylarana.Discovery
import com.github.mycrl.hylarana.DiscoveryService
import com.github.mycrl.hylarana.DiscoveryServiceQueryObserver
import com.github.mycrl.hylarana.HylaranaOptions
import com.github.mycrl.hylarana.HylaranaReceiver
import com.github.mycrl.hylarana.HylaranaReceiverObserver
import com.github.mycrl.hylarana.HylaranaSender
import com.github.mycrl.hylarana.HylaranaSenderConfigure
import com.github.mycrl.hylarana.HylaranaSenderObserver
import com.github.mycrl.hylarana.HylaranaService
import com.github.mycrl.hylarana.HylaranaStrategy
import com.github.mycrl.hylarana.HylaranaStrategyType
import com.github.mycrl.hylarana.MediaStreamDescription
import com.github.mycrl.hylarana.Video

class Notify(service: HylaranaBackgroundService) {
    companion object {
        private const val NotifyId = 100
        private const val NotifyChannelId = "HylaranaService"
        private const val NotifyChannelName = "HylaranaService"
    }

    init {
        val manager = service.getSystemService(Service.NOTIFICATION_SERVICE) as NotificationManager
        manager.createNotificationChannel(
            NotificationChannel(
                NotifyChannelId,
                NotifyChannelName,
                NotificationManager.IMPORTANCE_LOW
            )
        )

        val intent = Intent(service, MainActivity::class.java)
        val icon = BitmapFactory.decodeResource(service.resources, android.R.mipmap.sym_def_app_icon)
        val content =
            PendingIntent.getActivity(
                service,
                0,
                intent,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )

        val builder = Notification.Builder(service.applicationContext, NotifyChannelId)
        builder.setContentIntent(content)
        builder.setLargeIcon(icon)
        builder.setContentTitle("Screen recording")
        builder.setSmallIcon(android.R.mipmap.sym_def_app_icon)
        builder.setContentText("Recording screen......")
        builder.setWhen(System.currentTimeMillis())
        service.startForeground(NotifyId, builder.build())
    }
}

abstract class HylaranaBackgroundServiceObserver() {
    abstract fun onConnected()

    abstract fun onReceiverClosed()
}

class HylaranaBackgroundServiceBinder(private val service: HylaranaBackgroundService) : Binder() {
    fun createSender(intent: Intent, displayMetrics: DisplayMetrics) {
        service.createSender(intent, displayMetrics)
    }

    fun createReceiver() {
        service.createReceiver()
    }

    fun setRenderSurface(surface: Surface) {
        Log.i("simple", "set render surface to service.")

        service.setOutputSurface(surface)
    }

    fun connect(strategy: HylaranaStrategy) {
        service.connect(strategy)
    }

    fun stopSender() {
        Log.i("simple", "stop sender.")

        service.stopSender()
    }

    fun stopReceiver() {
        Log.i("simple", "stop receiver.")

        service.stopReceiver()
    }

    fun setObserver(observer: HylaranaBackgroundServiceObserver) {
        service.setObserver(observer)
    }
}

class HylaranaBackgroundService : Service() {
    private var observer: HylaranaBackgroundServiceObserver? = null
    private var mediaProjection: MediaProjection? = null
    private var virtualDisplay: VirtualDisplay? = null
    private var outputSurface: Surface? = null
    private var receiver: HylaranaReceiver? = null
    private var sender: HylaranaSender? = null
    private var discovery: DiscoveryService? = null
    private var strategy: HylaranaStrategy? = null

    override fun onBind(intent: Intent?): IBinder {
        return HylaranaBackgroundServiceBinder(this)
    }

    override fun onDestroy() {
        super.onDestroy()
        sender?.release()
        mediaProjection?.stop()
        virtualDisplay?.release()

        Log.w("simple", "service destroy.")
    }

    fun connect(strategy: HylaranaStrategy) {
        this.strategy = strategy

        try {
            observer?.onConnected()
        } catch (e: Exception) {
            Log.e("simple", "Hylarana connect exception", e)
        }
    }

    fun stopSender() {
        discovery?.release()
        discovery = null

        sender?.release()
        sender = null
    }

    fun stopReceiver() {
        discovery?.release()
        discovery = null

        receiver?.release()
        receiver = null
    }

    fun setObserver(observer: HylaranaBackgroundServiceObserver) {
        this.observer = observer
    }

    fun setOutputSurface(surface: Surface) {
        outputSurface = surface
    }

    fun createReceiver() {
        Log.i("simple", "create receiver.")

        discovery =
            Discovery()
                .query(
                    object : DiscoveryServiceQueryObserver() {
                        override fun resolve(addrs: Array<String>, description: MediaStreamDescription) {
                            if (receiver == null) {
                                if (description.transport.strategy.type == HylaranaStrategyType.DIRECT) {
                                    description.transport.strategy.addr =
                                        addrs[0] + ":" + description.transport.strategy.addr.split(":")[1]
                                }

                                val audioConfig = Audio.getAudioCodecConfigure()
                                receiver = HylaranaService.createReceiver(
                                    description,
                                    object : HylaranaReceiverObserver() {
                                        override val surface = outputSurface!!
                                        override val track =
                                            AudioTrack.Builder()
                                                .setAudioAttributes(
                                                    AudioAttributes.Builder()
                                                        .setUsage(AudioAttributes.USAGE_MEDIA)
                                                        .setContentType(
                                                            AudioAttributes.CONTENT_TYPE_MUSIC
                                                        )
                                                        .build()
                                                )
                                                .setAudioFormat(
                                                    AudioFormat.Builder()
                                                        .setEncoding(audioConfig.sampleBits)
                                                        .setSampleRate(audioConfig.sampleRate)
                                                        .setChannelMask(
                                                            AudioFormat.CHANNEL_OUT_STEREO
                                                        )
                                                        .build()
                                                )
                                                .setPerformanceMode(
                                                    AudioTrack.PERFORMANCE_MODE_LOW_LATENCY
                                                )
                                                .setTransferMode(AudioTrack.MODE_STREAM)
                                                .setBufferSizeInBytes(audioConfig.sampleRate / 10 * 2)
                                                .build()

                                        override fun close() {
                                            stopReceiver()
                                            observer?.onReceiverClosed()

                                            Log.w("simple", "receiver is released.")
                                        }
                                    }
                                )
                            }
                        }
                    }
                )
    }

    @SuppressLint("MissingPermission")
    fun createSender(intent: Intent, displayMetrics: DisplayMetrics) {
        Notify(this)

        Log.i("simple", "create sender.")

        mediaProjection =
            (getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager)
                .getMediaProjection(Activity.RESULT_OK, intent)

        mediaProjection?.registerCallback(object : MediaProjection.Callback() {}, null)

        val audioConfig = Audio.getAudioCodecConfigure()
        sender =
            strategy?.let {
                HylaranaService.createSender(
                    object : HylaranaSenderConfigure {
                        override val options = HylaranaOptions(strategy = it, mtu = 1500)

                        override val video =
                            object : Video.VideoEncoder.VideoEncoderConfigure {
                                override val format =
                                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface
                                override var height = displayMetrics.heightPixels
                                override var width = displayMetrics.widthPixels
                                override val bitRate = 500 * 1024 * 8
                                override val frameRate = 60
                            }
                    },
                    object : HylaranaSenderObserver() {
                        override val record =
                            AudioRecord.Builder()
                                .setAudioFormat(
                                    AudioFormat.Builder()
                                        .setSampleRate(audioConfig.sampleRate)
                                        .setChannelMask(AudioFormat.CHANNEL_IN_STEREO)
                                        .setEncoding(audioConfig.sampleBits)
                                        .build()
                                )
                                .setAudioPlaybackCaptureConfig(
                                    AudioPlaybackCaptureConfiguration.Builder(mediaProjection!!)
                                        .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                                        .addMatchingUsage(AudioAttributes.USAGE_GAME)
                                        .build()
                                )
                                .setBufferSizeInBytes(audioConfig.sampleRate / 10 * 2)
                                .build()

                        override fun close() {
                            super.close()

                            sender?.release()
                        }
                    }
                )
            }

        discovery =
            strategy?.let {
                Discovery()
                    .register(
                        3456,
                        sender!!.getDescription()
                    )
            }

        virtualDisplay =
            mediaProjection?.createVirtualDisplay(
                "HylaranaVirtualDisplayService",
                displayMetrics.widthPixels,
                displayMetrics.heightPixels,
                1,
                DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                null,
                null,
                null
            )

        virtualDisplay?.surface = sender?.getSurface()
    }
}
