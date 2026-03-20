/// Integration tests for real network I/O via host socket bridging
///
/// These tests demonstrate real TCP/UDP communication through the emulator.

use flexers_core::cpu::XtensaCpu;
use flexers_core::memory::Memory;
use flexers_stubs::functions::network::*;
use flexers_stubs::functions::socket_manager::reset_socket_manager;
use flexers_stubs::handler::RomStubHandler;
use std::sync::Arc;

const AF_INET: u32 = 2;
const SOCK_STREAM: u32 = 1;
const SOCK_DGRAM: u32 = 2;

fn create_test_cpu() -> XtensaCpu {
    reset_socket_manager();
    XtensaCpu::new(Arc::new(Memory::new()))
}

fn write_string_to_memory(cpu: &mut XtensaCpu, ptr: u32, s: &str) {
    for (i, &byte) in s.as_bytes().iter().enumerate() {
        cpu.memory().write_u8(ptr + i as u32, byte);
    }
    cpu.memory().write_u8(ptr + s.len() as u32, 0); // null terminator
}

fn write_sockaddr(cpu: &mut XtensaCpu, addr_ptr: u32, ip: [u8; 4], port: u16) {
    cpu.memory().write_u16(addr_ptr, AF_INET as u16);
    cpu.memory().write_u16(addr_ptr + 2, port.to_be());
    let ip_u32 = ((ip[0] as u32) << 24)
        | ((ip[1] as u32) << 16)
        | ((ip[2] as u32) << 8)
        | (ip[3] as u32);
    cpu.memory().write_u32(addr_ptr + 4, ip_u32);
}

#[test]
fn test_tcp_echo_loopback() {
    // Create a listener
    let mut cpu = create_test_cpu();

    // Create socket
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let listener_fd = Socket.call(&mut cpu);

    // Bind to loopback:0
    let sockaddr_ptr = 0x3FFE_0000;
    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], 0);
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    assert_eq!(Bind.call(&mut cpu), 0);

    // Listen
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 5);
    assert_eq!(Listen.call(&mut cpu), 0);

    // Get bound port
    let bound_port = {
        use flexers_stubs::functions::socket_manager::SOCKET_MANAGER;
        let manager = SOCKET_MANAGER.lock().unwrap();
        manager
            .get(listener_fd)
            .unwrap()
            .tcp_listener()
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    };

    // Create client socket
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let client_fd = Socket.call(&mut cpu);

    // Connect to listener
    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], bound_port);
    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    assert_eq!(Connect.call(&mut cpu), 0);

    // Accept connection (non-blocking, may need retry)
    let mut server_fd = u32::MAX;
    for _ in 0..10 {
        cpu.set_ar(2, listener_fd);
        cpu.set_ar(3, 0);
        cpu.set_ar(4, 0);
        server_fd = Accept.call(&mut cpu);
        if server_fd != u32::MAX {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_ne!(server_fd, u32::MAX, "Accept failed");

    // Send data from client
    let test_data = b"Hello, World!";
    let buf_ptr = 0x3FFE_1000;
    for (i, &byte) in test_data.iter().enumerate() {
        cpu.memory().write_u8(buf_ptr + i as u32, byte);
    }

    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, buf_ptr);
    cpu.set_ar(4, test_data.len() as u32);
    cpu.set_ar(5, 0);
    let sent = Send.call(&mut cpu);
    assert_eq!(sent, test_data.len() as u32);

    // Give time for data to arrive
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Receive on server side
    let recv_buf_ptr = 0x3FFE_2000;
    cpu.set_ar(2, server_fd);
    cpu.set_ar(3, recv_buf_ptr);
    cpu.set_ar(4, 1024);
    cpu.set_ar(5, 0);
    let received = Recv.call(&mut cpu);
    assert_eq!(received, test_data.len() as u32);

    // Verify data
    for (i, &expected_byte) in test_data.iter().enumerate() {
        let actual_byte = cpu.memory().read_u8(recv_buf_ptr + i as u32);
        assert_eq!(actual_byte, expected_byte);
    }

    // Clean up
    cpu.set_ar(2, client_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, server_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, listener_fd);
    Close.call(&mut cpu);
}

#[test]
fn test_dns_resolution() {
    let mut cpu = create_test_cpu();

    let hostname_ptr = 0x3FFE_0000;
    let service_ptr = 0x3FFE_0100;
    let result_ptr = 0x3FFE_2000;

    // Test resolving localhost
    write_string_to_memory(&mut cpu, hostname_ptr, "localhost");
    write_string_to_memory(&mut cpu, service_ptr, "80");

    cpu.set_ar(2, hostname_ptr);
    cpu.set_ar(3, service_ptr);
    cpu.set_ar(4, 0);
    cpu.set_ar(5, result_ptr);

    let result = Getaddrinfo.call(&mut cpu);
    assert_eq!(result, 0); // Success

    // Verify result pointer was written
    let addrinfo_ptr = cpu.memory().read_u32(result_ptr);
    assert_ne!(addrinfo_ptr, 0);

    // Verify port is 80
    let sockaddr_ptr = cpu.memory().read_u32(addrinfo_ptr + 20);
    let port = cpu.memory().read_u16(sockaddr_ptr + 2);
    let port = u16::from_be(port);
    assert_eq!(port, 80);
}

#[test]
fn test_udp_socket() {
    let mut cpu = create_test_cpu();

    // Create UDP socket
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_DGRAM);
    cpu.set_ar(4, 0);
    let fd = Socket.call(&mut cpu);
    assert!(fd >= 3);

    // Bind to loopback:0
    let sockaddr_ptr = 0x3FFE_0000;
    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], 0);
    cpu.set_ar(2, fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    assert_eq!(Bind.call(&mut cpu), 0);

    // Close
    cpu.set_ar(2, fd);
    assert_eq!(Close.call(&mut cpu), 0);
}

// ============================================================================
// Phase 10B Tests: Socket Multiplexing, Options, and IPv6
// ============================================================================

#[test]
fn test_select_single_socket() {
    let mut cpu = create_test_cpu();

    // Create and connect a socket
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let listener_fd = Socket.call(&mut cpu);

    let sockaddr_ptr = 0x3FFE_0000;
    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], 0);
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    Bind.call(&mut cpu);

    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 5);
    Listen.call(&mut cpu);

    let bound_port = {
        use flexers_stubs::functions::socket_manager::SOCKET_MANAGER;
        let manager = SOCKET_MANAGER.lock().unwrap();
        manager.get(listener_fd).unwrap().tcp_listener().unwrap().local_addr().unwrap().port()
    };

    // Create client
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let client_fd = Socket.call(&mut cpu);

    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], bound_port);
    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    Connect.call(&mut cpu);

    // Accept
    std::thread::sleep(std::time::Duration::from_millis(10));
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 0);
    cpu.set_ar(4, 0);
    let server_fd = Accept.call(&mut cpu);
    assert_ne!(server_fd, u32::MAX);

    // Send data to make socket readable
    let test_data = b"test";
    let buf_ptr = 0x3FFE_1000;
    for (i, &byte) in test_data.iter().enumerate() {
        cpu.memory().write_u8(buf_ptr + i as u32, byte);
    }
    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, buf_ptr);
    cpu.set_ar(4, test_data.len() as u32);
    cpu.set_ar(5, 0);
    Send.call(&mut cpu);

    std::thread::sleep(std::time::Duration::from_millis(10));

    // Setup fd_set for select (only server_fd)
    let readfds_ptr = 0x3FFE_2000;
    // Clear the fd_set
    for i in 0..128 {
        cpu.memory().write_u8(readfds_ptr + i, 0);
    }
    // Set bit for server_fd
    let byte_idx = server_fd / 8;
    let bit_idx = server_fd % 8;
    cpu.memory().write_u8(readfds_ptr + byte_idx, 1 << bit_idx);

    // Call select
    cpu.set_ar(2, server_fd + 1); // nfds
    cpu.set_ar(3, readfds_ptr);   // readfds
    cpu.set_ar(4, 0);              // writefds
    cpu.set_ar(5, 0);              // exceptfds
    cpu.set_ar(6, 0);              // timeout (null = block)

    let ready = Select.call(&mut cpu);
    assert_eq!(ready, 1, "Should have 1 socket ready for reading");

    // Verify the bit is still set
    let byte_val = cpu.memory().read_u8(readfds_ptr + byte_idx);
    assert_ne!(byte_val & (1 << bit_idx), 0, "server_fd should be ready");

    // Cleanup
    cpu.set_ar(2, client_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, server_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, listener_fd);
    Close.call(&mut cpu);
}

#[test]
fn test_select_no_ready_sockets() {
    let mut cpu = create_test_cpu();

    // Create socket but don't send data
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let listener_fd = Socket.call(&mut cpu);

    let sockaddr_ptr = 0x3FFE_0000;
    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], 0);
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    Bind.call(&mut cpu);

    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 5);
    Listen.call(&mut cpu);

    let bound_port = {
        use flexers_stubs::functions::socket_manager::SOCKET_MANAGER;
        let manager = SOCKET_MANAGER.lock().unwrap();
        manager.get(listener_fd).unwrap().tcp_listener().unwrap().local_addr().unwrap().port()
    };

    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let client_fd = Socket.call(&mut cpu);

    write_sockaddr(&mut cpu, sockaddr_ptr, [127, 0, 0, 1], bound_port);
    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 16);
    Connect.call(&mut cpu);

    std::thread::sleep(std::time::Duration::from_millis(10));

    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 0);
    cpu.set_ar(4, 0);
    let server_fd = Accept.call(&mut cpu);
    assert_ne!(server_fd, u32::MAX);

    // Setup fd_set (no data sent, so not readable)
    let readfds_ptr = 0x3FFE_2000;
    for i in 0..128 {
        cpu.memory().write_u8(readfds_ptr + i, 0);
    }
    let byte_idx = server_fd / 8;
    let bit_idx = server_fd % 8;
    cpu.memory().write_u8(readfds_ptr + byte_idx, 1 << bit_idx);

    // Call select
    cpu.set_ar(2, server_fd + 1);
    cpu.set_ar(3, readfds_ptr);
    cpu.set_ar(4, 0);
    cpu.set_ar(5, 0);
    cpu.set_ar(6, 0);

    let ready = Select.call(&mut cpu);
    assert_eq!(ready, 0, "Should have 0 sockets ready");

    // Verify bit was cleared
    let byte_val = cpu.memory().read_u8(readfds_ptr + byte_idx);
    assert_eq!(byte_val & (1 << bit_idx), 0, "Bit should be cleared");

    // Cleanup
    cpu.set_ar(2, client_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, server_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, listener_fd);
    Close.call(&mut cpu);
}

#[test]
fn test_setsockopt_tcp_nodelay() {
    let mut cpu = create_test_cpu();

    // Create TCP socket
    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let fd = Socket.call(&mut cpu);

    // Set TCP_NODELAY option
    const IPPROTO_TCP: i32 = 6;
    const TCP_NODELAY: i32 = 1;

    let optval_ptr = 0x3FFE_0000;
    cpu.memory().write_u32(optval_ptr, 1); // Enable

    cpu.set_ar(2, fd);
    cpu.set_ar(3, IPPROTO_TCP as u32);
    cpu.set_ar(4, TCP_NODELAY as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, 4); // sizeof(int)

    let result = SetSockOpt.call(&mut cpu);
    assert_eq!(result, 0, "setsockopt should succeed");

    // Get the option back
    let optlen_ptr = 0x3FFE_0100;
    cpu.memory().write_u32(optlen_ptr, 4);

    cpu.set_ar(2, fd);
    cpu.set_ar(3, IPPROTO_TCP as u32);
    cpu.set_ar(4, TCP_NODELAY as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, optlen_ptr);

    let result = GetSockOpt.call(&mut cpu);
    assert_eq!(result, 0, "getsockopt should succeed");

    let optval = cpu.memory().read_u32(optval_ptr);
    assert_eq!(optval & 0xFF, 1, "TCP_NODELAY should be enabled");

    // Cleanup
    cpu.set_ar(2, fd);
    Close.call(&mut cpu);
}

#[test]
fn test_setsockopt_rcvbuf() {
    let mut cpu = create_test_cpu();

    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let fd = Socket.call(&mut cpu);

    const SOL_SOCKET: i32 = 1;
    const SO_RCVBUF: i32 = 8;

    let optval_ptr = 0x3FFE_0000;
    cpu.memory().write_u32(optval_ptr, 16384); // 16KB

    cpu.set_ar(2, fd);
    cpu.set_ar(3, SOL_SOCKET as u32);
    cpu.set_ar(4, SO_RCVBUF as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, 4);

    let result = SetSockOpt.call(&mut cpu);
    assert_eq!(result, 0);

    // Get it back
    let optlen_ptr = 0x3FFE_0100;
    cpu.memory().write_u32(optlen_ptr, 4);

    cpu.set_ar(2, fd);
    cpu.set_ar(3, SOL_SOCKET as u32);
    cpu.set_ar(4, SO_RCVBUF as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, optlen_ptr);

    GetSockOpt.call(&mut cpu);
    let rcvbuf = cpu.memory().read_u32(optval_ptr);
    assert_eq!(rcvbuf, 16384);

    cpu.set_ar(2, fd);
    Close.call(&mut cpu);
}

#[test]
fn test_setsockopt_rcvtimeo() {
    let mut cpu = create_test_cpu();

    cpu.set_ar(2, AF_INET);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let fd = Socket.call(&mut cpu);

    const SOL_SOCKET: i32 = 1;
    const SO_RCVTIMEO: i32 = 20;

    // Set timeout to 2 seconds
    let optval_ptr = 0x3FFE_0000;
    cpu.memory().write_u32(optval_ptr, 2);      // tv_sec = 2
    cpu.memory().write_u32(optval_ptr + 4, 0);  // tv_usec = 0

    cpu.set_ar(2, fd);
    cpu.set_ar(3, SOL_SOCKET as u32);
    cpu.set_ar(4, SO_RCVTIMEO as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, 8); // sizeof(struct timeval)

    let result = SetSockOpt.call(&mut cpu);
    assert_eq!(result, 0);

    // Get it back
    let optlen_ptr = 0x3FFE_0100;
    cpu.memory().write_u32(optlen_ptr, 8);

    cpu.set_ar(2, fd);
    cpu.set_ar(3, SOL_SOCKET as u32);
    cpu.set_ar(4, SO_RCVTIMEO as u32);
    cpu.set_ar(5, optval_ptr);
    cpu.set_ar(6, optlen_ptr);

    GetSockOpt.call(&mut cpu);
    let tv_sec = cpu.memory().read_u32(optval_ptr);
    assert_eq!(tv_sec, 2);

    cpu.set_ar(2, fd);
    Close.call(&mut cpu);
}

const AF_INET6: u32 = 10;

#[test]
fn test_ipv6_socket_creation() {
    let mut cpu = create_test_cpu();

    // Create IPv6 socket
    cpu.set_ar(2, AF_INET6);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let fd = Socket.call(&mut cpu);

    assert!(fd >= 3, "IPv6 socket creation should succeed");
    assert_ne!(fd, u32::MAX);

    // Verify socket was created
    {
        use flexers_stubs::functions::socket_manager::SOCKET_MANAGER;
        let manager = SOCKET_MANAGER.lock().unwrap();
        assert!(manager.get(fd).is_some());
    }

    // Cleanup
    cpu.set_ar(2, fd);
    Close.call(&mut cpu);
}

#[test]
fn test_ipv6_loopback_connect() {
    let mut cpu = create_test_cpu();

    // Create IPv6 listener
    cpu.set_ar(2, AF_INET6);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let listener_fd = Socket.call(&mut cpu);
    assert_ne!(listener_fd, u32::MAX);

    // Bind to IPv6 loopback ::1
    let sockaddr_ptr = 0x3FFE_0000;
    cpu.memory().write_u16(sockaddr_ptr, AF_INET6 as u16); // family
    cpu.memory().write_u16(sockaddr_ptr + 2, 0);           // port 0 (auto)
    cpu.memory().write_u32(sockaddr_ptr + 4, 0);           // flowinfo

    // Write ::1 (all zeros except last byte = 1)
    for i in 0..15 {
        cpu.memory().write_u8(sockaddr_ptr + 8 + i, 0);
    }
    cpu.memory().write_u8(sockaddr_ptr + 8 + 15, 1);  // ::1
    cpu.memory().write_u32(sockaddr_ptr + 24, 0);     // scope_id

    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 28); // sizeof(sockaddr_in6)
    let bind_result = Bind.call(&mut cpu);
    assert_eq!(bind_result, 0, "IPv6 bind should succeed");

    // Listen
    cpu.set_ar(2, listener_fd);
    cpu.set_ar(3, 5);
    let listen_result = Listen.call(&mut cpu);
    assert_eq!(listen_result, 0);

    // Get the bound port
    let bound_port = {
        use flexers_stubs::functions::socket_manager::SOCKET_MANAGER;
        let manager = SOCKET_MANAGER.lock().unwrap();
        manager.get(listener_fd).unwrap().tcp_listener().unwrap().local_addr().unwrap().port()
    };

    // Create IPv6 client
    cpu.set_ar(2, AF_INET6);
    cpu.set_ar(3, SOCK_STREAM);
    cpu.set_ar(4, 0);
    let client_fd = Socket.call(&mut cpu);

    // Connect to ::1
    cpu.memory().write_u16(sockaddr_ptr + 2, bound_port.to_be());
    cpu.set_ar(2, client_fd);
    cpu.set_ar(3, sockaddr_ptr);
    cpu.set_ar(4, 28);
    let connect_result = Connect.call(&mut cpu);
    assert_eq!(connect_result, 0, "IPv6 connect should succeed");

    // Cleanup
    cpu.set_ar(2, client_fd);
    Close.call(&mut cpu);
    cpu.set_ar(2, listener_fd);
    Close.call(&mut cpu);
}
