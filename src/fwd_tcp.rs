/*
    Copyright 2022-2023 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::cli;
use crate::log_payload;
use crate::shared_state::PortIpPort;
use crate::shared_state::SharedState;
use crate::NwId;
///TODO: winapi functions should be added for windows support
use nix::net::if_::*;
use std::error::Error;
use std::ffi::CString;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::io::{self, ReadHalf, WriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::watch::Receiver;
use tokio::task;
use tokio::task::yield_now;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
const MAX_CLIENT_NUM: i32 = 128;
const TCP_PAYLOAD_SIZE: usize = 65535;
fn tcp_sock_ipv6_init(
    interface_name: &str,
    port_num: u16,
    max_client_num: i32,
) -> Result<std::net::TcpListener, Box<dyn Error>> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};

    // let ipv6_addr = Ipv6Addr::from_str(ipv6_str).expect("Failed to parse IPv6 address");
    let ipv6_addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    // Convert the interface name to  a CStr
    let ifname =
        CString::new(interface_name.as_bytes()).expect("Failed to create CStr from interface name");

    // Get the interface index (scope ID) using if_nametoindex
    let ifindex = if_nametoindex(ifname.as_c_str()).unwrap_or_else(|err| {
        panic!("Error getting {:?} interface index: {}", ifname, err);
    });

    // Create a SocketAddrV6 variable by specifying the address and port
    let socket_addr_v6 = SocketAddrV6::new(ipv6_addr, port_num, 0, ifindex);

    tracing::trace!("SocketAddrV6: {}", socket_addr_v6);

    let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))
        .expect("tcp socket ipv6 creation err");

    // Set socket options
    socket
        .set_reuse_address(true)
        .expect("tcp ipv6 set reuse addr err");
    socket
        .bind(&SockAddr::from(socket_addr_v6))
        .expect("tcp ipv6 bind err");
    socket
        .bind_device(Some(interface_name.as_bytes()))
        .expect("tcp ipv6 bind device err");
    socket
        .set_nonblocking(true)
        .expect("tcp ipv6 set nonblocking err");

    // Configure TCP-specific options if needed
    socket.set_nodelay(true).expect("tcp socket nodelay error");

    // Listen for incoming connections
    socket
        .listen(max_client_num)
        .expect("tcp max client size set err"); // Set the backlog queue size
    Ok(socket.into())
}

fn create_pinecone_tcp_sock(nw_id: NwId, port_num: u16) -> tokio::net::TcpListener {
    let std_tcp_sock = if nw_id == NwId::One {
        tcp_sock_ipv6_init(cli::get_if1_name().unwrap(), port_num, MAX_CLIENT_NUM).unwrap()
    } else {
        tcp_sock_ipv6_init(cli::get_if2_name().unwrap(), port_num, MAX_CLIENT_NUM).unwrap()
    };

    TcpListener::from_std(std_tcp_sock).expect("tcp pinecone from std err")
}

pub async fn start_tcp_pinecone_server(recv_nw_id: NwId, fwd_nw_id: NwId, state: SharedState) {
    tracing::info!("Pinecone server is starting nw id : {}", recv_nw_id as u16,);
    let tcp_port_num = state.get_tcp_src_port_nw_one(recv_nw_id).await;
    let sock = create_pinecone_tcp_sock(recv_nw_id, tcp_port_num);

    tracing::info!(
        "Pinecone server process is starting recv nw id : {},forwarded nw id:{},port num:{}",
        recv_nw_id as u16,
        fwd_nw_id as u16,
        tcp_port_num
    );

    loop {
        match sock.accept().await {
            Ok((socket, addr)) => {
                let state_client_fwd = state.clone();
                let route_conn: PortIpPort = PortIpPort {
                    nw_one_ip: addr.ip(),
                    nw_one_src_port: addr.port(),
                    ///TODO: if port is already used, new port should be assigned
                    nw_two_src_port: addr.port(),
                };
                let (server_in_tx, server_in_rx) = mpsc::channel(100);
                let (forwarding_tx, forwarding_rx) = mpsc::channel(100);
                let (client_in_tx, client_in_rx) = mpsc::channel(100);

                let wr_client_handle: task::JoinHandle<()>;
                let rd_client_handle: task::JoinHandle<()>;

                // Tcp client should be started for forwarded network
                if let Some((wr_handle, rd_handle)) = start_tcp_pinecone_fwd_client(
                    fwd_nw_id,
                    recv_nw_id,
                    route_conn,
                    state_client_fwd,
                    forwarding_rx,
                    client_in_tx,
                )
                .await
                {
                    wr_client_handle = wr_handle;
                    rd_client_handle = rd_handle;
                } else {
                    drop(server_in_tx);
                    drop(forwarding_tx);
                    continue;
                }

                tracing::info!(
                    "tcp pinecone connection has been established,nw id:{},route map:{:?}",
                    recv_nw_id as u16,
                    route_conn
                );

                let (recv_socket, sender_socket) = io::split(socket);

                // Asynchronously send tcp data.
                let server_wr = tokio::spawn(async move {
                    sender_tcp_pinecone_process(fwd_nw_id, sender_socket, route_conn, client_in_rx)
                        .await;
                });

                let state_recv = state.clone();
                // Asynchronously wait for an inbound socket.
                let server_rd = tokio::spawn(async move {
                    receive_tcp_pinecone_process(
                        recv_nw_id,
                        fwd_nw_id,
                        recv_socket,
                        state_recv,
                        route_conn,
                        server_in_tx,
                    )
                    .await;
                });
                let state_fwd_process = state.clone();

                let forwarding_process_handle = tokio::spawn(async move {
                    forwarding_process(
                        recv_nw_id,
                        fwd_nw_id,
                        route_conn,
                        state_fwd_process,
                        server_in_rx,
                        forwarding_tx,
                    )
                    .await;
                });

                state
                    .add_new_tcp_conn_route(
                        recv_nw_id,
                        route_conn,
                        Some(server_wr),
                        Some(server_rd),
                        Some(forwarding_process_handle),
                        Some(wr_client_handle),
                        Some(rd_client_handle),
                    )
                    .await;
            }
            Err(err) => {
                tracing::error!(
                    "Error accepting connection,recv nw id:{},forwarded nw id:{},err:{:?}",
                    recv_nw_id as u16,
                    fwd_nw_id as u16,
                    err
                );
            }
        }
    }
    // tracing::info!("Pinecone server is closed nw id : {}", recv_nw_id as u16,);
}

async fn sender_tcp_pinecone_process(
    nw_id: NwId,
    mut sender_socket: WriteHalf<TcpStream>,
    route_info: PortIpPort,
    mut rx_chan: mpsc::Receiver<Vec<u8>>,
) {
    loop {
        if let Some(data) = rx_chan.recv().await {
            if let Err(err) = sender_socket.write_all(&data).await {
                tracing::error!("tcp pinecone server writeall: {:?}", err);
            }

            tracing::debug!(
                "tcp pinecone server outgoing data,nw_id:{},route_info:{:?},size:{},payload:\n",
                nw_id as u16,
                route_info,
                data.len()
            );
            log_payload("\n", &data).await;
        }
    }
}

async fn receive_tcp_pinecone_process(
    recv_nw_id: NwId,
    fwd_nw_id: NwId,
    mut recv_socket: ReadHalf<TcpStream>,
    state: SharedState,
    route_info: PortIpPort,
    tx_chan: mpsc::Sender<Vec<u8>>,
) {
    //let port_num = state.get_tcp_src_port_nw_one(NwId::One).await;
    let mut buf: Vec<u8> = vec![0; TCP_PAYLOAD_SIZE];

    loop {
        buf.resize_with(TCP_PAYLOAD_SIZE, Default::default);
        match recv_socket.read(&mut buf).await {
            Ok(0) => {
                tracing::info!(
                    "tcp pinecone connection has been closed,nw_id:{},route map:{:?}",
                    recv_nw_id as u16,
                    route_info
                );
                state
                    .send_term_signal_for_tcproute(recv_nw_id, route_info)
                    .await;
                sleep(Duration::from_millis(1000)).await;
            }
            Ok(n) => {
                tracing::debug!(
                    "tcp pinecone server incoming data,nw_id:{},route_info:{:?},size:{},payload:\n",
                    recv_nw_id as u16,
                    route_info,
                    n
                );
                log_payload("\n", &buf[0..n].to_vec()).await;

                /*state
                .insert_tcp_incoming_data(recv_nw_id, route_info, buf[0..n].to_vec())
                .await;*/
                //let _ = tx_fwd_process_waker.send(true);
                if let Err(err) = tx_chan.send(buf[0..n].to_vec()).await {
                    tracing::error!(
                        "tcp pinecone server sending data to other task eror:{:?}",
                        err
                    );
                }
            }
            Err(e) => {
                tracing::error!("Error reading from socket: {}", e);
                state
                    .send_term_signal_for_tcproute(recv_nw_id, route_info)
                    .await;
            }
        }
    }

    state
        .remove_tcp_incoming_route(recv_nw_id, route_info)
        .await;
}

///packet validation and forwarding process should be done in this function
async fn forwarding_process(
    recv_nw_id: NwId,
    fwd_nw_id: NwId,
    route_info: PortIpPort,
    shared_state: SharedState,
    mut rx_chan: mpsc::Receiver<Vec<u8>>,
    tx_chan: mpsc::Sender<Vec<u8>>,
) {
    loop {
        if let Some(data) = rx_chan.recv().await {
            let len = data.len();
            if validate_data(&data, route_info) {
                if let Err(err) = tx_chan.send(data).await {
                    tracing::error!(
                        "tcp pinecone forwarding, sending data to other task eror:{:?}",
                        err
                    );
                }
            }
            tracing::info!(
                "Packet forwarding process,recv nw id:{},forwarded nw id:{},route map:{:?},size:{}",
                recv_nw_id as u16,
                fwd_nw_id as u16,
                route_info,
                len
            );
        }
    }

    shared_state
        .remove_tcp_outgoing_route(fwd_nw_id, route_info)
        .await;
}

fn validate_data(_data: &[u8], _route_info: PortIpPort) -> bool {
    true
}

async fn start_tcp_pinecone_fwd_client(
    recv_nw_id: NwId,
    fwd_nw_id: NwId,
    route_info: PortIpPort,
    shared_state: SharedState,
    rx_chn: mpsc::Receiver<Vec<u8>>,
    tx_chn: mpsc::Sender<Vec<u8>>,
) -> Option<(task::JoinHandle<()>, task::JoinHandle<()>)> {
    let addr = shared_state
        .get_tcp_pinecone_dest_sock_addr(NwId::Two)
        .await;

    tracing::error!("client: addr:{:?}", addr);

    if let Ok(stream) = TcpStream::connect(addr).await {
        tracing::info!("Tcp client forwarding connection is established");
        let (rd_sock, wr_sock) = io::split(stream);
        let rd_state = shared_state.clone();
        let rd_handle = tokio::spawn(async move {
            receive_tcp_pinecone_client_process(
                recv_nw_id, fwd_nw_id, rd_sock, route_info, rd_state, tx_chn,
            )
            .await;
        });

        let wr_handle = tokio::spawn(async move {
            sender_tcp_pinecone_client_process(
                recv_nw_id,
                fwd_nw_id,
                wr_sock,
                route_info,
                shared_state,
                rx_chn,
            )
            .await;
        });

        return Some((wr_handle, rd_handle));
    }

    tracing::error!("Tcp client forwarding connection cannot be established");
    None
}

async fn receive_tcp_pinecone_client_process(
    recv_nw_id: NwId,
    fwd_nw_id: NwId,
    mut socket: ReadHalf<TcpStream>,
    route_info: PortIpPort,
    shared_state: SharedState,
    tx_chan: mpsc::Sender<Vec<u8>>,
) {
    tracing::debug!(
        "tcp pinecone client  receiver is started,route info:{:?},recv nw id:{},fwd nw id:{}",
        route_info,
        recv_nw_id as u16,
        fwd_nw_id as u16
    );
    let mut buf: Vec<u8> = vec![0; TCP_PAYLOAD_SIZE];

    loop {
        buf.resize_with(TCP_PAYLOAD_SIZE, Default::default);
        match socket.read(&mut buf).await {
            Ok(0) => {
                tracing::info!(
                    "tcp pinecone client connection has been closed,nw_id:{},route map:{:?}",
                    recv_nw_id as u16,
                    route_info
                );
                shared_state
                    .send_term_signal_for_tcproute(recv_nw_id, route_info)
                    .await;
                sleep(Duration::from_millis(1000)).await;
            }
            Ok(n) => {
                tracing::debug!(
                    "tcp pinecone client incoming data,nw_id:{},route_info:{:?},size:{},payload:\n",
                    recv_nw_id as u16,
                    route_info,
                    n
                );
                log_payload("\n", &buf[0..n].to_vec()).await;

                /*shared_state
                .insert_tcp_incoming_data(recv_nw_id, route_info, buf[0..n].to_vec())
                .await;*/
                if let Err(err) = tx_chan.send(buf[0..n].to_vec()).await {
                    tracing::error!(
                        "tcp pinecone client, sending data to other task eror:{:?}",
                        err
                    );
                }
            }
            Err(e) => {
                tracing::error!("Error reading from socket: {}", e);
                shared_state
                    .send_term_signal_for_tcproute(recv_nw_id, route_info)
                    .await;
            }
        }
    }
}

async fn sender_tcp_pinecone_client_process(
    recv_nw_id: NwId,
    fwd_nw_id: NwId,
    mut socket: WriteHalf<TcpStream>,
    route_info: PortIpPort,
    shared_state: SharedState,
    mut rx_chan: mpsc::Receiver<Vec<u8>>,
) {
    loop {
        if let Some(data) = rx_chan.recv().await {
            if let Err(err) = socket.write_all(&data).await {
                tracing::error!("tcp pinecone client writeall: {:?}", err);
            }

            tracing::debug!(
                "tcp pinecone client outgoing,recv nw id:{},fwd nw id:{},route_info:{:?},size:{},payload:\n",
                recv_nw_id as u16,
                fwd_nw_id as u16,
                route_info,
                data.len()
            );
            log_payload("\n", &data).await;
        }
    }
}
