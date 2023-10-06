///TODO: winapi functions should be added for windows support
use nix::net::if_::*;
use std::error::Error;
use std::ffi::CString;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use tokio::io::{self, ReadHalf,WriteHalf};
use tokio::net::{TcpListener, ToSocketAddrs,TcpStream};
use tokio::time::{sleep, Duration};
use crate::NwId;
use crate::cli;
use crate::shared_state::SharedState;
use crate::shared_state::PortIpPort;
use tokio::io::{AsyncReadExt,AsyncWriteExt};


const MAX_CLIENT_NUM:i32=128;








fn tcp_sock_ipv6_init(
    interface_name: &str,
    port_num: u16,
    max_client_num:i32
) -> Result<std::net::TcpListener, Box<dyn Error>> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};

    // let ipv6_addr = Ipv6Addr::from_str(ipv6_str).expect("Failed to parse IPv6 address");
    let ipv6_addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    // Convert the interface name to  a CStr
    let ifname =
        CString::new(interface_name.as_bytes()).expect("Failed to create CStr from interface name");

    // Get the interface index (scope ID) using if_nametoindex
    let ifindex = if_nametoindex(ifname.as_c_str()).unwrap_or_else(|err| {
        panic!("Error getting {:?} interface index: {}",ifname, err);
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
    // socket.set_tcp_nodelay(true)?;

    // Listen for incoming connections
    socket.listen(max_client_num).expect("tcp max client size set err"); // Set the backlog queue size
    Ok(socket.into())
}


fn create_pinecone_tcp_sock(nw_id: NwId,port_num: u16) -> tokio::net::TcpListener {
    let std_tcp_sock = if nw_id == NwId::One {
        tcp_sock_ipv6_init(cli::get_if1_name().unwrap(),port_num,MAX_CLIENT_NUM).unwrap()
    } else {
        tcp_sock_ipv6_init(cli::get_if2_name().unwrap(),port_num,MAX_CLIENT_NUM).unwrap()
    };

    TcpListener::from_std(std_tcp_sock).expect("tcp pinecone from std err")
}


pub async fn start_tcp_pinecone_server(nw_id:NwId,state:SharedState){

    tracing::info!("Pinecone server is starting nw id : {}",nw_id as u16,);
    let port_num=state.get_tcp_src_port_nw_one(nw_id).await;
    let tcp_pinecone_sock = create_pinecone_tcp_sock(nw_id,port_num);


    let tcp_pinecone_server_handle = tokio::spawn(async move {
        tcp_pinecone_server_process(nw_id,tcp_pinecone_sock,state.clone()).await;
    });

    tcp_pinecone_server_handle
    .await
    .expect("tcp pinecone server has started error");
}




pub async fn tcp_pinecone_server_process(nw_id: NwId,sock:TcpListener,state:SharedState){
    
    let tcp_port_num=state.get_tcp_src_port_nw_one(nw_id).await;
    tracing::info!("Pinecone server process is starting nw id : {},port num:{}",nw_id as u16,tcp_port_num);

    loop {
        
        // if udp port has been changed, terminate the server
        if tcp_port_num!=state.get_tcp_src_port_nw_one(nw_id).await{
            break;
        }
 

        match sock.accept().await{
            Ok((socket, addr)) => {
            let state_recv: SharedState=state.clone();
            let state_send: SharedState=state.clone();

            let route_conn:PortIpPort=PortIpPort{
                nw_one_ip:addr.ip(),
                nw_one_src_port:addr.port(),
                ///TODO: if port is already used, new port should be assigned
                nw_two_src_port:addr.port()
            };
            tracing::info!("tcp pinecone connection has been established,nw_id:{},route map:{:?}",nw_id as u16,route_conn);

            let (recv_socket, sender_socket) = io::split(socket);
            
            // Asynchronously wait for an inbound socket.
            tokio::spawn(async move {
                sender_tcp_pinecone_process(nw_id,sender_socket, state_send,route_conn).await;

            });

            // Asynchronously send tcp data.
            tokio::spawn(async move {
                receive_tcp_pinecone_process(nw_id,recv_socket, state_recv,route_conn).await;

            });
            
            }
            Err(err) => {

                tracing::error!("Error accepting connection,Nw id:{},err:{:?}",nw_id as u16,err);

            }

        }

    }
}


async fn sender_tcp_pinecone_process(nw_id:NwId,sender_socket:WriteHalf<TcpStream>,state:SharedState,route_info:PortIpPort){



/*loop {
    
}*/

}


#[tracing::instrument]
async fn receive_tcp_pinecone_process(nw_id:NwId,mut recv_socket:ReadHalf<TcpStream>,state:SharedState,route_info:PortIpPort){


loop{
    let mut buf: Vec<u8> = vec![0;2048];

    match recv_socket.read(&mut buf).await {
        Ok(0) => {
            break;
        }
        Ok(n) => {        
            tracing::debug!("tcp pinecone incoming data,nw_id:{},route_info:{:?},data:{}\n{:?}",nw_id as u16,route_info,buf.len(),&buf[0..n]);
         },
        Err(e) => {
            tracing::error!("Error reading from socket: {}", e);
            break;
        }
    }

}
tracing::info!("tcp pinecone connection has been closed,nw_id:{},route map:{:?}",nw_id as u16,route_info);

}
