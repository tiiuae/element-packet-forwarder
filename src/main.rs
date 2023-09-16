/// command line parsing and handling module
mod cli;


use std::net::{Ipv6Addr, SocketAddrV6};
use tokio::net::{TcpListener, TcpStream};
use tokio::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //println!("Hello world!");
    cli::handling_args()?;
    println!("Hello world!");
    Ok(())
}
