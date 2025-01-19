use std::{
    io::Error,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    thread,
};

use crate::{
    adapter::StreamReceiverAdapterAbstract, MulticastSocket, StreamInfo, StreamInfoKind,
    StreamMultiReceiverAdapter, StreamReceiverAdapter, TransmissionFragmentDecoder,
    TransmissionOptions, TransmissionSocket, TransportOptions, TransportStrategy, UnPackage,
};

enum Socket {
    MulticastSocket(Arc<MulticastSocket>),
    TransmissionSocket(Arc<TransmissionSocket>),
}

pub struct Receiver<T: StreamReceiverAdapterAbstract> {
    socket: Option<Socket>,
    adapter: Arc<T>,
}

impl<T: Default + StreamReceiverAdapterAbstract> Default for Receiver<T> {
    fn default() -> Self {
        Self {
            adapter: Arc::new(T::default()),
            socket: None,
        }
    }
}

impl<T: StreamReceiverAdapterAbstract> Receiver<T> {
    pub fn get_adapter(&self) -> Arc<T> {
        self.adapter.clone()
    }

    pub fn close(&self) {
        self.adapter.close();
    }
}

impl<T: StreamReceiverAdapterAbstract> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close();

        if let Some(socket) = self.socket.as_ref() {
            match socket {
                Socket::MulticastSocket(socket) => socket.close(),
                Socket::TransmissionSocket(socket) => socket.close(),
            }
        }
    }
}

fn create_multicast_receiver<T>(id: String, addr: SocketAddr) -> Result<Receiver<T>, Error>
where
    T: Default + StreamReceiverAdapterAbstract + 'static,
{
    let mut receiver = Receiver::<T>::default();

    // Creating a multicast receiver
    let socket = Arc::new(MulticastSocket::new(
        match addr.ip() {
            IpAddr::V4(v4) => v4,
            IpAddr::V6(_) => unimplemented!("not supports ipv6 multicast"),
        },
        SocketAddr::new("0.0.0.0".parse().unwrap(), addr.port()),
        20,
    )?);

    log::info!("create multicast receiver, id={}, addr={}", id, addr);
    receiver.socket = Some(Socket::MulticastSocket(socket.clone()));

    let mut sequence = 0;
    let adapter_ = Arc::downgrade(&receiver.adapter);
    thread::Builder::new()
        .name("HylaranaStreamMulticastReceiverThread".to_string())
        .spawn(move || {
            while let Some((seq, bytes)) = socket.read() {
                if bytes.is_empty() {
                    break;
                }

                if let Some(adapter) = adapter_.upgrade() {
                    // Check whether the sequence number is continuous, in
                    // order to check whether packet loss has occurred
                    if seq == 0 || seq - 1 == sequence {
                        if let Some((info, package)) = UnPackage::unpack(bytes) {
                            if !adapter.send(package, info.kind, info.flags, info.timestamp) {
                                log::error!("adapter on buf failed.");

                                break;
                            }
                        } else {
                            adapter.lose();
                        }
                    } else {
                        adapter.lose()
                    }

                    sequence = seq;
                } else {
                    break;
                }
            }

            log::warn!("multicast receiver is closed, id={}, addr={}", id, addr);

            if let Some(adapter) = adapter_.upgrade() {
                adapter.close();
            }
        })?;

    Ok(receiver)
}

fn create_srt_receiver<T>(id: String, addr: SocketAddr, mtu: usize) -> Result<Receiver<T>, Error>
where
    T: Default + StreamReceiverAdapterAbstract + 'static,
{
    let mut receiver = Receiver::<T>::default();

    // Create an srt configuration and carry stream information
    let mut opt = TransmissionOptions::default();
    opt.fc = 32;
    opt.latency = 20;
    opt.mtu = mtu as u32;
    opt.stream_id = Some(
        StreamInfo {
            kind: StreamInfoKind::Subscriber,
            id: id.clone(),
        }
        .to_string(),
    );

    // Create an srt connection to the server
    let socket = Arc::new(TransmissionSocket::connect(addr, opt)?);

    log::info!("receiver connect to srt server, id={}, addr={}", id, addr);
    receiver.socket = Some(Socket::TransmissionSocket(socket.clone()));

    let mut sequence = 0;
    let adapter_ = Arc::downgrade(&receiver.adapter);
    thread::Builder::new()
        .name("HylaranaStreamReceiverThread".to_string())
        .spawn(move || {
            let mut buf = [0u8; 2000];
            let mut decoder = TransmissionFragmentDecoder::new();

            loop {
                match socket.read(&mut buf) {
                    Ok(size) => {
                        if size == 0 {
                            break;
                        }

                        // All the fragments received from SRT are split and need to be
                        // reassembled here
                        if let Some((seq, bytes)) = decoder.decode(&buf[..size]) {
                            if let Some(adapter) = adapter_.upgrade() {
                                // Check whether the sequence number is continuous, in
                                // order to
                                // check whether packet loss has
                                // occurred
                                if seq == 0 || seq - 1 == sequence {
                                    if let Some((info, package)) = UnPackage::unpack(bytes) {
                                        if !adapter.send(
                                            package,
                                            info.kind,
                                            info.flags,
                                            info.timestamp,
                                        ) {
                                            log::error!("adapter on buf failed.");

                                            break;
                                        }
                                    } else {
                                        adapter.lose();
                                    }
                                } else {
                                    adapter.lose()
                                }

                                sequence = seq;
                            } else {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("{:?}", e);

                        break;
                    }
                }
            }

            log::warn!("srt receiver is closed, id={}, addr={}", id, addr);

            if let Some(adapter) = adapter_.upgrade() {
                adapter.close();
            }
        })?;

    Ok(receiver)
}

fn create_receiver<T: Default + StreamReceiverAdapterAbstract + 'static>(
    id: String,
    options: TransportOptions,
) -> Result<Receiver<T>, Error> {
    match options.strategy {
        TransportStrategy::Multicast(addr) => create_multicast_receiver(id, addr),
        TransportStrategy::Direct(addr) | TransportStrategy::Relay(addr) => {
            create_srt_receiver(id, addr, options.mtu)
        }
    }
}

/// Create channel-separated receivers where audio and video channels are
/// received independently, so that a channel can be easily processed separately
/// from different threads.
pub fn create_split_receiver(
    id: String,
    options: TransportOptions,
) -> Result<Receiver<StreamMultiReceiverAdapter>, Error> {
    create_receiver::<StreamMultiReceiverAdapter>(id, options)
}

/// Creating a mixed channel is the opposite of separating channels, where the
/// data from all channels is mixed together, and the data received from the
/// receiver is mixed, and you need to process it yourself by data type.
pub fn create_mix_receiver(
    id: String,
    options: TransportOptions,
) -> Result<Receiver<StreamReceiverAdapter>, Error> {
    create_receiver::<StreamReceiverAdapter>(id, options)
}
