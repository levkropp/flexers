/// Network (socket) function stubs
///
/// These provide minimal socket API stubs for testing network code without real hardware.

use crate::handler::RomStubHandler;
use flexers_core::cpu::XtensaCpu;

/// Socket constants
const AF_INET: u32 = 2;
const SOCK_STREAM: u32 = 1;
const SOCK_DGRAM: u32 = 2;

/// Fake socket file descriptor (starts at 3 to avoid stdin/stdout/stderr)
static mut NEXT_SOCKET_FD: u32 = 3;

/// socket() - Create socket
///
/// int socket(int domain, int type, int protocol);
pub struct Socket;

impl RomStubHandler for Socket {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _domain = cpu.get_ar(2);   // AF_INET
        let _sock_type = cpu.get_ar(3); // SOCK_STREAM/SOCK_DGRAM
        let _protocol = cpu.get_ar(4);  // 0

        // Return fake socket fd
        unsafe {
            let fd = NEXT_SOCKET_FD;
            NEXT_SOCKET_FD += 1;
            fd
        }
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success
        0
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success (immediate connection)
        0
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success
        0
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return fake client socket fd
        unsafe {
            let fd = NEXT_SOCKET_FD;
            NEXT_SOCKET_FD += 1;
            fd
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
        let _sockfd = cpu.get_ar(2);
        let _buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4);
        let _flags = cpu.get_ar(5);

        // Return bytes sent (simulate success)
        len
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
        let _sockfd = cpu.get_ar(2);
        let _buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4);
        let _flags = cpu.get_ar(5);
        let _dest_addr = cpu.get_ar(6);
        let _addrlen = cpu.get_ar(7);

        // Return bytes sent (simulate success)
        len
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return 0 (no data available)
        // Real implementation could use host sockets
        0
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return 0 (no data available)
        0
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
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Return success
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
        let _hostname = cpu.get_ar(2);
        let _servname = cpu.get_ar(3);
        let _hints = cpu.get_ar(4);
        let result_ptr = cpu.get_ar(5);

        // Allocate fake addrinfo structure
        // For simplicity, we'll use a fixed address in DRAM
        let addrinfo_addr = 0x3FFE_1000;

        // Write addrinfo structure (simplified)
        // struct addrinfo {
        //     int ai_flags;
        //     int ai_family;
        //     int ai_socktype;
        //     int ai_protocol;
        //     socklen_t ai_addrlen;
        //     struct sockaddr *ai_addr;
        //     char *ai_canonname;
        //     struct addrinfo *ai_next;
        // };

        cpu.memory().write_u32(addrinfo_addr, 0);          // ai_flags
        cpu.memory().write_u32(addrinfo_addr + 4, AF_INET); // ai_family
        cpu.memory().write_u32(addrinfo_addr + 8, SOCK_STREAM); // ai_socktype
        cpu.memory().write_u32(addrinfo_addr + 12, 0);     // ai_protocol
        cpu.memory().write_u32(addrinfo_addr + 16, 16);    // ai_addrlen
        cpu.memory().write_u32(addrinfo_addr + 20, 0x3FFE_1100); // ai_addr (fake sockaddr)
        cpu.memory().write_u32(addrinfo_addr + 24, 0);     // ai_canonname
        cpu.memory().write_u32(addrinfo_addr + 28, 0);     // ai_next

        // Write fake sockaddr (127.0.0.1:80)
        let sockaddr = 0x3FFE_1100;
        cpu.memory().write_u16(sockaddr, AF_INET as u16);  // sa_family
        cpu.memory().write_u16(sockaddr + 2, 80);          // port (big-endian)
        cpu.memory().write_u32(sockaddr + 4, 0x7F000001);  // 127.0.0.1

        // Write result pointer
        if result_ptr != 0 {
            cpu.memory().write_u32(result_ptr, addrinfo_addr);
        }

        0  // Success
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
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    fn create_test_cpu() -> XtensaCpu {
        XtensaCpu::new(Arc::new(Memory::new()))
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
    }

    #[test]
    fn test_connect() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, 3);  // socket fd
        cpu.set_ar(3, 0x3FFE_0000);  // sockaddr ptr
        cpu.set_ar(4, 16);  // addrlen

        let stub = Connect;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);  // Success
    }

    #[test]
    fn test_send() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, 3);  // socket fd
        cpu.set_ar(3, 0x3FFE_0000);  // buffer ptr
        cpu.set_ar(4, 100);  // length
        cpu.set_ar(5, 0);  // flags

        let stub = Send;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 100);  // Bytes sent
    }

    #[test]
    fn test_recv() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, 3);  // socket fd
        cpu.set_ar(3, 0x3FFE_0000);  // buffer ptr
        cpu.set_ar(4, 100);  // length
        cpu.set_ar(5, 0);  // flags

        let stub = Recv;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);  // No data
    }

    #[test]
    fn test_close() {
        let mut cpu = create_test_cpu();
        cpu.set_ar(2, 3);  // socket fd

        let stub = Close;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);  // Success
    }

    #[test]
    fn test_getaddrinfo() {
        let mut cpu = create_test_cpu();
        let result_ptr = 0x3FFE_2000;

        cpu.set_ar(2, 0x3FFE_0000);  // hostname ptr
        cpu.set_ar(3, 0);  // service
        cpu.set_ar(4, 0);  // hints
        cpu.set_ar(5, result_ptr);

        let stub = Getaddrinfo;
        let result = stub.call(&mut cpu);
        assert_eq!(result, 0);  // Success

        // Check that result pointer was written
        let addrinfo_ptr = cpu.memory().read_u32(result_ptr);
        assert_ne!(addrinfo_ptr, 0);
    }
}
