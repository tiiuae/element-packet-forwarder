/// command line parsing and handling module
mod cli;
/// udp communication module
mod udp;
use std::net::{Ipv6Addr, SocketAddrV6};
use tokio::net::{TcpListener, TcpStream};
use tokio::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
   //let args= cli::handling_args()?;
   //cli::get_if1_ip()?;
   //cli::get_if2_ip()?;
   
    println!("ip: {:?}, name: {:?}",cli::get_if1_ip(),cli::get_if1_name());


  // udp::udp_ipv6_init("wlp0s20f3",cli::get_if1_ip()?.to_string().as_str());
    
        Ok(())
}
