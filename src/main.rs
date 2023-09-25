/// command line parsing and handling module
mod cli;
/// udp communication module
mod fwd_udp;
use tokio::net::{TcpListener, TcpStream,UdpSocket};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::error::Error;
use tokio::time::sleep;

#[derive(PartialEq)]
pub enum NwId{

    One,
    Two,

}


// Import the hex crate
#[tokio::main]
async fn main() ->  Result<(), Box<dyn Error>>{
    
     // Start configuring a `fmt` subscriber
     let subscriber = tracing_subscriber::fmt()
     // Use a more compact, abbreviated log format
     .compact()
     // Display source code file paths
     .with_file(true)
     // Display source code line numbers
     .with_line_number(true)
     // Display the thread ID an event was recorded on
     .with_thread_ids(true)
     // Don't display the event's target (module path)
     .with_target(false)
     .with_max_level(tracing::Level::TRACE)
     // Build the subscriber
     .finish();

   // Set the subscriber as the default
   tracing::subscriber::set_global_default(subscriber).unwrap();

    let udp_pinecone_mcast_sock=create_pinecone_udp_sock(NwId::One);

    let udp_pinecone_mcast_nw_one_handle=tokio::spawn(async move {
        udp_pinecone_receive_nw_one(udp_pinecone_mcast_sock).await;
    });

    let udp_pinecone_mcast_sock=create_pinecone_udp_sock(NwId::Two);

    let udp_pinecone_mcast_nw_two_handle=tokio::spawn(async move {
        udp_pinecone_receive_nw_two(udp_pinecone_mcast_sock).await;
    });



    udp_pinecone_mcast_nw_one_handle.await.expect("udp pinecone network one mcast function error");
    udp_pinecone_mcast_nw_two_handle.await.expect("udp pinecone network two mcast function error");

    Ok(())
}


/// Receive bytes from Udp Socket from nw one
async fn udp_pinecone_receive_nw_one(rx_socket: UdpSocket) {
    let mut buf = vec![0; 1024];
    loop {
        match rx_socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                let data = buf[..size].to_vec();
                log_payload(&format!("[1]Udp data received from {}, size{},payload:\n",peer.ip(),size),&data).await;
            }
            Err(e) => {
                tracing::error!("Error receiving data: {:?}", e);
            }
        }
    }

}


/// Receive bytes from Udp Socket from nw two
async fn udp_pinecone_receive_nw_two(rx_socket: UdpSocket) {
    let mut buf = vec![0; 1024];
    loop {
        match rx_socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                let data = buf[..size].to_vec();
                log_payload(&format!("[2]Udp data received from {}, size{},payload:\n",peer.ip(),size),&data).await;
            }
            Err(e) => {
                tracing::error!("Error receiving data: {:?}", e);
            }
        }
    }

}

async fn log_payload(str:&str,data:&Vec<u8>){

    let mut formatted_payload = String::new();

    formatted_payload.push_str(str);
    for (index, value) in data.iter().enumerate() {
        // Append the hexadecimal representation of the byte
        formatted_payload.push_str(&format!("{:02x?}", value));

        if (index + 1) % 8 == 0 {
            // Start a new line after every 8 values
            formatted_payload.push('\n');
        } else {
            // Add a space between bytes
            formatted_payload.push(' ');
        }
    }

    // Ensure a new line at the end if the vector length is not a multiple of 8
    if data.len() % 8 != 0 {
        formatted_payload.push('\n');
    }

    // Log the entire formatted payload
    tracing::trace!("{}", formatted_payload);
}


fn create_pinecone_udp_sock(nw_id:NwId)->tokio::net::UdpSocket{

    let std_udp_sock:std::net::UdpSocket=
    if nw_id == NwId::One {
       fwd_udp::udp_ipv6_init(cli::get_if1_name().unwrap())

    }
    else{
        fwd_udp::udp_ipv6_init(cli::get_if2_name().unwrap())
    };


    UdpSocket::from_std(std_udp_sock).expect("from std err")


}