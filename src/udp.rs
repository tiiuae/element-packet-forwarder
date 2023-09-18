
///TODO: winapi functions should be added for windows support
use nix::net::if_::*;


use std::net::{SocketAddrV6, Ipv6Addr,UdpSocket,IpAddr};
use std::str::FromStr;
use std::ffi::CString;
use std::error::Error;

use crate::udp;

///ff02::114 
const PINECONE_UDP_MCAST_ADDR:Ipv6Addr=Ipv6Addr::new(0xff02,0x00,0x00,0x00,0x00,0x00,0x00,0x0114);  
const PINECONE_UDP_MCAST_PORT:u16=60606;





pub fn udp_ipv6_init(interface_name:&str,ipv6_str:&str)->std::net::UdpSocket{

    let ipv6_addr = Ipv6Addr::from_str(ipv6_str).expect("Failed to parse IPv6 address");


    // Convert the interface name to  a CStr
    let ifname =  CString::new(interface_name.as_bytes()).expect("Failed to create CStr from interface name");

    // Get the interface index (scope ID) using if_nametoindex
    let ifindex=if_nametoindex(ifname.as_c_str()).unwrap_or_else(|err| {
        panic!("Error getting interface index: {}", err);
    });    
    
    // Create a SocketAddrV6 variable by specifying the address and port
    let socket_addr_v6 = SocketAddrV6::new(ipv6_addr, PINECONE_UDP_MCAST_PORT, 0, ifindex);
   
    println!("SocketAddrV6: {}", socket_addr_v6);

   
  get_udpsock_with_mcastv6_opts(&socket_addr_v6,&PINECONE_UDP_MCAST_ADDR,ifindex).expect("Failed to set multicast options")

   
   

}

fn get_udpsock_with_mcastv6_opts(addr:&SocketAddrV6,multiaddr:&Ipv6Addr,if_index:u32)->Result<std::net::UdpSocket, Box<dyn Error>> {

    use socket2::{Domain, Type, Protocol, Socket,SockAddr};

    let socket = Socket::new(
        Domain::IPV6,
        Type::DGRAM,
        Some(Protocol::UDP),
    )?;

    socket.set_reuse_address(true)?;
    socket.bind(&SockAddr::from(*addr))?;

    socket.set_multicast_loop_v6(false)?;
    socket.set_only_v6(true)?;
    socket.join_multicast_v6(multiaddr, if_index)?;
    socket.set_nonblocking(true)?;
    Ok(socket.into())

}

