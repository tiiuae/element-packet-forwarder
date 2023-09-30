///TODO: winapi functions should be added for windows support
use nix::net::if_::*;

use std::error::Error;
use std::ffi::CString;
use std::net::{IpAddr, Ipv6Addr, SocketAddrV6, UdpSocket};

///ff02::114
const PINECONE_UDP_MCAST_ADDR: Ipv6Addr =
    Ipv6Addr::new(0xff02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0114);
const PINECONE_UDP_MCAST_PORT: u16 = 60606;

pub fn udp_ipv6_init(interface_name: &str) -> std::net::UdpSocket {
    // let ipv6_addr = Ipv6Addr::from_str(ipv6_str).expect("Failed to parse IPv6 address");
    let ipv6_addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    // Convert the interface name to  a CStr
    let ifname =
        CString::new(interface_name.as_bytes()).expect("Failed to create CStr from interface name");

    // Get the interface index (scope ID) using if_nametoindex
    let ifindex = if_nametoindex(ifname.as_c_str()).unwrap_or_else(|err| {
        panic!("Error getting interface index: {}", err);
    });

    // Create a SocketAddrV6 variable by specifying the address and port
    let socket_addr_v6 = SocketAddrV6::new(ipv6_addr, PINECONE_UDP_MCAST_PORT, 0, ifindex);

    println!("SocketAddrV6: {}", socket_addr_v6);

    get_udpsock_with_mcastv6_opts(
        &socket_addr_v6,
        &PINECONE_UDP_MCAST_ADDR,
        ifindex,
        interface_name,
    )
    .expect("Failed to set multicast options")
}

fn get_udpsock_with_mcastv6_opts(
    addr: &SocketAddrV6,
    multiaddr: &Ipv6Addr,
    if_index: u32,
    if_name: &str,
) -> Result<std::net::UdpSocket, Box<dyn Error>> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};

    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true).expect("set_reuse_Addr err");
    socket.bind(&SockAddr::from(*addr)).expect("bind error");
    socket
        .bind_device(Some(if_name.as_bytes()))
        .expect("bind device err");
    // socket.set_only_v6(true).unwrap_or_else(|e|{println!("error : {}",e)});
    socket
        .join_multicast_v6(multiaddr, if_index)
        .expect("join multicast v6 error");
    socket
        .set_multicast_if_v6(if_index)
        .expect("set multicast interface v6");

    socket
        .set_multicast_loop_v6(false)
        .expect("set mcast loop v6 err");

    socket.set_nonblocking(true).expect("set nonblocking err");

    Ok(socket.into())
}
