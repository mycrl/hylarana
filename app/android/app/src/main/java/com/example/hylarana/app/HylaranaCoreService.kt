package com.example.hylarana.app

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.hardware.display.DisplayManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioPlaybackCaptureConfiguration
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.view.Surface
import androidx.annotation.RequiresPermission
import androidx.core.app.NotificationCompat
import com.github.mycrl.hylarana.Audio
import com.github.mycrl.hylarana.HylaranaReceiver
import com.github.mycrl.hylarana.HylaranaReceiverObserver
import com.github.mycrl.hylarana.HylaranaSender
import com.github.mycrl.hylarana.HylaranaSenderConfigure
import com.github.mycrl.hylarana.HylaranaSenderObserver
import com.github.mycrl.hylarana.HylaranaService
import com.github.mycrl.hylarana.MediaStreamDescription

class HylaranaCoreService : Service() {
    private var sender: HylaranaSender? = null
    private var receiver: HylaranaReceiver? = null

    override fun onBind(intent: Intent): IBinder {
        return ServiceBinder(this)
    }

    override fun onDestroy() {
        super.onDestroy()

        sender?.release()
        receiver?.release()
    }

    @RequiresPermission(Manifest.permission.RECORD_AUDIO)
    fun createSender(
        intent: Intent,
        configure: HylaranaSenderConfigure,
        observer: Observer
    ): MediaStreamDescription? {
        val notification = Notification(this)

        val mediaProjection =
            (getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager)
                .getMediaProjection(Activity.RESULT_OK, intent)

        mediaProjection?.registerCallback(object : MediaProjection.Callback() {}, null)

        val virtualDisplay = mediaProjection?.createVirtualDisplay(
            "HylaranaVirtualDisplayService",
            configure.video.width,
            configure.video.height,
            1,
            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
            null,
            null,
            null
        )

        val audioConfig = Audio.getAudioCodecConfigure()
        sender = HylaranaService.createSender(configure, object : HylaranaSenderObserver() {
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
                sender = null

                notification.close()
                mediaProjection.stop()
                virtualDisplay?.release()

                observer.onClosed()
            }
        })

        virtualDisplay?.surface = sender?.surface

        return sender?.getDescription()
    }

    fun createReceiver(surface: Surface, description: MediaStreamDescription, observer: Observer) {
        val audioConfig = Audio.getAudioCodecConfigure()
        receiver = HylaranaService.createReceiver(description, object : HylaranaReceiverObserver() {
            override val surface = surface
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
                receiver?.release()
                receiver = null

                observer.onClosed()
            }
        })
    }

    class ServiceBinder(private val service: HylaranaCoreService) : Binder() {
        @RequiresPermission(Manifest.permission.RECORD_AUDIO)
        fun createSender(
            intent: Intent,
            configure: HylaranaSenderConfigure,
            observer: Observer
        ): MediaStreamDescription? {
            return service.createSender(intent, configure, observer)
        }

        fun createReceiver(
            surface: Surface,
            description: MediaStreamDescription,
            observer: Observer
        ) {
            service.createReceiver(surface, description, observer)
        }

        fun stopSender() {
            service.sender?.release()
            service.sender = null
        }

        fun stopReceiver() {
            service.receiver?.release()
            service.receiver = null
        }
    }

    abstract class Observer() {
        abstract fun onClosed()
    }

    @SuppressLint("ForegroundServiceType")
    class Notification(private val service: HylaranaCoreService) {
        init {
            service.getSystemService(NotificationManager::class.java).createNotificationChannel(
                NotificationChannel(
                    "HylaranaCoreService",
                    "HylaranaCoreService",
                    NotificationManager.IMPORTANCE_LOW
                )
            )

            service.startForeground(
                1, NotificationCompat.Builder(service, "HylaranaCoreService")
                    .setSmallIcon(R.drawable.ic_launcher_foreground)
                    .setContentTitle("HylaranaCoreService")
                    .setContentText("HylaranaCoreService is casting...")
                    .setPriority(NotificationCompat.PRIORITY_DEFAULT).build()
            )
        }

        fun close() {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                service.stopForeground(Service.STOP_FOREGROUND_REMOVE)
            } else {
                service.stopForeground(true)
            }
        }
    }
}