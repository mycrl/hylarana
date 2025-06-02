package com.github.mycrl.hylarana

abstract class DiscoveryServiceObserver {

    /**
     * The query service has yielded results.
     */
    abstract fun onLine(localId: String, id: String, ip: String)

    abstract fun offLine(localId: String, id: String, ip: String)

    abstract fun onMetadata(localId: String, id: String, ip: String, metadata: ByteArray)
}

class DiscoveryService(
    private val setMetadataHandle: (ByteArray) -> Boolean,
    private val releaseHandle: () -> Unit
) {
    fun setMetadata(metadata: ByteArray): Boolean {
        return setMetadataHandle(metadata)
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
    fun createService(bind: String, observer: DiscoveryServiceObserver): DiscoveryService {
        val discovery = discoveryCreate(bind, observer)
        if (discovery == 0L) {
            throw Exception("failed to create discovery service")
        }

        return DiscoveryService({ metadata ->
            discoverySetMetadata(discovery, metadata)
        }, { ->
            discoveryRelease(discovery)
        })
    }

    /**
     * Register the service, the service type is fixed, you can customize the
     * port number, id is the identifying information of the service, used to
     * distinguish between different publishers, in properties you can add
     * customized data to the published service.
     */
    private external fun discoveryCreate(bind: String, observer: DiscoveryServiceObserver): Long

    /**
     * Query the registered service, the service type is fixed, when the query
     * is published the callback function will call back all the network
     * addresses of the service publisher as well as the attribute information.
     */
    private external fun discoverySetMetadata(discovery: Long, metadata: ByteArray): Boolean

    /**
     * release the discovery service
     */
    private external fun discoveryRelease(discovery: Long)
}