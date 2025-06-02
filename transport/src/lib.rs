mod filter;
mod protocol;

use std::io::{Error, ErrorKind, Result};

use bytes::{Buf, BufMut, Bytes, BytesMut};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use self::{
    receiver::{Receiver as TransportReceiver, ReceiverSink as TransportReceiverSink},
    sender::Sender as TransportSender,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TransportOptions {
    /// Maximum Transmission Unit size
    pub mtu: u32,
    // Maximum bandwidth in bytes per second
    pub max_bandwidth: i64,
    // Latency in milliseconds
    pub latency: u32,
    // Connection timeout in milliseconds
    pub timeout: u32,
    // Forward Error Correction configuration
    pub fec: String,
    // Flow control window size
    pub fc: u32,
}

impl Default for TransportOptions {
    fn default() -> Self {
        Self {
            fec: "fec,layout:staircase,rows:2,cols:10,arq:onreq".to_string(),
            max_bandwidth: -1,
            timeout: 2000,
            latency: 20,
            mtu: 1500,
            fc: 32,
        }
    }
}

/// Initialize the SRT communication protocol, mainly initializing some
/// log-related things.
pub fn startup() -> bool {
    protocol::startup()
}

/// Clean up the SRT environment and prepare to exit.
pub fn shutdown() {
    protocol::cleanup()
}

/// Represents different types of data buffers in the transport layer
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    Partial = 0,  // Represents a partial frame or incomplete data
    KeyFrame = 1, // Represents a complete key frame in video streaming
    Config = 2,   // Represents configuration data
}

impl TryFrom<u8> for BufferType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Partial,
            1 => Self::KeyFrame,
            2 => Self::Config,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid buffer type: {}", value),
                ));
            }
        })
    }
}

/// Represents different types of media streams
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    Video = 0, // Video stream
    Audio = 1, // Audio stream
}

impl TryFrom<u8> for StreamType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Video,
            1 => Self::Audio,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid stream type: {}", value),
                ));
            }
        })
    }
}

/// Generic buffer structure for handling different types of data
#[derive(Debug, Clone)]
pub struct Buffer<T> {
    pub stream: StreamType, // Type of stream (video/audio)
    pub ty: BufferType,     // Type of buffer (keyframe/config/etc)
    pub timestamp: u64,     // Timestamp for synchronization
    pub data: T,            // The actual data payload
}

impl<T> Buffer<T> {
    /// Size of the header in bytes for each buffer
    const HEAD_SIZE: usize = 14;

    /// Creates a BytesMut and copies from src to a buffer. The created buffer
    /// contains the initial message header required for message encoding, which
    /// is an optimization to reduce data copying in the process.
    pub fn copy_from_slice(src: &[u8]) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(src.len() + Self::HEAD_SIZE);
        bytes.put_bytes(0, Self::HEAD_SIZE);
        bytes.put(src);
        bytes
    }

    /// Create a BytesMut and initialize it according to the capacity. The
    /// created buffer contains the initialization message header required
    /// for message encoding, which is an optimization to reduce data
    /// copying in the process.
    pub fn with_capacity(size: usize) -> BytesMut {
        BytesMut::zeroed(size + Self::HEAD_SIZE)
    }
}

impl Buffer<BytesMut> {
    /// Encodes the buffer into a network packet format
    /// The result may be null if an empty packet is passed in
    pub(crate) fn encode(mut self, sequence: u32) -> Bytes {
        let size = self.data.len();

        // Temporarily clear the buffer to write header
        unsafe {
            self.data.set_len(0);
        }

        // Write header information
        self.data.put_u32(sequence);
        self.data.put_u8(self.stream as u8);
        self.data.put_u8(self.ty as u8);
        self.data.put_u64(self.timestamp);

        // Restore the original data
        unsafe {
            self.data.set_len(size);
        }

        self.data.freeze()
    }
}

impl Buffer<Bytes> {
    /// Decodes network packets into Buffer structure
    /// Separates different types of data and validates the packet format
    pub(crate) fn decode(mut bytes: Bytes) -> Result<(u32, Buffer<Bytes>)> {
        Ok((
            bytes.get_u32(),
            Buffer {
                stream: StreamType::try_from(bytes.get_u8())?,
                ty: BufferType::try_from(bytes.get_u8())?,
                timestamp: bytes.get_u64(),
                data: bytes,
            },
        ))
    }
}

mod receiver {
    use std::{io::Error, net::SocketAddr, sync::Arc, thread};

    use bytes::Bytes;

    use super::{
        Buffer, TransportOptions,
        filter::StreamConsumer,
        protocol::{FragmentDecoder, SrtOptions, SrtSocket},
    };

    /// Trait for handling received data
    pub trait ReceiverSink: Send {
        /// Process received buffer data
        /// Returns false if processing should stop
        fn sink(&mut self, buffer: Buffer<Bytes>) -> bool;
        /// Cleanup when receiver is closed
        fn close(&mut self);
    }

    /// Handles receiving data over SRT protocol
    pub struct Receiver {
        socket: Arc<SrtSocket>,
    }

    impl Receiver {
        /// Creates a new receiver with specified options and sink
        /// Establishes SRT connection and spawns a thread for data processing
        pub fn new<S: ReceiverSink + 'static>(
            addr: SocketAddr,
            options: TransportOptions,
            mut sinker: S,
        ) -> Result<Self, Error> {
            log::info!("transport create receiver, addr={}", addr);

            // Create SRT connection with optimized settings
            let socket = Arc::new(SrtSocket::connect(addr, {
                let mut opt = SrtOptions::default();
                opt.max_bandwidth = options.max_bandwidth;
                opt.timeout = options.timeout;
                opt.latency = options.latency;
                opt.fec = options.fec;
                opt.mtu = options.mtu;
                opt.fc = options.fc;

                opt
            })?);

            // Spawn receiver thread
            let socket_ = socket.clone();
            thread::Builder::new()
                .name("HylaranaTransportReceiverThread".to_string())
                .spawn(move || {
                    let mut bytes = [0u8; 4096];
                    let mut decoder = FragmentDecoder::new();
                    let mut consumer = StreamConsumer::default();

                    // Main receive loop
                    loop {
                        match socket_.read(&mut bytes) {
                            Ok(size) => {
                                if size == 0 {
                                    break;
                                }

                                // Process received data
                                if let Some(packet) = decoder.decode(&bytes[..size]) {
                                    if let Some(buffer) = consumer.filter(packet) {
                                        if !sinker.sink(buffer) {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("{:?}", e);
                                break;
                            }
                        }
                    }

                    log::warn!("transport receiver is closed, addr={}", addr);

                    sinker.close();
                })?;

            Ok(Self { socket })
        }
    }

    impl Drop for Receiver {
        fn drop(&mut self) {
            log::info!("transport receiver is drop");

            self.socket.close();
        }
    }
}

mod sender {
    use std::{
        io::{Error, ErrorKind, Result},
        net::SocketAddr,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    };

    use arc_swap::ArcSwapOption;
    use bytes::BytesMut;
    use parking_lot::Mutex;

    use super::{
        Buffer, TransportOptions,
        filter::StreamProducer,
        protocol::{FragmentEncoder, SRT_TRACEBSTATS, SrtOptions, SrtServer, SrtSocket},
    };

    /// Handles sending data over SRT protocol
    pub struct Sender {
        working: Arc<AtomicBool>,
        producer: StreamProducer,
        encoder: Mutex<FragmentEncoder>,
        socket: Arc<ArcSwapOption<SrtSocket>>,
        server: Arc<SrtServer>,
        address: SocketAddr,
    }

    impl Sender {
        /// Creates a new sender with specified options
        /// Initializes SRT server and spawns thread for connection handling
        pub fn new(bind: SocketAddr, options: TransportOptions) -> Result<Self> {
            log::info!("transport create sender, bind={}", bind);

            let working = Arc::new(AtomicBool::new(true));
            let socket: Arc<ArcSwapOption<SrtSocket>> = Default::default();

            // Initialize SRT server with optimized settings
            let server = Arc::new(SrtServer::bind(
                bind,
                {
                    let mut opt = SrtOptions::default();
                    opt.max_bandwidth = options.max_bandwidth;
                    opt.timeout = options.timeout;
                    opt.latency = options.latency;
                    opt.fec = options.fec;
                    opt.mtu = options.mtu;
                    opt.fc = options.fc;

                    opt
                },
                1,
            )?);

            let address = server
                .local_addr()
                .ok_or_else(|| Error::new(ErrorKind::AddrNotAvailable, ""))?;

            // Spawn server thread for connection handling
            let working_ = working.clone();
            let server_ = server.clone();
            let socket_ = Arc::downgrade(&socket);
            thread::Builder::new()
                .name("HylaranaTransportSenderThread".to_string())
                .spawn(move || {
                    while let Ok((socket, addr)) = server_.accept() {
                        if let Some(srt_socket) = socket_.upgrade() {
                            srt_socket.store(Some(Arc::new(socket)));

                            log::info!("transport srt server accept a socket, addr={}", addr);
                        } else {
                            break;
                        }
                    }

                    log::info!("transport srt server is closed, addr={}", address);

                    working_.store(false, Ordering::Relaxed);
                })?;

            Ok(Self {
                encoder: Mutex::new(FragmentEncoder::new(options.mtu as usize)),
                producer: Default::default(),
                address,
                working,
                socket,
                server,
            })
        }

        /// Calculates and returns the packet loss rate
        /// Returns a value between 0.0 and 1.0
        pub fn get_pkt_lose_rate(&self) -> f64 {
            if let Some(socket) = self.socket.load().as_ref() {
                if let Ok(SRT_TRACEBSTATS {
                    pktSndDrop,
                    pktSentUnique,
                    ..
                }) = socket.get_stats()
                {
                    log::info!(
                        "transport pkt send drop={}, send count={}",
                        pktSndDrop,
                        pktSentUnique
                    );

                    return (pktSndDrop as f64 / pktSentUnique as f64 * 10.0).floor() / 10.0;
                }
            }

            0.0
        }

        /// Sends data through the SRT connection
        /// Handles data fragmentation and error recovery
        pub fn send(&self, buffer: Buffer<BytesMut>) -> Result<()> {
            if !self.working.load(Ordering::Relaxed) {
                return Err(Error::new(ErrorKind::NetworkDown, "srt server is closed"));
            }

            if buffer.data.is_empty() {
                return Ok(());
            }

            let mut is_close = false;
            {
                let socket = self.socket.load();
                let mut encoder = self.encoder.lock();

                // Process and send each filtered buffer
                for buffer in self.producer.filter(buffer) {
                    if let Some(socket) = socket.as_ref() {
                        for chunk in encoder.encode(&buffer) {
                            if let Err(e) = socket.send(chunk) {
                                log::warn!(
                                    "transport failed to send data with srt current socket, err={:?}",
                                    e
                                );

                                is_close = true;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }

            if is_close {
                self.socket.store(None);
            }

            Ok(())
        }

        pub fn local_addr(&self) -> SocketAddr {
            self.address
        }
    }

    impl Drop for Sender {
        fn drop(&mut self) {
            log::info!("transport sender is drop");

            self.server.close();
        }
    }
}
