use crate::cli;
use crate::fwd_tcp;
use crate::log_payload;
use crate::shared_state::*;
///TODO: winapi functions should be added for windows support
use nix::net::if_::*;
use std::error::Error;
use std::ffi::CString;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::task::yield_now;
use tokio::time::{sleep, Duration};
///ff02::114
const PINECONE_UDP_MCAST_ADDR: Ipv6Addr =
    Ipv6Addr::new(0xff02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0114);
const PINECONE_UDP_MCAST_PORT: u16 = 60606;
const PINECONE_UDP_MCAST_ADDR_PORT_STR: &str = "ff02::114:60606";

async fn udp_ipv6_init(interface_name: &str) -> std::net::UdpSocket {
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

    tracing::trace!("SocketAddrV6: {}", socket_addr_v6);

    get_udpsock_with_mcastv6_opts(
        &socket_addr_v6,
        &PINECONE_UDP_MCAST_ADDR,
        ifindex,
        interface_name,
    )
    .await
    .expect("Failed to set multicast options")
}

async fn get_udpsock_with_mcastv6_opts(
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

/// It starts pinecone udp multicast functionality
///
/// # Arguments
///
/// * `shared_state` - shared data instance between tasks
///
/// ## Usage
///
/// ```no_run
/// use element_packet_forwarder::fwd_udp;
/// use element_packet_forwarder::shared_state::*;
/// use element_packet_forwarder::start_proxy;
/// use element_packet_forwarder::start_tracing_engine;
/// use futures::join;
/// use std::error::Error;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let shared_state = SharedState::new().await;
///
///    let _pinecone_res = join!(
///         fwd_udp::start_pinecone_udp_mcast(shared_state.clone())
///     );
///   Ok(())
///
/// }
/// ```
pub async fn start_pinecone_udp_mcast(
    shared_state: SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    let udp_pinecone_mcast_sock = create_pinecone_udp_sock(NwId::One).await;
    let udp_pinecone_mcast_sock_recv = Arc::new(udp_pinecone_mcast_sock);
    let udp_pinecone_mcast_sock_send = udp_pinecone_mcast_sock_recv.clone();

    let state = shared_state.clone();
    let udp_pinecone_mcast_nw_one_send_handle = tokio::spawn(async move {
        udp_pinecone_send_nw_one(udp_pinecone_mcast_sock_send, state).await;
    });

    /* let state=shared_state.clone();
    //receiver task for network one
    let udp_pinecone_mcast_nw_one_recv_handle = tokio::spawn(async move {
        udp_pinecone_receive_nw_one(udp_pinecone_mcast_sock_recv,state).await;
    });*/

    let udp_pinecone_mcast_sock = create_pinecone_udp_sock(NwId::Two).await;
    let udp_pinecone_mcast_sock_recv = Arc::new(udp_pinecone_mcast_sock);
    let udp_pinecone_mcast_sock_send = udp_pinecone_mcast_sock_recv.clone();

    /*let state=shared_state.clone();
    //sender task for network two
    let udp_pinecone_mcast_nw_two_send_handle = tokio::spawn(async move {
        udp_pinecone_send_nw_two(udp_pinecone_mcast_sock_send,state).await;
    });*/

    let state = shared_state.clone();
    //receiver task for network two
    let udp_pinecone_mcast_nw_two_recv_handle = tokio::spawn(async move {
        udp_pinecone_receive_nw_two(udp_pinecone_mcast_sock_recv, state).await;
    });

    /*udp_pinecone_mcast_nw_one_recv_handle
    .await
    .expect("udp pinecone receive from network one mcast function error");*/

    udp_pinecone_mcast_nw_two_recv_handle
        .await
        .expect("udp pinecone receive from network two mcast function error");
    udp_pinecone_mcast_nw_one_send_handle
        .await
        .expect("udp pinecone send to network one mcast function error");
    /* udp_pinecone_mcast_nw_two_send_handle
    .await
    .expect("udp pinecone send to network two mcast function error");*/

    Ok(())
}

async fn create_pinecone_udp_sock(nw_id: NwId) -> tokio::net::UdpSocket {
    let std_udp_sock: std::net::UdpSocket = if nw_id == NwId::One {
        udp_ipv6_init(cli::get_if1_name().unwrap()).await
    } else {
        udp_ipv6_init(cli::get_if2_name().unwrap()).await
    };

    UdpSocket::from_std(std_udp_sock).expect("from std err")
}

/// Send bytes to Udp Socket from  nw two
async fn udp_pinecone_send_nw_one(tx_socket: Arc<tokio::net::UdpSocket>, state: SharedState) {
    let sockaddr: SocketAddr =
        SocketAddr::new(IpAddr::V6(PINECONE_UDP_MCAST_ADDR), PINECONE_UDP_MCAST_PORT);

    loop {
        let data = state.get_udp_incoming_pinecone_data(1).await;

        if data.is_some() {
            let log_data = data.clone();
            let _ = tx_socket.send_to(&data.unwrap(), sockaddr).await;
            state.udp_pinecone_reset_tick(1).await;
            log_payload(
                "[1]Udp data is sent payload:\n".to_string().as_str(),
                &log_data.unwrap(),
            )
            .await;
        } else {
            state.udp_pinecone_feed_tick(1).await;
        }

        sleep(Duration::from_millis(1000)).await;
    }
}

/// Receive bytes from Udp Socket from nw two
async fn udp_pinecone_receive_nw_two(rx_socket: Arc<tokio::net::UdpSocket>, state: SharedState) {
    let mut port_num = state.get_tcp_src_port_nw_one(NwId::Two).await;
    loop {
        let mut buf = vec![0; 96];

        match rx_socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                let data = buf[..size].to_vec();
                log_payload(
                    &format!(
                        "[2]Udp data received from {}, size{},payload:\n",
                        peer.ip(),
                        size
                    ),
                    &data,
                )
                .await;
                state.insert_udp_incoming_pinecone_data(1, data).await;

                let curr_port_num = state.get_tcp_src_port_nw_one(NwId::Two).await;
                if port_num != curr_port_num {
                    port_num = curr_port_num;
                    let state = state.clone();

                    tokio::spawn(async move {
                        fwd_tcp::start_tcp_pinecone_server(NwId::One, NwId::Two, state).await;
                    });
                }
            }
            Err(e) => {
                tracing::error!("Error receiving data: {:?}", e);
            }
        }
    }
}
