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
