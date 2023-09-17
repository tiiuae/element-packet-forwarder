/// command line parsing and handling module
mod cli;


use std::net::{Ipv6Addr, SocketAddrV6};
use tokio::net::{TcpListener, TcpStream};
use tokio::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
   //let args= cli::handling_args()?;
   cli::get_if1_ip()?;
   cli::get_if2_ip()?;

    Ok(())
}
