/// Network (socket) function stubs with real I/O
///
/// These provide real socket API implementation by bridging to host OS sockets.

use crate::handler::RomStubHandler;
use crate::functions::socket_manager::{SocketManager, SocketType, SOCKET_MANAGER};
use flexers_core::cpu::XtensaCpu;
use std::net::{SocketAddr, ToSocketAddrs};

/// Socket constants
const AF_INET: u32 = 2;
const SOCK_STREAM: u32 = 1;
const SOCK_DGRAM: u32 = 2;

/// Helper macro to lock socket manager (handles poisoned mutex in tests)
macro_rules! lock_manager {
    () => {
        SOCKET_MANAGER.lock().unwrap_or_else(|e| e.into_inner())
    };
}

/// Helper function to read null-terminated string from memory
fn read_string_from_memory(cpu: &XtensaCpu, ptr: u32, max_len: usize) -> String {
    if ptr == 0 {
        return String::new();
    }

    let mut bytes = Vec::new();
    for i in 0..max_len {
        let byte = cpu.memory().read_u8(ptr + i as u32);
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8_lossy(&bytes).to_string()
}

/// Helper function to parse sockaddr_in from memory
fn parse_sockaddr(cpu: &XtensaCpu, addr_ptr: u32) -> Option<SocketAddr> {
    if addr_ptr == 0 {
        return None;
    }

    let family = cpu.memory().read_u16(addr_ptr);
    if family != AF_INET as u16 {
        return None;
    }

    let port = cpu.memory().read_u16(addr_ptr + 2);
    let port = u16::from_be(port); // Network byte order

    let ip = cpu.memory().read_u32(addr_ptr + 4);
    // IP is in network byte order (big-endian)
    let addr = SocketAddr::from((
        [
            ((ip >> 24) & 0xFF) as u8,
            ((ip >> 16) & 0xFF) as u8,
            ((ip >> 8) & 0xFF) as u8,
            (ip & 0xFF) as u8,
        ],
        port,
    ));

    Some(addr)
}

/// Helper function to write sockaddr_in to memory
fn write_sockaddr(cpu: &mut XtensaCpu, addr_ptr: u32, addr: SocketAddr) {
    if addr_ptr == 0 {
        return;
    }

    match addr {
        SocketAddr::V4(addr_v4) => {
            cpu.memory().write_u16(addr_ptr, AF_INET as u16);
            cpu.memory().write_u16(addr_ptr + 2, addr_v4.port().to_be());
            let ip = u32::from(*addr_v4.ip());
            cpu.memory().write_u32(addr_ptr + 4, ip);
        }
        _ => {} // Only support IPv4
    }
}

/// socket() - Create socket
///
/// int socket(int domain, int type, int protocol);
pub struct Socket;

impl RomStubHandler for Socket {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let domain = cpu.get_ar(2);
        let sock_type = cpu.get_ar(3);
        let _protocol = cpu.get_ar(4);

        if domain != AF_INET {
            return u32::MAX; // -1 (error)
        }

        let socket_type = match sock_type {
            SOCK_STREAM => SocketType::TcpStream,
            SOCK_DGRAM => SocketType::Udp,
            _ => return u32::MAX,
        };

        lock_manager!()
            .create_socket(socket_type)
    }

    fn name(&self) -> &str {
        "socket"
    }
}

/// bind() - Bind socket to address
///
/// int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
pub struct Bind;

impl RomStubHandler for Bind {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let addr_ptr = cpu.get_ar(3);
        let _addrlen = cpu.get_ar(4);

        let addr = match parse_sockaddr(cpu, addr_ptr) {
            Some(addr) => addr,
            None => return u32::MAX,
        };

        let result = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.bind(addr).ok());

        if result.is_some() {
            0
        } else {
            u32::MAX
        }
    }

    fn name(&self) -> &str {
        "bind"
    }
}

/// connect() - Connect to server
///
/// int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
pub struct Connect;

impl RomStubHandler for Connect {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let addr_ptr = cpu.get_ar(3);
        let _addrlen = cpu.get_ar(4);

        let addr = match parse_sockaddr(cpu, addr_ptr) {
            Some(addr) => addr,
            None => return u32::MAX,
        };

        let result = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.connect(addr).ok());

        if result.is_some() {
            0
        } else {
            u32::MAX
        }
    }

    fn name(&self) -> &str {
        "connect"
    }
}

/// listen() - Listen for connections
///
/// int listen(int sockfd, int backlog);
pub struct Listen;

impl RomStubHandler for Listen {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let _backlog = cpu.get_ar(3);

        let result = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.listen().ok());

        if result.is_some() {
            0
        } else {
            u32::MAX
        }
    }

    fn name(&self) -> &str {
        "listen"
    }
}

/// accept() - Accept connection
///
/// int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
pub struct Accept;

impl RomStubHandler for Accept {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let addr_ptr = cpu.get_ar(3);
        let addrlen_ptr = cpu.get_ar(4);

        let result = lock_manager!().accept(sockfd);

        match result {
            Ok((client_fd, client_addr)) => {
                // Write client address to memory
                if addr_ptr != 0 {
                    write_sockaddr(cpu, addr_ptr, client_addr);
                }

                if addrlen_ptr != 0 {
                    cpu.memory().write_u32(addrlen_ptr, 16);
                }

                client_fd
            }
            Err(e) => {
                // Return -1 for WouldBlock (non-blocking) or other errors
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    u32::MAX
                } else {
                    u32::MAX
                }
            }
        }
    }

    fn name(&self) -> &str {
        "accept"
    }
}

/// send() - Send data
///
/// ssize_t send(int sockfd, const void *buf, size_t len, int flags);
pub struct Send;

impl RomStubHandler for Send {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;
        let _flags = cpu.get_ar(5);

        // Copy data from emulator memory
        let mut data = vec![0u8; len];
        for i in 0..len {
            data[i] = cpu.memory().read_u8(buf_ptr + i as u32);
        }

        // Send via real socket
        let bytes_sent = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.send(&data).ok())
            .unwrap_or(0);

        bytes_sent as u32
    }

    fn name(&self) -> &str {
        "send"
    }
}

/// sendto() - Send data to specific address
///
/// ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
///                const struct sockaddr *dest_addr, socklen_t addrlen);
pub struct SendTo;

impl RomStubHandler for SendTo {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;
        let _flags = cpu.get_ar(5);
        let dest_addr_ptr = cpu.get_ar(6);
        let _addrlen = cpu.get_ar(7);

        // Parse destination address
        let dest_addr = match parse_sockaddr(cpu, dest_addr_ptr) {
            Some(addr) => addr,
            None => return u32::MAX,
        };

        // Copy data from emulator memory
        let mut data = vec![0u8; len];
        for i in 0..len {
            data[i] = cpu.memory().read_u8(buf_ptr + i as u32);
        }

        // Send via real socket
        let bytes_sent = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.send_to(&data, dest_addr).ok())
            .unwrap_or(0);

        bytes_sent as u32
    }

    fn name(&self) -> &str {
        "sendto"
    }
}

/// recv() - Receive data
///
/// ssize_t recv(int sockfd, void *buf, size_t len, int flags);
pub struct Recv;

impl RomStubHandler for Recv {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;
        let _flags = cpu.get_ar(5);

        // Receive from real socket
        let data = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.recv(len).ok())
            .unwrap_or_default();

        // Copy to emulator memory
        for (i, &byte) in data.iter().enumerate() {
            cpu.memory().write_u8(buf_ptr + i as u32, byte);
        }

        data.len() as u32
    }

    fn name(&self) -> &str {
        "recv"
    }
}

/// recvfrom() - Receive data from specific address
///
/// ssize_t recvfrom(int sockfd, void *buf, size_t len, int flags,
///                  struct sockaddr *src_addr, socklen_t *addrlen);
pub struct RecvFrom;

impl RomStubHandler for RecvFrom {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;
        let _flags = cpu.get_ar(5);
        let src_addr_ptr = cpu.get_ar(6);
        let addrlen_ptr = cpu.get_ar(7);

        // Receive from real socket
        let result = lock_manager!()
            .get_mut(sockfd)
            .and_then(|socket| socket.recv_from(len).ok());

        match result {
            Some((data, src_addr)) => {
                // Copy to emulator memory
                for (i, &byte) in data.iter().enumerate() {
                    cpu.memory().write_u8(buf_ptr + i as u32, byte);
                }

                // Write source address
                if src_addr_ptr != 0 {
                    write_sockaddr(cpu, src_addr_ptr, src_addr);
                }

                if addrlen_ptr != 0 {
                    cpu.memory().write_u32(addrlen_ptr, 16);
                }

                data.len() as u32
            }
            None => 0,
        }
    }

    fn name(&self) -> &str {
        "recvfrom"
    }
}

/// close() - Close socket
///
/// int close(int fd);
pub struct Close;

impl RomStubHandler for Close {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sockfd = cpu.get_ar(2);

        lock_manager!().close(sockfd);

        0
    }

    fn name(&self) -> &str {
        "close"
    }
}

/// setsockopt() - Set socket options
///
/// int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen);
pub struct SetSockOpt;

impl RomStubHandler for SetSockOpt {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success
        0
    }

    fn name(&self) -> &str {
        "setsockopt"
    }
}

/// getsockopt() - Get socket options
///
/// int getsockopt(int sockfd, int level, int optname, void *optval, socklen_t *optlen);
pub struct GetSockOpt;

impl RomStubHandler for GetSockOpt {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success
        0
    }

    fn name(&self) -> &str {
        "getsockopt"
    }
}

/// getaddrinfo() - DNS lookup
///
/// int getaddrinfo(const char *node, const char *service,
///                 const struct addrinfo *hints, struct addrinfo **res);
pub struct Getaddrinfo;

impl RomStubHandler for Getaddrinfo {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let hostname_ptr = cpu.get_ar(2);
        let servname_ptr = cpu.get_ar(3);
        let _hints_ptr = cpu.get_ar(4);
        let result_ptr = cpu.get_ar(5);

        // Read hostname from memory
        let hostname = read_string_from_memory(cpu, hostname_ptr, 256);
        if hostname.is_empty() {
            return 1; // EAI_FAIL
        }

        // Read port/service from memory (optional)
        let port: u16 = if servname_ptr != 0 {
            let service = read_string_from_memory(cpu, servname_ptr, 32);
            service.parse().unwrap_or(80)
        } else {
            80
        };

        // Perform real DNS lookup using std::net
        let addrs: Vec<SocketAddr> = match (hostname.as_str(), port).to_socket_addrs() {
            Ok(iter) => iter.collect(),
            Err(_) => {
                // Fallback: try to parse as IP address
                if let Ok(ip_addr) = hostname.parse::<std::net::Ipv4Addr>() {
                    vec![SocketAddr::from((ip_addr, port))]
                } else {
                    return 1; // EAI_FAIL
                }
            }
        };

        if addrs.is_empty() {
            return 1; // EAI_FAIL
        }

        // Allocate addrinfo in DRAM (use dynamic allocation region)
        let addrinfo_addr = 0x3FFE_1000;
        let sockaddr_addr = 0x3FFE_1100;

        // Write first result (IPv4 only)
        let first_addr = addrs.iter().find(|a| a.is_ipv4());
        if first_addr.is_none() {
            return 1; // No IPv4 address found
        }

        let first_addr = *first_addr.unwrap();

        // Write sockaddr_in
        write_sockaddr(cpu, sockaddr_addr, first_addr);

        // Write addrinfo structure
        cpu.memory().write_u32(addrinfo_addr, 0);       // ai_flags
        cpu.memory().write_u32(addrinfo_addr + 4, AF_INET);   // ai_family (AF_INET)
        cpu.memory().write_u32(addrinfo_addr + 8, SOCK_STREAM);   // ai_socktype (SOCK_STREAM)
        cpu.memory().write_u32(addrinfo_addr + 12, 0);  // ai_protocol
        cpu.memory().write_u32(addrinfo_addr + 16, 16); // ai_addrlen
        cpu.memory().write_u32(addrinfo_addr + 20, sockaddr_addr); // ai_addr
        cpu.memory().write_u32(addrinfo_addr + 24, 0);  // ai_canonname
        cpu.memory().write_u32(addrinfo_addr + 28, 0);  // ai_next

        // Write result pointer
        cpu.memory().write_u32(result_ptr, addrinfo_addr);

        0 // Success
    }

    fn name(&self) -> &str {
        "getaddrinfo"
    }
}

/// freeaddrinfo() - Free address info
///
/// void freeaddrinfo(struct addrinfo *res);
pub struct Freeaddrinfo;

impl RomStubHandler for Freeaddrinfo {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Nothing to free (we used static addresses)
        0
    }

    fn name(&self) -> &str {
        "freeaddrinfo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functions::socket_manager::reset_socket_manager;
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    fn create_test_cpu() -> XtensaCpu {
        reset_socket_manager(); // Reset before each test
        XtensaCpu::new(Arc::new(Memory::new()))
    }

    fn write_string_to_memory(cpu: &mut XtensaCpu, ptr: u32, s: &str) {
        for (i, &byte) in s.as_bytes().iter().enumerate() {
            cpu.memory().write_u8(ptr + i as u32, byte);
        }
        cpu.memory().write_u8(ptr + s.len() as u32, 0); // null terminator
    }

    #[test]
    fn test_socket_creation() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let stub = Socket;
        let fd = stub.call(&mut cpu);
        assert!(fd >= 3);  // Valid socket fd
        assert_ne!(fd, u32::MAX);  // Not an error
    }

    #[test]
    fn test_socket_creation_invalid_domain() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, 99);  // Invalid domain
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let stub = Socket;
        let fd = stub.call(&mut cpu);
        assert_eq!(fd, u32::MAX);  // Error
    }

    #[test]
    fn test_tcp_loopback_connect() {
        // Create a TCP listener
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let socket_stub = Socket;
        let listener_fd = socket_stub.call(&mut cpu);

        // Bind to loopback:0 (auto-assign port)
        let sockaddr_ptr = 0x3FFE_0000;
        cpu.memory().write_u16(sockaddr_ptr, AF_INET as u16);
        cpu.memory().write_u16(sockaddr_ptr + 2, 0);  // port 0 (auto)
        cpu.memory().write_u32(sockaddr_ptr + 4, 0x7F000001);  // 127.0.0.1

        cpu.set_ar(2, listener_fd);
        cpu.set_ar(3, sockaddr_ptr);
        cpu.set_ar(4, 16);

        let bind_stub = Bind;
        let result = bind_stub.call(&mut cpu);
        assert_eq!(result, 0);

        // Listen
        cpu.set_ar(2, listener_fd);
        cpu.set_ar(3, 5);

        let listen_stub = Listen;
        let result = listen_stub.call(&mut cpu);
        assert_eq!(result, 0);

        // Get the actual bound port
        let bound_port = {
            let manager = lock_manager!();
            let socket = manager.get(listener_fd).unwrap();
            socket.tcp_listener().unwrap().local_addr().unwrap().port()
        };

        // Create client socket and connect
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let client_fd = socket_stub.call(&mut cpu);

        // Connect to the listener
        cpu.memory().write_u16(sockaddr_ptr, AF_INET as u16);
        cpu.memory().write_u16(sockaddr_ptr + 2, bound_port.to_be());
        cpu.memory().write_u32(sockaddr_ptr + 4, 0x7F000001);

        cpu.set_ar(2, client_fd);
        cpu.set_ar(3, sockaddr_ptr);
        cpu.set_ar(4, 16);

        let connect_stub = Connect;
        let result = connect_stub.call(&mut cpu);
        assert_eq!(result, 0);  // Success
    }

    #[test]
    fn test_recv_nonblocking_returns_zero() {
        // Create and connect a socket
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let socket_stub = Socket;
        let listener_fd = socket_stub.call(&mut cpu);

        // Bind and listen
        let sockaddr_ptr = 0x3FFE_0000;
        cpu.memory().write_u16(sockaddr_ptr, AF_INET as u16);
        cpu.memory().write_u16(sockaddr_ptr + 2, 0);
        cpu.memory().write_u32(sockaddr_ptr + 4, 0x7F000001);

        cpu.set_ar(2, listener_fd);
        cpu.set_ar(3, sockaddr_ptr);
        cpu.set_ar(4, 16);
        Bind.call(&mut cpu);

        cpu.set_ar(2, listener_fd);
        cpu.set_ar(3, 5);
        Listen.call(&mut cpu);

        let bound_port = {
            let manager = lock_manager!();
            manager.get(listener_fd).unwrap()
                .tcp_listener().unwrap().local_addr().unwrap().port()
        };

        // Connect
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);
        let client_fd = socket_stub.call(&mut cpu);

        cpu.memory().write_u16(sockaddr_ptr, AF_INET as u16);
        cpu.memory().write_u16(sockaddr_ptr + 2, bound_port.to_be());
        cpu.memory().write_u32(sockaddr_ptr + 4, 0x7F000001);

        cpu.set_ar(2, client_fd);
        cpu.set_ar(3, sockaddr_ptr);
        cpu.set_ar(4, 16);
        Connect.call(&mut cpu);

        // Try to receive when no data is available
        let buf_ptr = 0x3FFE_1000;
        cpu.set_ar(2, client_fd);
        cpu.set_ar(3, buf_ptr);
        cpu.set_ar(4, 100);
        cpu.set_ar(5, 0);

        let recv_stub = Recv;
        let result = recv_stub.call(&mut cpu);
        assert_eq!(result, 0);  // No data, non-blocking
    }

    #[test]
    fn test_close() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_STREAM);
        cpu.set_ar(4, 0);

        let socket_stub = Socket;
        let fd = socket_stub.call(&mut cpu);

        // Verify socket exists
        {
            let manager = lock_manager!();
            assert!(manager.get(fd).is_some());
        }

        // Close socket
        cpu.set_ar(2, fd);
        let close_stub = Close;
        let result = close_stub.call(&mut cpu);
        assert_eq!(result, 0);

        // Verify socket is removed
        {
            let manager = lock_manager!();
            assert!(manager.get(fd).is_none());
        }
    }

    #[test]
    fn test_getaddrinfo_localhost() {
        let mut cpu = create_test_cpu();
        let hostname_ptr = 0x3FFE_0000;
        let result_ptr = 0x3FFE_2000;

        // Write "127.0.0.1" to memory
        write_string_to_memory(&mut cpu, hostname_ptr, "127.0.0.1");

        cpu.set_ar(2, hostname_ptr);
        cpu.set_ar(3, 0);  // service (null)
        cpu.set_ar(4, 0);  // hints
        cpu.set_ar(5, result_ptr);

        let stub = Getaddrinfo;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);  // Success

        // Check that result pointer was written
        let addrinfo_ptr = cpu.memory().read_u32(result_ptr);
        assert_ne!(addrinfo_ptr, 0);

        // Verify address is 127.0.0.1
        let sockaddr_ptr = cpu.memory().read_u32(addrinfo_ptr + 20);
        let ip = cpu.memory().read_u32(sockaddr_ptr + 4);
        assert_eq!(ip, 0x7F000001);  // 127.0.0.1
    }

    #[test]
    fn test_getaddrinfo_with_port() {
        let mut cpu = create_test_cpu();
        let hostname_ptr = 0x3FFE_0000;
        let service_ptr = 0x3FFE_0100;
        let result_ptr = 0x3FFE_2000;

        write_string_to_memory(&mut cpu, hostname_ptr, "127.0.0.1");
        write_string_to_memory(&mut cpu, service_ptr, "8080");

        cpu.set_ar(2, hostname_ptr);
        cpu.set_ar(3, service_ptr);
        cpu.set_ar(4, 0);
        cpu.set_ar(5, result_ptr);

        let stub = Getaddrinfo;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);

        // Verify port is 8080
        let addrinfo_ptr = cpu.memory().read_u32(result_ptr);
        let sockaddr_ptr = cpu.memory().read_u32(addrinfo_ptr + 20);
        let port = cpu.memory().read_u16(sockaddr_ptr + 2);
        let port = u16::from_be(port);
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_udp_socket_creation() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, AF_INET);
        cpu.set_ar(3, SOCK_DGRAM);
        cpu.set_ar(4, 0);

        let stub = Socket;
        let fd = stub.call(&mut cpu);
        assert!(fd >= 3);

        // Verify it's a UDP socket
        {
            let manager = lock_manager!();
            let socket = manager.get(fd).unwrap();
            assert_eq!(socket.socket_type(), SocketType::Udp);
        }
    }
}
