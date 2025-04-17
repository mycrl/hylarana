package com.github.mycrl.hylarana

abstract class DiscoveryServiceObserver {

    /**
     * The query service has yielded results.
     */
    abstract fun onLine(localId: String, id: String, ip: String)

    abstract fun offLine(localId: String, id: String, ip: String)

    abstract fun onMessage(localId: String, id: String, ip: String, message: ByteArray)
}

class DiscoveryService(
    private val broadcastHandle: (ByteArray) -> Boolean,
    private val releaseHandle: () -> Unit
) {
    fun broadcast(message: ByteArray): Boolean {
        return broadcastHandle(message)
    }

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
    fun createService(topic: String, observer: DiscoveryServiceObserver): DiscoveryService {
        val discovery = create(topic, observer)
        if (discovery == 0L) {
            throw Exception("failed to create discovery service")
        }

        return DiscoveryService({ message ->
            broadcast(discovery, message)
        }, { ->
            release(discovery)
        })
    }

    /**
     * Register the service, the service type is fixed, you can customize the
     * port number, id is the identifying information of the service, used to
     * distinguish between different publishers, in properties you can add
     * customized data to the published service.
     */
    private external fun create(topic: String, observer: DiscoveryServiceObserver): Long

    /**
     * Query the registered service, the service type is fixed, when the query
     * is published the callback function will call back all the network
     * addresses of the service publisher as well as the attribute information.
     */
    private external fun broadcast(discovery: Long, message: ByteArray): Boolean

    /**
     * release the discovery service
     */
    private external fun release(discovery: Long)
}