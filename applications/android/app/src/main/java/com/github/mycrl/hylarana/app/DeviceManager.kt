package com.github.mycrl.hylarana.app

import android.util.Log
import com.github.mycrl.hylarana.Discovery
import com.github.mycrl.hylarana.DiscoveryService
import com.github.mycrl.hylarana.DiscoveryServiceObserver
import com.github.mycrl.hylarana.MediaStreamDescription
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

class DeviceManager(private var settings: Settings, scope: CoroutineScope) {
    private val channel = Channel<Unit>()
    private var discovery: DiscoveryService? = null
    private val devices: HashMap<String, Device> = HashMap()

    val all: List<Device> get() = devices.values.toList()
    val watcher: Channel<Unit> get() = channel

    init {
        discovery =
            Discovery().createService(settings.value.network.bind, object : DiscoveryServiceObserver() {
                override fun onLine(localId: String, id: String, ip: String) {
                    Log.i("hylarana", "device manager on line, local_id=$localId, id=$id, ip=$ip")
                }

                override fun offLine(localId: String, id: String, ip: String) {
                    Log.i("hylarana", "device manager off line, local_id=$localId, id=$id")

                    devices.remove(id)

                    scope.launch {
                        channel.send(Unit)
                    }
                }

                override fun onMetadata(
                    localId: String,
                    id: String,
                    ip: String,
                    metadata: ByteArray
                ) {
                    Log.i("hylarana", "device manager on metadata")

                    try {
                        val (targets, name, kind, description) = Json.decodeFromString<ServiceMessage>(
                            String(metadata, Charsets.UTF_8)
                        )

                        Log.i(
                            "hylarana",
                            "device manager on message, targets=$targets, name=$name, kind=$kind, description=$description"
                        )

                        if (targets.isEmpty() || targets.contains(localId)) {
                            devices[id] = Device(name, ip, kind, description)

                            scope.launch {
                                channel.send(Unit)
                            }
                        }
                    } catch (e: Error) {
                        Log.e("hylarana", "device manager on metadata error=$e")
                    }
                }
            })
    }

    fun setMetadata(targets: List<String>, metadata: DeviceMetadata?) {
        val payload = ServiceMessage(
            targets = targets,
            name = settings.value.system.name,
            kind = DeviceType.Android,
            metadata = metadata
        )

        Log.i("hylarana", "device manager send message=$payload")

        discovery?.setMetadata(Json.encodeToString(payload).toByteArray(Charsets.UTF_8))
    }

    fun release() {
        discovery?.release()
    }

    @Serializable
    data class DeviceMetadata(
        val port: Int,
        val description: MediaStreamDescription,
    )

    @Serializable
    data class Device(
        val name: String,
        val ip: String,
        val kind: DeviceType,
        val metadata: DeviceMetadata?,
    )

    @Serializable
    enum class DeviceType {
        Windows,
        Android,
        Apple,
        Linux,
    }

    @Serializable
    data class ServiceMessage(
        val targets: List<String>,
        val name: String,
        val kind: DeviceType,
        val metadata: DeviceMetadata?,
    )
}
