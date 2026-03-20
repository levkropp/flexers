/// Socket state management for bridging emulated sockets to host OS sockets
///
/// This module provides a SocketManager that tracks fake socket file descriptors
/// (returned to firmware) and maps them to real host sockets (std::net).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

pub type SocketFd = u32;

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    TcpStream,
    TcpListener,
    Udp,
}

/// Socket state
pub struct SocketState {
    /// Fake fd returned to firmware
    fd: SocketFd,

    /// Socket type (can be upgraded from TcpStream to TcpListener on bind)
    socket_type: SocketType,

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
}

impl SocketState {
    pub fn new_tcp_stream(fd: SocketFd) -> Self {
        Self {
            fd,
            socket_type: SocketType::TcpStream,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
        }
    }

    pub fn new_tcp_listener(fd: SocketFd) -> Self {
        Self {
            fd,
            socket_type: SocketType::TcpListener,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
        }
    }

    pub fn new_udp(fd: SocketFd) -> Self {
        Self {
            fd,
            socket_type: SocketType::Udp,
            tcp_stream: None,
            tcp_listener: None,
            udp_socket: None,
            connected: false,
            bound_addr: None,
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

    /// Send data
    pub fn send(&mut self, data: &[u8]) -> std::io::Result<usize> {
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

        let n = if let Some(ref mut stream) = self.tcp_stream {
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
    pub fn create_socket(&mut self, socket_type: SocketType) -> SocketFd {
        let fd = self.next_fd;
        self.next_fd += 1;

        let state = match socket_type {
            SocketType::TcpStream => SocketState::new_tcp_stream(fd),
            SocketType::TcpListener => SocketState::new_tcp_listener(fd),
            SocketType::Udp => SocketState::new_udp(fd),
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
        let (stream, addr) = if let Some(listener) = self.sockets.get_mut(&listener_fd) {
            listener.accept()?
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid socket fd",
            ));
        };

        // Create new socket for client
        let client_fd = self.next_fd;
        self.next_fd += 1;

        let mut client_state = SocketState::new_tcp_stream(client_fd);
        client_state.tcp_stream = Some(stream);
        client_state.connected = true;

        self.sockets.insert(client_fd, client_state);

        Ok((client_fd, addr))
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
        let fd1 = manager.create_socket(SocketType::TcpStream);
        let fd2 = manager.create_socket(SocketType::Udp);

        assert_eq!(fd1, 3);
        assert_eq!(fd2, 4);
        assert!(manager.get(fd1).is_some());
        assert!(manager.get(fd2).is_some());
    }

    #[test]
    fn test_socket_type_tracking() {
        let mut manager = SocketManager::new();
        let tcp_fd = manager.create_socket(SocketType::TcpStream);
        let udp_fd = manager.create_socket(SocketType::Udp);

        assert_eq!(manager.get(tcp_fd).unwrap().socket_type(), SocketType::TcpStream);
        assert_eq!(manager.get(udp_fd).unwrap().socket_type(), SocketType::Udp);
    }

    #[test]
    fn test_socket_close() {
        let mut manager = SocketManager::new();
        let fd = manager.create_socket(SocketType::TcpStream);

        assert!(manager.get(fd).is_some());
        manager.close(fd);
        assert!(manager.get(fd).is_none());
    }

    #[test]
    fn test_tcp_loopback_connection() {
        // Start a listener on loopback
        let mut listener_socket = SocketState::new_tcp_listener(100);
        let bind_result = listener_socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)));
        assert!(bind_result.is_ok());

        let listen_result = listener_socket.listen();
        assert!(listen_result.is_ok());

        // Get the actual port that was bound
        let listener_addr = listener_socket.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        // Connect to it
        let mut client_socket = SocketState::new_tcp_stream(101);
        let connect_result = client_socket.connect(listener_addr);
        assert!(connect_result.is_ok());
        assert!(client_socket.is_connected());
    }

    #[test]
    fn test_udp_socket_creation() {
        let mut socket = SocketState::new_udp(200);
        let bind_result = socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)));
        assert!(bind_result.is_ok());
        assert!(socket.udp_socket.is_some());
    }

    #[test]
    fn test_nonblocking_recv_returns_zero() {
        // Create a connected TCP socket (to loopback)
        let mut listener = SocketState::new_tcp_listener(300);
        listener.bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        listener.listen().unwrap();

        let listener_addr = listener.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        let mut client = SocketState::new_tcp_stream(301);
        client.connect(listener_addr).unwrap();

        // Try to receive when no data is available (non-blocking)
        let result = client.recv(1024);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);  // Should return empty, not block
    }

    #[test]
    fn test_send_recv_loopback() {
        // Set up listener
        let mut listener = SocketState::new_tcp_listener(400);
        listener.bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        listener.listen().unwrap();
        let listener_addr = listener.tcp_listener.as_ref().unwrap().local_addr().unwrap();

        // Connect client
        let mut client = SocketState::new_tcp_stream(401);
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
}
