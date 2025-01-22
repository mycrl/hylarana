package com.github.mycrl.hylarana

import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

abstract class DiscoveryServiceQueryObserver {

    /**
     * The query service has yielded results.
     */
    abstract fun resolve(addrs: Array<String>, description: MediaStreamDescription)
}

abstract class DiscoveryServiceQueryRawObserver {

    /**
     * The query service has yielded results.
     */
    abstract fun resolve(addrs: String, description: String)
}

class DiscoveryService(private val releaseHandle: () -> Unit) {
    /**
     * release the discovery service
     */
    fun release() {
        releaseHandle()
    }
}

/**
 * LAN service discovery, which exposes its services through the MDNS protocol and can allow other
 * nodes or clients to discover the current service.
 */
class Discovery {
    companion object {
        init {
            System.loadLibrary("hylarana")
        }
    }

    /**
     * Register the service, the service type is fixed, you can customize the
     * port number, id is the identifying information of the service, used to
     * distinguish between different publishers, in properties you can add
     * customized data to the published service.
     */
    fun register(port: Int, description: MediaStreamDescription): DiscoveryService {
        val discovery = registerDiscoveryService(port, Json.encodeToString(description))
        if (discovery == 0L) {
            throw Exception("failed to register discovery service")
        }

        return DiscoveryService {
            run { releaseDiscoveryService(discovery) }
        }
    }

    /**
     * Query the registered service, the service type is fixed, when the query
     * is published the callback function will call back all the network
     * addresses of the service publisher as well as the attribute information.
     */
    fun query(observer: DiscoveryServiceQueryObserver): DiscoveryService {
        val discovery = queryDiscoveryService(object : DiscoveryServiceQueryRawObserver() {
            override fun resolve(addrs: String, description: String) {
                observer.resolve(Json.decodeFromString(addrs), Json.decodeFromString(description))
            }
        })
        if (discovery == 0L) {
            throw Exception("failed to query discovery service")
        }

        return DiscoveryService {
            run { releaseDiscoveryService(discovery) }
        }
    }

    /**
     * Register the service, the service type is fixed, you can customize the
     * port number, id is the identifying information of the service, used to
     * distinguish between different publishers, in properties you can add
     * customized data to the published service.
     */
    private external fun registerDiscoveryService(port: Int, description: String): Long

    /**
     * Query the registered service, the service type is fixed, when the query
     * is published the callback function will call back all the network
     * addresses of the service publisher as well as the attribute information.
     */
    private external fun queryDiscoveryService(observer: DiscoveryServiceQueryRawObserver): Long

    /**
     * release the discovery service
     */
    private external fun releaseDiscoveryService(discovery: Long)
}