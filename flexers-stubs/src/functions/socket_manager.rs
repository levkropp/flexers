/// Socket state management for bridging emulated sockets to host OS sockets
///
/// This module provides a SocketManager that tracks fake socket file descriptors
/// (returned to firmware) and maps them to real host sockets (std::net).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use rustls::{ClientConnection, StreamOwned as TlsStream};

use super::tls_manager::TLS_MANAGER;

pub type SocketFd = u32;

/// Address family
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddressFamily {
    IPv4,
    IPv6,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    TcpStream,
    TcpListener,
    Udp,
}

/// TLS handshake state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TlsHandshakeState {
    NotStarted,
    InProgress,
    Complete,
    Failed,
}

/// Socket state
pub struct SocketState {
    /// Fake fd returned to firmware
    fd: SocketFd,

    /// Socket type (can be upgraded from TcpStream to TcpListener on bind)
    socket_type: SocketType,

    /// Address family
    address_family: AddressFamily,

    /// TCP stream (if connected)
    tcp_stream: Option<TcpStream>,

    /// TCP listener (if listening)
    tcp_listener: Option<TcpListener>,

    /// UDP socket
    udp_socket: Option<UdpSocket>,

    /// Connected state
    connected: bool,

    /// Bound address (for UDP sockets)
    bound_addr: Option<SocketAddr>,

    /// Socket options
    reuse_addr: bool,
    tcp_nodelay: bool,
    recv_buffer_size: usize,
    send_buffer_size: usize,
    recv_timeout_ms: Option<u64>,

    /// TLS support
    tls_stream: Option<Box<TlsStream<ClientConnection, TcpStream>>>,
    tls_handshake_state: TlsHandshakeState,
    tls_server_name: Option<String>,
}

impl SocketState {
    pub fn new_tcp_stream(fd: SocketFd, family: AddressFamily) -> Self {
        Self {
            fd,
            socket_type: SocketType::TcpStream,
            address_family: family,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
            reuse_addr: false,
            tcp_nodelay: false,
            recv_buffer_size: 8192,
            send_buffer_size: 8192,
            recv_timeout_ms: None,
            tls_stream: None,
            tls_handshake_state: TlsHandshakeState::NotStarted,
            tls_server_name: None,
        }
    }

    pub fn new_tcp_listener(fd: SocketFd, family: AddressFamily) -> Self {
        Self {
            fd,
            socket_type: SocketType::TcpListener,
            address_family: family,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
            reuse_addr: false,
            tcp_nodelay: false,
            recv_buffer_size: 8192,
            send_buffer_size: 8192,
            recv_timeout_ms: None,
            tls_stream: None,
            tls_handshake_state: TlsHandshakeState::NotStarted,
            tls_server_name: None,
        }
    }

    pub fn new_udp(fd: SocketFd, family: AddressFamily) -> Self {
        Self {
            fd,
            socket_type: SocketType::Udp,
            address_family: family,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
            reuse_addr: false,
            tcp_nodelay: false,
            recv_buffer_size: 8192,
            send_buffer_size: 8192,
            recv_timeout_ms: None,
            tls_stream: None,
            tls_handshake_state: TlsHandshakeState::NotStarted,
            tls_server_name: None,
        }
    }

    /// Connect to remote address (TCP only)
    pub fn connect(&mut self, addr: SocketAddr) -> std::io::Result<()> {
        match self.socket_type {
            SocketType::TcpStream => {
                let stream = TcpStream::connect(addr)?;
                stream.set_nonblocking(true)?;
                self.tcp_stream = Some(stream);
                self.connected = true;
                Ok(())
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid socket type for connect",
            )),
        }
    }

    /// Bind socket to address
    pub fn bind(&mut self, addr: SocketAddr) -> std::io::Result<()> {
        match self.socket_type {
            SocketType::TcpStream => {
                // TCP stream socket being bound becomes a listener
                let listener = TcpListener::bind(addr)?;
                listener.set_nonblocking(true)?;
                self.tcp_listener = Some(listener);
                self.bound_addr = Some(addr);
                self.socket_type = SocketType::TcpListener; // Upgrade to listener
                Ok(())
            }
            SocketType::TcpListener => {
                let listener = TcpListener::bind(addr)?;
                listener.set_nonblocking(true)?;
                self.tcp_listener = Some(listener);
                self.bound_addr = Some(addr);
                Ok(())
            }
            SocketType::Udp => {
                let socket = UdpSocket::bind(addr)?;
                socket.set_nonblocking(true)?;
                self.udp_socket = Some(socket);
                self.bound_addr = Some(addr);
                Ok(())
            }
        }
    }

    /// Listen for connections (TCP listener only)
    pub fn listen(&mut self) -> std::io::Result<()> {
        match self.socket_type {
            SocketType::TcpListener => {
                // TcpListener is already listening after bind
                if self.tcp_listener.is_some() {
                    Ok(())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotConnected,
                        "Socket not bound",
                    ))
                }
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid socket type for listen",
            )),
        }
    }

    /// Accept connection (TCP listener only)
    pub fn accept(&mut self) -> std::io::Result<(TcpStream, SocketAddr)> {
        match self.socket_type {
            SocketType::TcpListener => {
                if let Some(ref listener) = self.tcp_listener {
                    let (stream, addr) = listener.accept()?;
                    stream.set_nonblocking(true)?;
                    Ok((stream, addr))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotConnected,
                        "Socket not listening",
                    ))
                }
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid socket type for accept",
            )),
        }
    }

    /// Upgrade existing TCP connection to TLS (non-blocking)
    pub fn start_tls_handshake(&mut self, server_name: &str) -> std::io::Result<()> {
        use rustls::pki_types::ServerName;
        use std::io::{Error, ErrorKind};

        // Take ownership of tcp_stream
        let tcp_stream = self.tcp_stream.take()
            .ok_or_else(|| Error::new(ErrorKind::NotConnected, "No TCP stream"))?;

        // Get TLS config from TLS_MANAGER
        let config = TLS_MANAGER.lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Failed to lock TLS manager"))?
            .get_client_config();

        // Create rustls ServerName with SNI
        let server_name_obj = ServerName::try_from(server_name.to_string())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid SNI hostname"))?;

        // Create ClientConnection
        let conn = ClientConnection::new(config, server_name_obj)
            .map_err(|e| Error::new(ErrorKind::Other, format!("TLS connection error: {}", e)))?;

        // Create TLS stream (already non-blocking from TCP stream)
        let mut tls_stream = TlsStream::new(conn, tcp_stream);

        // Start handshake (may return WouldBlock)
        match tls_stream.conn.complete_io(&mut tls_stream.sock) {
            Ok(_) => {
                self.tls_handshake_state = TlsHandshakeState::Complete;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                self.tls_handshake_state = TlsHandshakeState::InProgress;
            }
            Err(e) => {
                self.tls_handshake_state = TlsHandshakeState::Failed;
                // Put tcp_stream back since handshake failed
                self.tcp_stream = Some(tls_stream.sock);
                return Err(e);
            }
        }

        self.tls_stream = Some(Box::new(tls_stream));
        self.tls_server_name = Some(server_name.to_string());
        Ok(())
    }

    /// Continue TLS handshake (for non-blocking completion)
    pub fn continue_tls_handshake(&mut self) -> std::io::Result<TlsHandshakeState> {
        use std::io::{Error, ErrorKind};

        if self.tls_handshake_state != TlsHandshakeState::InProgress {
            return Ok(self.tls_handshake_state);
        }

        let tls_stream = self.tls_stream.as_mut()
            .ok_or_else(|| Error::new(ErrorKind::Other, "No TLS stream"))?;

        match tls_stream.conn.complete_io(&mut tls_stream.sock) {
            Ok(_) => {
                self.tls_handshake_state = TlsHandshakeState::Complete;
                Ok(TlsHandshakeState::Complete)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                Ok(TlsHandshakeState::InProgress)
            }
            Err(e) => {
                self.tls_handshake_state = TlsHandshakeState::Failed;
                Err(e)
            }
        }
    }

    /// Get TLS handshake state
    pub fn get_tls_handshake_state(&self) -> TlsHandshakeState {
        self.tls_handshake_state
    }

    /// Check if TLS is active
    pub fn is_tls_active(&self) -> bool {
        self.tls_stream.is_some() && self.tls_handshake_state == TlsHandshakeState::Complete
    }

    /// Send data
    pub fn send(&mut self, data: &[u8]) -> std::io::Result<usize> {
        // Priority 1: TLS (if active)
        if let Some(ref mut tls_stream) = self.tls_stream {
            return tls_stream.write(data);
        }

        // Priority 2: Plain TCP
        if let Some(ref mut stream) = self.tcp_stream {
            stream.write(data)
        } else if let Some(ref mut socket) = self.udp_socket {
            socket.send(data)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Socket not connected",
            ))
        }
    }

    /// Send data to specific address (UDP only)
    pub fn send_to(&mut self, data: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        if let Some(ref mut socket) = self.udp_socket {
            socket.send_to(data, addr)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not a UDP socket",
            ))
        }
    }

    /// Receive data (non-blocking)
    pub fn recv(&mut self, max_len: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; max_len];

        let n = if let Some(ref mut tls_stream) = self.tls_stream {
            // TLS read (decryption happens automatically)
            match tls_stream.read(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                Err(e) => return Err(e),
            }
        } else if let Some(ref mut stream) = self.tcp_stream {
            match stream.read(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                Err(e) => return Err(e),
            }
        } else if let Some(ref mut socket) = self.udp_socket {
            match socket.recv(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => 0,
                Err(e) => return Err(e),
            }
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Socket not connected",
            ));
        };

        buf.truncate(n);
        Ok(buf)
    }

    /// Receive data from specific address (UDP only, non-blocking)
    pub fn recv_from(&mut self, max_len: usize) -> std::io::Result<(Vec<u8>, SocketAddr)> {
        if let Some(ref mut socket) = self.udp_socket {
            let mut buf = vec![0u8; max_len];
            match socket.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    buf.truncate(n);
                    Ok((buf, addr))
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    Ok((Vec::new(), SocketAddr::from(([0, 0, 0, 0], 0))))
                }
                Err(e) => Err(e),
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not a UDP socket",
            ))
        }
    }

    /// Get socket type
    pub fn socket_type(&self) -> SocketType {
        self.socket_type
    }

    /// Check if socket is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get TCP listener reference (for testing)
    pub fn tcp_listener(&self) -> Option<&TcpListener> {
        self.tcp_listener.as_ref()
    }

    /// Set socket option
    pub fn set_option(&mut self, level: i32, optname: i32, optval: &[u8]) -> std::io::Result<()> {
        const SOL_SOCKET: i32 = 1;
        const IPPROTO_TCP: i32 = 6;
        const SO_REUSEADDR: i32 = 2;
        const SO_RCVBUF: i32 = 8;
        const SO_SNDBUF: i32 = 7;
        const SO_RCVTIMEO: i32 = 20;
        const TCP_NODELAY: i32 = 1;

        match (level, optname) {
            (SOL_SOCKET, SO_REUSEADDR) => {
                self.reuse_addr = if optval.is_empty() { false } else { optval[0] != 0 };
                // Apply to underlying listener if exists
                if let Some(ref listener) = self.tcp_listener {
                    #[cfg(unix)]
                    {
                        use std::os::unix::io::AsRawFd;
                        let fd = listener.as_raw_fd();
                        unsafe {
                            let optval: i32 = if self.reuse_addr { 1 } else { 0 };
                            libc::setsockopt(
                                fd,
                                libc::SOL_SOCKET,
                                libc::SO_REUSEADDR,
                                &optval as *const _ as *const libc::c_void,
                                std::mem::size_of::<i32>() as libc::socklen_t,
                            );
                        }
                    }
                    #[cfg(windows)]
                    {
                        // Windows requires different handling - reuse_addr is set before bind
                        // For now, just track the setting
                    }
                }
                Ok(())
            }
            (SOL_SOCKET, SO_RCVBUF) => {
                if optval.len() >= 4 {
                    self.recv_buffer_size = u32::from_le_bytes([
                        optval[0], optval[1], optval[2], optval[3]
                    ]) as usize;
                }
                Ok(())
            }
            (SOL_SOCKET, SO_SNDBUF) => {
                if optval.len() >= 4 {
                    self.send_buffer_size = u32::from_le_bytes([
                        optval[0], optval[1], optval[2], optval[3]
                    ]) as usize;
                }
                Ok(())
            }
            (SOL_SOCKET, SO_RCVTIMEO) => {
                if optval.len() >= 8 {
                    let tv_sec = u32::from_le_bytes([
                        optval[0], optval[1], optval[2], optval[3]
                    ]);
                    let tv_usec = u32::from_le_bytes([
                        optval[4], optval[5], optval[6], optval[7]
                    ]);
                    let timeout_ms = (tv_sec as u64 * 1000) + (tv_usec as u64 / 1000);
                    self.recv_timeout_ms = if timeout_ms > 0 { Some(timeout_ms) } else { None };

                    // Apply timeout to underlying socket
                    if let Some(ref stream) = self.tcp_stream {
                        let duration = if let Some(ms) = self.recv_timeout_ms {
                            Some(std::time::Duration::from_millis(ms))
                        } else {
                            None
                        };
                        stream.set_read_timeout(duration)?;
                    }
                    if let Some(ref socket) = self.udp_socket {
                        let duration = if let Some(ms) = self.recv_timeout_ms {
                            Some(std::time::Duration::from_millis(ms))
                        } else {
                            None
                        };
                        socket.set_read_timeout(duration)?;
                    }
                }
                Ok(())
            }
            (IPPROTO_TCP, TCP_NODELAY) => {
                self.tcp_nodelay = if optval.is_empty() { false } else { optval[0] != 0 };
                if let Some(ref stream) = self.tcp_stream {
                    stream.set_nodelay(self.tcp_nodelay)?;
                }
                Ok(())
            }
            _ => Ok(()), // Ignore unknown options
        }
    }

    /// Get socket option
    pub fn get_option(&self, level: i32, optname: i32) -> Vec<u8> {
        const SOL_SOCKET: i32 = 1;
        const IPPROTO_TCP: i32 = 6;
        const SO_REUSEADDR: i32 = 2;
        const SO_RCVBUF: i32 = 8;
        const SO_SNDBUF: i32 = 7;
        const SO_RCVTIMEO: i32 = 20;
        const TCP_NODELAY: i32 = 1;

        match (level, optname) {
            (SOL_SOCKET, SO_REUSEADDR) => {
                vec![if self.reuse_addr { 1 } else { 0 }, 0, 0, 0]
            }
            (SOL_SOCKET, SO_RCVBUF) => {
                (self.recv_buffer_size as u32).to_le_bytes().to_vec()
            }
            (SOL_SOCKET, SO_SNDBUF) => {
                (self.send_buffer_size as u32).to_le_bytes().to_vec()
            }
            (SOL_SOCKET, SO_RCVTIMEO) => {
                let (tv_sec, tv_usec) = if let Some(ms) = self.recv_timeout_ms {
                    ((ms / 1000) as u32, ((ms % 1000) * 1000) as u32)
                } else {
                    (0, 0)
                };
                let mut result = Vec::new();
                result.extend_from_slice(&tv_sec.to_le_bytes());
                result.extend_from_slice(&tv_usec.to_le_bytes());
                result
            }
            (IPPROTO_TCP, TCP_NODELAY) => {
                vec![if self.tcp_nodelay { 1 } else { 0 }, 0, 0, 0]
            }
            _ => vec![0u8; 4], // Unknown option
        }
    }
}

/// Socket manager
pub struct SocketManager {
    /// Fake fd → socket state mapping
    sockets: HashMap<SocketFd, SocketState>,

    /// Next fake fd to allocate
    next_fd: SocketFd,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
            next_fd: 3, // Start at 3 (avoid stdin/stdout/stderr)
        }
    }

    /// Create new socket
    pub fn create_socket(&mut self, socket_type: SocketType, family: AddressFamily) -> SocketFd {
        let fd = self.next_fd;
        self.next_fd += 1;

        let state = match socket_type {
            SocketType::TcpStream => SocketState::new_tcp_stream(fd, family),
            SocketType::TcpListener => SocketState::new_tcp_listener(fd, family),
            SocketType::Udp => SocketState::new_udp(fd, family),
        };

        self.sockets.insert(fd, state);
        fd
    }

    /// Get mutable socket state
    pub fn get_mut(&mut self, fd: SocketFd) -> Option<&mut SocketState> {
        self.sockets.get_mut(&fd)
    }

    /// Get immutable socket state
    pub fn get(&self, fd: SocketFd) -> Option<&SocketState> {
        self.sockets.get(&fd)
    }

    /// Close socket
    pub fn close(&mut self, fd: SocketFd) {
        self.sockets.remove(&fd);
    }

    /// Accept connection and create new socket for client
    pub fn accept(&mut self, listener_fd: SocketFd) -> std::io::Result<(SocketFd, SocketAddr)> {
        // Accept connection on listener socket
        let (stream, addr, family) = if let Some(listener) = self.sockets.get_mut(&listener_fd) {
            let (s, a) = listener.accept()?;
            (s, a, listener.address_family)
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid socket fd",
            ));
        };

        // Create new socket for client
        let client_fd = self.next_fd;
        self.next_fd += 1;

        let mut client_state = SocketState::new_tcp_stream(client_fd, family);
        client_state.tcp_stream = Some(stream);
        client_state.connected = true;

        self.sockets.insert(client_fd, client_state);

        Ok((client_fd, addr))
    }

    /// Check if socket is ready for reading (has data or can accept)
    pub fn is_ready_read(&self, fd: SocketFd) -> bool {
        if let Some(socket) = self.sockets.get(&fd) {
            match socket.socket_type {
                SocketType::TcpStream => {
                    // Try non-blocking peek of 1 byte
                    if let Some(ref stream) = socket.tcp_stream {
                        let mut buf = [0u8; 1];
                        match stream.peek(&mut buf) {
                            Ok(n) => n > 0,
                            Err(_) => false,
                        }
                    } else {
                        false
                    }
                }
                SocketType::TcpListener => {
                    // For listener, we need to check if accept would succeed
                    // This is a bit tricky - we can't peek at accept
                    // We'll use a non-blocking accept and if it succeeds,
                    // we need to store the connection for the actual accept call
                    // For now, return false to avoid complexity
                    // A better implementation would track pending connections
                    false
                }
                SocketType::Udp => {
                    // Try non-blocking peek
                    if let Some(ref socket) = socket.udp_socket {
                        let mut buf = [0u8; 1];
                        match socket.peek(&mut buf) {
                            Ok(n) => n > 0,
                            Err(_) => false,
                        }
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    /// Check if socket is ready for writing
    pub fn is_ready_write(&self, fd: SocketFd) -> bool {
        if let Some(socket) = self.sockets.get(&fd) {
            socket.is_connected() || socket.socket_type == SocketType::Udp
        } else {
            false
        }
    }
}

impl Default for SocketManager {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    pub static ref SOCKET_MANAGER: Arc<Mutex<SocketManager>> =
        Arc::new(Mutex::new(SocketManager::new()));
}

/// Reset socket manager for testing
pub fn reset_socket_manager() {
    // Clear all sockets for testing
    if let Ok(mut manager) = SOCKET_MANAGER.lock() {
        *manager = SocketManager::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_manager_creation() {
        let mut manager = SocketManager::new();
        let fd1 = manager.create_socket(SocketType::TcpStream, AddressFamily::IPv4);
        let fd2 = manager.create_socket(SocketType::Udp, AddressFamily::IPv4);

        assert_eq!(fd1, 3);
        assert_eq!(fd2, 4);
        assert!(manager.get(fd1).is_some());
        assert!(manager.get(fd2).is_some());
    }

    #[test]
    fn test_socket_type_tracking() {
        let mut manager = SocketManager::new();
        let tcp_fd = manager.create_socket(SocketType::TcpStream, AddressFamily::IPv4);
        let udp_fd = manager.create_socket(SocketType::Udp, AddressFamily::IPv4);

        assert_eq!(manager.get(tcp_fd).unwrap().socket_type(), SocketType::TcpStream);
        assert_eq!(manager.get(udp_fd).unwrap().socket_type(), SocketType::Udp);
    }

    #[test]
    fn test_socket_close() {
        let mut manager = SocketManager::new();
        let fd = manager.create_socket(SocketType::TcpStream, AddressFamily::IPv4);

        assert!(manager.get(fd).is_some());
        manager.close(fd);
        assert!(manager.get(fd).is_none());
    }

    #[test]
    fn test_tcp_loopback_connection() {
        // Start a listener on loopback
        let mut listener_socket = SocketState::new_tcp_listener(100, AddressFamily::IPv4);
        let bind_result = listener_socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)));
        assert!(bind_result.is_ok());

        let listen_result = listener_socket.listen();
        assert!(listen_result.is_ok());

        // Get the actual port that was bound
        let listener_addr = listener_socket.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        // Connect to it
        let mut client_socket = SocketState::new_tcp_stream(101, AddressFamily::IPv4);
        let connect_result = client_socket.connect(listener_addr);
        assert!(connect_result.is_ok());
        assert!(client_socket.is_connected());
    }

    #[test]
    fn test_udp_socket_creation() {
        let mut socket = SocketState::new_udp(200, AddressFamily::IPv4);
        let bind_result = socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)));
        assert!(bind_result.is_ok());
        assert!(socket.udp_socket.is_some());
    }

    #[test]
    fn test_nonblocking_recv_returns_zero() {
        // Create a connected TCP socket (to loopback)
        let mut listener = SocketState::new_tcp_listener(300, AddressFamily::IPv4);
        listener.bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        listener.listen().unwrap();

        let listener_addr = listener.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        let mut client = SocketState::new_tcp_stream(301, AddressFamily::IPv4);
        client.connect(listener_addr).unwrap();

        // Try to receive when no data is available (non-blocking)
        let result = client.recv(1024);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);  // Should return empty, not block
    }

    #[test]
    fn test_send_recv_loopback() {
        // Set up listener
        let mut listener = SocketState::new_tcp_listener(400, AddressFamily::IPv4);
        listener.bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        listener.listen().unwrap();
        let listener_addr = listener.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        // Connect client
        let mut client = SocketState::new_tcp_stream(401, AddressFamily::IPv4);
        client.connect(listener_addr).unwrap();

        // Accept connection
        let (mut server_stream, _addr) = listener.accept().unwrap();

        // Send data from client
        let test_data = b"Hello, World!";
        let sent = client.send(test_data).unwrap();
        assert_eq!(sent, test_data.len());

        // Give time for data to arrive
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Receive on server side
        let mut buf = vec![0u8; 1024];
        let received = server_stream.read(&mut buf).unwrap();
        assert_eq!(received, test_data.len());
        assert_eq!(&buf[..received], test_data);
    }

    #[test]
    fn test_tls_handshake_state_tracking() {
        let socket = SocketState::new_tcp_stream(500, AddressFamily::IPv4);
        assert_eq!(socket.get_tls_handshake_state(), TlsHandshakeState::NotStarted);
        assert!(!socket.is_tls_active());
    }

    #[test]
    fn test_tls_requires_tcp_stream() {
        let mut socket = SocketState::new_tcp_stream(501, AddressFamily::IPv4);

        // Should fail without TCP connection
        let result = socket.start_tls_handshake("example.com");
        assert!(result.is_err());
    }
}
