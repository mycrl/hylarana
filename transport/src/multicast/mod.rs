mod dequeue;
mod fragments;

use std::{
    io::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

use bytes::Bytes;
use common::runtime::get_runtime_handle;
use crossbeam::channel::{bounded, Receiver};
use fragments::FragmentEncoder;
use tokio::sync::mpsc::{channel, Sender};

use self::{
    dequeue::Dequeue,
    fragments::{Fragment, FragmentDecoder},
};

/// A UDP socket.
///
/// After creating a UdpSocket by binding it to a socket address, data can be
/// sent to and received from any other socket address.
///
/// Although UDP is a connectionless protocol, this implementation provides an
/// interface to set an address where data should be sent and received from.
/// After setting a remote address with connect, data can be sent to and
/// received from that address with send and recv.
///
/// As stated in the User Datagram Protocol’s specification in IETF RFC 768, UDP
/// is an unordered, unreliable protocol;
///
/// This client is only used to receive multicast packets and does not send
/// multicast packets.
pub struct Socket {
    rx: Receiver<(u64, Bytes)>,
    signal: Sender<()>,
}

unsafe impl Send for Socket {}
unsafe impl Sync for Socket {}

impl Socket {
    /// Creates a UDP socket from the given address.
    ///
    /// You need to specify the multicast group for the udp session to join to
    /// the specified multicast group.
    ///
    /// Note that only IPV4 is supported.
    pub fn new(multicast: Ipv4Addr, bind: SocketAddr, delay: usize) -> Result<Self, Error> {
        assert!(bind.is_ipv4());

        get_runtime_handle().block_on(Self::create(multicast, bind, delay))
    }

    /// Reads packets sent from the multicast server.
    ///
    /// Because the packets are reordered, it is possible to read out more than
    /// one packet at a time.
    ///
    /// Note that there may be packet loss.
    pub fn read(&self) -> Option<(u64, Bytes)> {
        self.rx.recv().ok()
    }

    pub fn close(&self) {
        let _ = self.signal.send(());
    }

    async fn create(multicast: Ipv4Addr, bind: SocketAddr, delay: usize) -> Result<Self, Error> {
        let socket = socket2::Socket::from(UdpSocket::bind(bind)?);
        socket.set_recv_buffer_size(4 * 1024 * 1024)?;
        socket.set_nonblocking(true)?;

        let socket = tokio::net::UdpSocket::from_std(socket.into())?;
        if let IpAddr::V4(bind) = bind.ip() {
            socket.join_multicast_v4(multicast, bind)?;
            socket.set_broadcast(true)?;
        }

        let (signal, mut closed) = channel(1);
        let (tx, rx) = bounded(5);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            let mut queue = Dequeue::new(delay);
            let mut decoder = FragmentDecoder::new();

            'a: loop {
                tokio::select! {
                    Ok(size) = socket.recv(&mut buf[..]) => {
                        if size == 0 {
                            break;
                        }

                        if let Ok(packet) = Fragment::try_from(&buf[..size]) {
                            queue.push(packet);

                            while let Some(chunk) = queue.pop() {
                                if let Some(packet) = decoder.decode(chunk) {
                                    if tx.send(packet).is_err() {
                                        break 'a;
                                    }
                                }
                            }
                        }
                    }
                    _ = closed.recv() => {
                        break
                    }
                    else => break
                }
            }
        });

        Ok(Self { signal, rx })
    }
}

/// A UDP server.
///
/// After creating a UdpSocket by binding it to a socket address, data can be
/// sent to and received from any other socket address.
///
/// Although UDP is a connectionless protocol, this implementation provides an
/// interface to set an address where data should be sent and received from.
/// After setting a remote address with connect, data can be sent to and
/// received from that address with send and recv.
///
/// As stated in the User Datagram Protocol’s specification in IETF RFC 768, UDP
/// is an unordered, unreliable protocol;
///
/// This server is used to send multicast packets to all members of a multicast
/// group.
pub struct Server {
    target: SocketAddr,
    socket: UdpSocket,
    encoder: FragmentEncoder,
}

impl Server {
    /// Creates a UDP socket from the given address.
    ///
    /// You need to specify the multicast group for the udp session to join to
    /// the specified multicast group.
    ///
    /// Note that only IPV4 is supported.
    ///
    /// MTU is used to specify the network unit size, this is used to limit the
    /// maximum size of packets sent.
    pub fn new(multicast: Ipv4Addr, bind: SocketAddr, mtu: usize) -> Result<Self, Error> {
        assert!(bind.is_ipv4());

        let socket = UdpSocket::bind(SocketAddr::new(bind.ip(), 0))?;
        if let IpAddr::V4(bind) = bind.ip() {
            socket.join_multicast_v4(&multicast, &bind)?;
            socket.set_multicast_loop_v4(false)?;
        }

        Ok(Self {
            target: SocketAddr::new(IpAddr::V4(multicast), bind.port()),
            encoder: FragmentEncoder::new(mtu),
            socket,
        })
    }

    /// Sends data on the socket to the remote address to which it is connected.
    ///
    /// Sends the packet to all members of the multicast group.
    ///
    /// Note that there may be packet loss.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.is_empty() {
            return Ok(());
        }

        for chunk in self.encoder.encode(bytes) {
            self.socket.send_to(chunk, self.target)?;
        }

        Ok(())
    }
}
