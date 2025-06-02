// SRT (Secure Reliable Transport) protocol implementation
// This module provides a safe Rust wrapper around the SRT C library

use std::{
    ffi::{CStr, c_char, c_int, c_void},
    fmt::Debug,
    io::Error,
    mem::MaybeUninit,
    net::SocketAddr,
    ptr::null_mut,
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use common::strings::PSTR;
use log::{Level, log};
use os_socketaddr::OsSocketAddr;

pub use self::sys::SRT_TRACEBSTATS;

// Include auto-generated bindings for the SRT C library
#[allow(
    dead_code,
    unused_imports,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals
)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// Helper function to get the last SRT error as a Rust Error
fn current_error() -> Error {
    Error::other(
        unsafe { CStr::from_ptr(sys::srt_getlasterror_str()) }
            .to_str()
            .map(|s| s.to_string())
            .ok()
            .unwrap_or_default(),
    )
}

// SRT logging levels mapping
#[repr(C)]
#[allow(unused)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SRT_LOG_LEVEL {
    LOG_EMERG = 0,
    LOG_ALERT,
    LOG_CRIT,
    LOG_ERR,
    LOG_WARNING,
    LOG_NOTICE,
    LOG_INFO,
    LOG_DEBUG,
}

// Custom log handler for SRT that maps SRT log levels to Rust's log levels
unsafe extern "C" fn loghandler(
    _ctx: *mut c_void,
    level: c_int,
    _file: *const c_char,
    _line: c_int,
    area: *const c_char,
    message: *const c_char,
) {
    if let (Ok(area), Ok(message)) = (
        PSTR::from(area).to_string(),
        PSTR::from(message).to_string(),
    ) {
        let level = match unsafe { std::mem::transmute(level) } {
            SRT_LOG_LEVEL::LOG_EMERG | SRT_LOG_LEVEL::LOG_CRIT | SRT_LOG_LEVEL::LOG_ERR => {
                Level::Error
            }
            SRT_LOG_LEVEL::LOG_ALERT | SRT_LOG_LEVEL::LOG_WARNING => Level::Warn,
            SRT_LOG_LEVEL::LOG_NOTICE | SRT_LOG_LEVEL::LOG_INFO => Level::Info,
            SRT_LOG_LEVEL::LOG_DEBUG => Level::Debug,
        };

        log!(
            target: "srt",
            level,
            "area={}, message={}",
            area,
            message.replace(['\r', '\n'], "")
        );
    }
}

// Initialize SRT library and set up logging
pub fn startup() -> bool {
    unsafe { sys::srt_setloglevel(SRT_LOG_LEVEL::LOG_INFO as c_int) }
    unsafe { sys::srt_setloghandler(null_mut(), Some(loghandler)) }
    unsafe { sys::srt_startup() != -1 }
}

// Cleanup SRT library resources
pub fn cleanup() {
    unsafe {
        sys::srt_cleanup();
    }
}

// Configuration options for SRT connections
#[derive(Debug, Clone)]
pub struct SrtOptions {
    pub max_bandwidth: i64, // Maximum bandwidth in bytes per second
    pub latency: u32,       // Latency in milliseconds
    pub timeout: u32,       // Connection timeout in milliseconds
    pub fec: String,        // Forward Error Correction configuration
    pub mtu: u32,           // Maximum Transmission Unit size
    pub fc: u32,            // Flow control window size
}

impl SrtOptions {
    // Apply SRT socket options to a socket
    fn apply_socket(&self, fd: i32) -> Result<(), Error> {
        // Set transmission type to live mode
        set_sock_opt(
            fd,
            sys::SRT_SOCKOPT::SRTO_TRANSTYPE,
            &sys::SRT_TRANSTYPE::SRTT_LIVE,
        )?;

        // Enable synchronous receive mode
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_RCVSYN, &1_i32)?;

        // Disable synchronous send mode
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_SNDSYN, &0_i32)?;

        // Enable timestamp-based packet delivery mode
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_TSBPDMODE, &1_i32)?;

        // Enable too-late packet drop
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_TLPKTDROP, &1_i32)?;

        // Set flow control window size
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_FC, &self.fc)?;

        // Set maximum segment size
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_MSS, &self.mtu)?;

        // Set receive latency
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_RCVLATENCY, &self.latency)?;

        // Set maximum bandwidth
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_MAXBW, &self.max_bandwidth)?;

        // Set peer idle timeout
        set_sock_opt(fd, sys::SRT_SOCKOPT::SRTO_PEERIDLETIMEO, &self.timeout)?;

        // Set packet filter (FEC configuration)
        set_sock_opt_str(fd, sys::SRT_SOCKOPT::SRTO_PACKETFILTER, &self.fec)?;

        Ok(())
    }
}

impl Default for SrtOptions {
    fn default() -> Self {
        Self {
            fec: "fec,layout:staircase,rows:2,cols:10,arq:onreq".to_string(),
            max_bandwidth: -1,
            timeout: 2000,
            latency: 60,
            mtu: 1500,
            fc: 25600,
        }
    }
}

// Helper function to set socket options with type safety
fn set_sock_opt<T: Sized + Debug + PartialEq>(
    sock: sys::SRTSOCKET,
    opt: sys::SRT_SOCKOPT,
    flag: &T,
) -> Result<(), Error> {
    if unsafe {
        sys::srt_setsockflag(
            sock,
            opt,
            flag as *const T as *const _,
            size_of::<T>() as c_int,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(current_error())
    }
}

// Helper function to set string socket options
fn set_sock_opt_str(sock: sys::SRTSOCKET, opt: sys::SRT_SOCKOPT, flag: &str) -> Result<(), Error> {
    if unsafe {
        sys::srt_setsockflag(
            sock,
            opt,
            PSTR::from(flag).as_ptr() as *const _,
            flag.len() as c_int,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(current_error())
    }
}

// Fragment encoder for breaking large messages into smaller packets
pub struct FragmentEncoder {
    max_pkt_size: usize,    // Maximum packet size
    packets: Vec<BytesMut>, // Buffer for packet fragments
    sequence: u32,          // Sequence number for packet ordering
}

impl FragmentEncoder {
    const HEAD_SIZE: usize = 8; // Size of packet header (sequence + size)

    // Create a new fragment encoder with specified MTU
    pub fn new(mtu: usize) -> Self {
        Self {
            max_pkt_size: (mtu as usize) - (1500 - 1316), // Adjust for SRT overhead
            packets: Default::default(),
            sequence: 0,
        }
    }

    // Encode a message into multiple fragments
    pub fn encode(&mut self, bytes: &[u8]) -> &[BytesMut] {
        let mut size = 0;

        // Split message into chunks that fit within max_pkt_size
        for (i, chunk) in bytes
            .chunks(self.max_pkt_size - Self::HEAD_SIZE)
            .enumerate()
        {
            {
                if self.packets.get(i).is_none() {
                    self.packets
                        .push(BytesMut::with_capacity(self.max_pkt_size));
                }
            }

            if let Some(buf) = self.packets.get_mut(i) {
                buf.clear();

                // Add sequence number and total size to header
                buf.put_u32(self.sequence);
                buf.put_u32(bytes.len() as u32);
                buf.extend_from_slice(chunk);

                size += 1;
            }
        }

        self.sequence = self.sequence.wrapping_add(1);
        &self.packets[..size]
    }
}

// Fragment decoder for reassembling packets into complete messages
pub struct FragmentDecoder {
    bytes: BytesMut,    // Buffer for reassembling fragments
    last_sequence: u32, // Last processed sequence number
    last_size: usize,   // Size of the complete message
}

impl Default for FragmentDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentDecoder {
    // Create a new fragment decoder with default buffer size
    pub fn new() -> Self {
        Self {
            bytes: BytesMut::with_capacity(4096 * 1024), // 4MB buffer
            last_sequence: u32::MAX,
            last_size: 0,
        }
    }

    // Decode a fragment and return complete message if available
    pub fn decode(&mut self, mut bytes: &[u8]) -> Option<Bytes> {
        let mut packet = None;

        // Extract header information
        let sequence = bytes.get_u32();
        let size = bytes.get_u32() as usize;

        // Check if this is a new message
        if sequence != self.last_sequence {
            if !self.bytes.is_empty() && self.bytes.len() >= self.last_size {
                packet = Some(Bytes::copy_from_slice(&self.bytes[..self.last_size]));
            }

            self.bytes.clear();
        }

        // Add fragment to buffer
        self.bytes.put(bytes);

        self.last_sequence = sequence;
        self.last_size = size;

        packet
    }
}

// SRT socket wrapper for client connections
// Provides a safe interface to interact with SRT sockets
pub struct SrtSocket {
    fd: sys::SRTSOCKET, // SRT socket file descriptor
}

impl SrtSocket {
    // Internal constructor used by connect() and accept()
    fn new(fd: sys::SRTSOCKET) -> Self {
        Self { fd }
    }

    // Get connection statistics including bandwidth, latency, and packet loss
    pub fn get_stats(&self) -> Result<sys::SRT_TRACEBSTATS, Error> {
        let mut stats = MaybeUninit::<sys::SRT_TRACEBSTATS>::uninit();
        if unsafe { sys::srt_bstats(self.fd, stats.as_mut_ptr(), true as i32) } != 0 {
            return Err(current_error());
        }

        Ok(unsafe { stats.assume_init() })
    }

    // Establishes a new SRT connection in live mode
    pub fn connect(addr: SocketAddr, opt: SrtOptions) -> Result<Self, Error> {
        let fd = unsafe { sys::srt_create_socket() };
        if fd == sys::SRT_INVALID_SOCK {
            return Err(current_error());
        } else {
            opt.apply_socket(fd)?;
        }

        let addr: OsSocketAddr = addr.into();
        if unsafe { sys::srt_connect(fd, addr.as_ptr() as *const _, addr.len() as c_int) } == -1 {
            return Err(current_error());
        }

        Ok(Self::new(fd))
    }

    // Blocking read operation that waits for data
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        let size = unsafe {
            sys::srt_recv(
                self.fd,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as c_int,
            )
        };
        if size < 0 {
            return Err(current_error());
        }

        Ok(size as usize)
    }

    // Send data through the socket
    // Note: Data size must not exceed MTU size, use FragmentEncoder for larger
    // messages
    pub fn send(&self, buffer: &[u8]) -> Result<(), Error> {
        if buffer.is_empty() {
            return Ok(());
        }

        if unsafe { sys::srt_send(self.fd, buffer.as_ptr() as *const _, buffer.len() as c_int) }
            != buffer.len() as i32
        {
            return Err(current_error());
        }

        Ok(())
    }

    // Close the socket and release resources
    pub fn close(&self) {
        unsafe { sys::srt_close(self.fd) };
    }
}

// Ensures proper cleanup of SRT resources
impl Drop for SrtSocket {
    fn drop(&mut self) {
        self.close()
    }
}

// SRT server for accepting incoming connections
// Handles socket binding, listening, and connection acceptance
pub struct SrtServer {
    fd: sys::SRTSOCKET, // SRT server socket file descriptor
}

// Allows server to be used in multi-threaded environments
unsafe impl Send for SrtServer {}
unsafe impl Sync for SrtServer {}

impl SrtServer {
    // Initializes server socket and starts listening
    pub fn bind(addr: SocketAddr, opt: SrtOptions, backlog: u32) -> Result<Self, Error> {
        let fd = unsafe { sys::srt_create_socket() };
        if fd == sys::SRT_INVALID_SOCK {
            return Err(current_error());
        } else {
            opt.apply_socket(fd)?;
        }

        let addr: OsSocketAddr = addr.into();
        if unsafe { sys::srt_bind(fd, addr.as_ptr() as *const _, addr.len() as c_int) } == -1 {
            return Err(current_error());
        }

        if unsafe { sys::srt_listen(fd, backlog as c_int) } == -1 {
            return Err(current_error());
        }

        Ok(Self { fd })
    }

    // Blocking operation that waits for new client connections
    // Returns a new socket for client communication and its address
    pub fn accept(&self) -> Result<(SrtSocket, SocketAddr), Error> {
        let status = unsafe { sys::srt_getsockstate(self.fd) };
        if status != sys::SRT_SOCKSTATUS::SRTS_LISTENING {
            return Err(Error::other(format!("{:?}", status)));
        }

        let mut addr = OsSocketAddr::new();
        let mut addrlen = addr.capacity() as c_int;

        let fd = unsafe { sys::srt_accept(self.fd, addr.as_mut_ptr() as *mut _, &mut addrlen) };
        if fd != sys::SRT_INVALID_SOCK {
            if let Some(addr) = addr.into() {
                return Ok((SrtSocket::new(fd), addr));
            }
        }

        Err(current_error())
    }

    /// Extracts the address to which the socket was bound. Although you should
    /// know the address(es) that you have used for binding yourself, this
    /// function can be useful for extracting the local outgoing port number
    /// when it was specified as 0 with binding for system autoselection. With
    /// this function you can extract the port number after it has been
    /// autoselected.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        let mut addr = OsSocketAddr::new();
        let mut addrlen = addr.capacity() as c_int;
        unsafe {
            sys::srt_getsockname(self.fd, addr.as_mut_ptr() as *mut _, &mut addrlen);
        }

        addr.into()
    }

    // Close server socket and stop accepting connections
    pub fn close(&self) {
        unsafe { sys::srt_close(self.fd) };
    }
}

// Ensures proper cleanup of SRT resources
impl Drop for SrtServer {
    fn drop(&mut self) {
        self.close()
    }
}
