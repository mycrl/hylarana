package com.example.hylarana.app

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentName
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.ActivityInfo
import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.media.MediaCodecInfo
import android.media.projection.MediaProjectionManager
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import android.view.Surface
import android.view.SurfaceView
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.zIndex
import androidx.core.content.ContextCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.github.mycrl.hylarana.HylaranaSenderConfigure
import com.github.mycrl.hylarana.Video
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private val scope = CoroutineScope(Dispatchers.IO)

    private var isSurfaceShow by mutableStateOf(false)

    private lateinit var settings: Settings
    private lateinit var service: Intent
    private lateinit var surface: Surface
    private lateinit var deviceManager: DeviceManager

    private val bridge = Bridge()
    private var status = Bridge.Status.Idle
    private val permissions = Permissions(this)

    private var serviceBinder: HylaranaCoreService.ServiceBinder? = null
    private val serviceConnection: ServiceConnection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
            serviceBinder = service as HylaranaCoreService.ServiceBinder
        }

        override fun onServiceDisconnected(name: ComponentName?) {

        }
    }

    override fun onDestroy() {
        super.onDestroy()

        stopService(service)
        deviceManager.release()
        bridge.release()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { Activity() }

        // Landscape only, with 180° flip support
        requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_USER_LANDSCAPE

        settings = Settings(this)
        deviceManager = DeviceManager(settings, scope)

        service = run {
            val intent = Intent(this, HylaranaCoreService::class.java)
            bindService(intent, serviceConnection, BIND_AUTO_CREATE)
            intent
        }

        bridge.on<Unit, Bridge.Status>(Bridge.Method.GET_STATUS) {
            status
        }

        bridge.on<String, Unit>(Bridge.Method.SET_NAME) {
            run {
                val value = settings.value
                value.system.name = it
                settings.value = value
            }

            deviceManager.send(listOf(), null)
        }

        bridge.on<Unit, List<DeviceManager.Device>>(Bridge.Method.GET_DEVICES) {
            deviceManager.all
        }

        bridge.on<Unit, Settings.Model>(Bridge.Method.GET_SETTINGS) {
            settings.value
        }

        bridge.on<Settings.Model, Unit>(Bridge.Method.SET_SETTINGS) {
            settings.value = it;
        }

        bridge.on<String, Array<Bridge.Source>>(Bridge.Method.GET_CAPTURE_SOURCES) {
            when (it) {
                "Screen" -> {
                    arrayOf(
                        Bridge.Source(
                            id = "default",
                            index = 0,
                            isDefault = true,
                            kind = "Screen",
                            name = "Main display"
                        )
                    )
                }

                "Audio" -> {
                    arrayOf(
                        Bridge.Source(
                            id = "default",
                            index = 0,
                            isDefault = true,
                            kind = "Audio",
                            name = "Main display audio"
                        )
                    )
                }

                else -> arrayOf()
            }
        }

        bridge.on<Bridge.CreateSenderParams, Unit>(Bridge.Method.CREATE_SENDER) {
            val (targets, options) = it

            Handler(Looper.getMainLooper()).post {
                if (status == Bridge.Status.Idle &&
                    serviceBinder != null &&
                    options.media.video != null
                ) {

                    permissions.request { intent ->
                        if (ContextCompat.checkSelfPermission(
                                this,
                                Manifest.permission.RECORD_AUDIO
                            ) == PackageManager.PERMISSION_GRANTED
                        ) {
                            val description = serviceBinder!!.createSender(
                                intent, HylaranaSenderConfigure(
                                    options = options.transport,
                                    video = Video.VideoEncoder.VideoEncoderConfigure(
                                        format = MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface,
                                        frameRate = options.media.video.options.frameRate,
                                        bitRate = options.media.video.options.bitRate,
                                        height = options.media.video.options.height,
                                        width = options.media.video.options.width,
                                    )
                                ), object : HylaranaCoreService.Observer() {
                                    override fun onClosed() {
                                        deviceManager.send(listOf(), null)

                                        status = Bridge.Status.Idle
                                        bridge.emit(Bridge.Event.STATUS_CHANGE)
                                    }
                                })

                            description?.let {
                                deviceManager.send(targets, description)

                                status = Bridge.Status.Sending
                                bridge.emit(Bridge.Event.STATUS_CHANGE)
                            }
                        }
                    }
                }
            }
        }

        bridge.on<Unit, Unit>(Bridge.Method.CLOSE_SENDER) {
            serviceBinder?.stopSender()
        }

        bridge.on<Bridge.CreateReceiverParams, Unit>(Bridge.Method.CREATE_RECEIVER) {
            val (_, _, description) = it

            Handler(Looper.getMainLooper()).post {
                if (status == Bridge.Status.Idle && serviceBinder != null) {
                    isSurfaceShow = true

                    serviceBinder!!.createReceiver(
                        surface,
                        description,
                        object : HylaranaCoreService.Observer() {
                            override fun onClosed() {
                                isSurfaceShow = false

                                status = Bridge.Status.Idle
                                bridge.emit(Bridge.Event.STATUS_CHANGE)
                            }
                        })

                    status = Bridge.Status.Receiving
                    bridge.emit(Bridge.Event.STATUS_CHANGE)
                }
            }
        }

        bridge.on<Unit, Unit>(Bridge.Method.CLOSE_RECEIVER) {
            serviceBinder?.stopReceiver()
        }

        scope.launch {
            for (x in deviceManager.watcher) {
                Log.i("hylarana", "device manager change event, send notify to webview")

                bridge.emit(Bridge.Event.DEVICE_CHANGE)
            }
        }

        bridge.emit(Bridge.Event.READY)
    }

    @SuppressLint("ClickableViewAccessibility")
    @Composable
    fun Activity() {
        var isSurfaceCloseButtonShow by remember { mutableStateOf(false) }

        DisposableEffect(isSurfaceShow) {
            if (isSurfaceShow) {
                window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            } else {
                window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }

            onDispose {
                window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }

        LaunchedEffect(isSurfaceShow) {
            val windowInsetsController = WindowInsetsControllerCompat(window, window.decorView)

            if (isSurfaceShow) {
                WindowCompat.setDecorFitsSystemWindows(window, false)
                windowInsetsController.hide(WindowInsetsCompat.Type.systemBars())
                windowInsetsController.systemBarsBehavior =
                    WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            } else {
                WindowCompat.setDecorFitsSystemWindows(window, true)
                windowInsetsController.show(WindowInsetsCompat.Type.systemBars())
            }
        }

        Box(modifier = Modifier.fillMaxSize()) {
            AndroidView(
                modifier = Modifier
                    .fillMaxSize()
                    .zIndex(if (isSurfaceShow) 0f else 1f),
                factory = { Frontend(it, bridge) }
            )

            AndroidView(
                modifier = Modifier
                    .fillMaxSize()
                    .zIndex(if (isSurfaceShow) 1f else 0f),
                factory = {
                    SurfaceView(it).apply {
                        surface = holder.surface

                        holder.setFormat(PixelFormat.TRANSLUCENT)

                        setOnTouchListener { _, _ ->
                            if (!isSurfaceCloseButtonShow) {
                                isSurfaceCloseButtonShow = true
                                Handler(Looper.getMainLooper()).postDelayed({
                                    isSurfaceCloseButtonShow = false
                                }, 5000)
                            }

                            false
                        }
                    }
                }
            )

            if (isSurfaceShow && isSurfaceCloseButtonShow) {
                Box(
                    modifier = Modifier
                        .zIndex(2f)
                        .fillMaxSize()
                        .background(Color.Black.copy(alpha = 0.5f))
                        .align(Alignment.Center)
                ) {
                    IconButton(
                        onClick = {
                            serviceBinder?.stopReceiver()
                        },
                        modifier = Modifier
                            .zIndex(3f)
                            .size(80.dp)
                            .align(Alignment.Center)
                            .background(Color.Black.copy(alpha = 0.5f), shape = CircleShape)
                    ) {
                        Icon(
                            tint = Color.White,
                            contentDescription = "Close",
                            painter = painterResource(id = R.drawable.power_off),
                            modifier = Modifier
                                .zIndex(4f)
                                .size(50.dp)
                        )
                    }
                }
            }
        }
    }

    class Permissions(private val activity: MainActivity) {
        private var callback: ((Intent) -> Unit)? = null

        private val captureScreenPermission =
            activity.registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
                if (result.resultCode == Activity.RESULT_OK && result.data != null) {
                    callback?.let { it(result.data!!) }
                } else {
                    Log.e("hylarana", "failed to request screen capture permission")
                }

                callback = null
            }

        private val captureAudioPermission =
            activity.registerForActivityResult(ActivityResultContracts.RequestPermission()) { isGranted ->
                if (isGranted) {
                    captureScreenPermission.launch(
                        (activity.getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager).createScreenCaptureIntent()
                    )
                } else {
                    Log.e("hylarana", "failed to request audio record permission")

                    callback = null
                }
            }

        fun request(callback: (Intent) -> Unit) {
            this.callback = callback

            captureAudioPermission.launch(Manifest.permission.RECORD_AUDIO)
        }
    }
}