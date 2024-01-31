/*
    Copyright 2022-2023 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use clap::Parser;
use lazy_static::lazy_static;
use pnet::datalink::{self};
use std::error::Error;
use std::net::IpAddr;
use std::str;
use std::net::Ipv4Addr;
lazy_static! {
    static ref CLI_ARGS: Args = {


        // Initialize the IP address using a function or any other logic
        let args=handling_args().expect("Error in argument handling");
        println!("{args:?}");
        args
    };
}

const VERSION:&str = concat!("\ncommit sha: ",env!("VERGEN_GIT_SHA") ,"\ncommit date: ",env!("VERGEN_GIT_COMMIT_DATE"),"\ncommit timestamp:",env!("VERGEN_GIT_COMMIT_TIMESTAMP"),"\nbuild timestamp:",env!("VERGEN_BUILD_TIMESTAMP"));

/// Packet forwarder cli argument parser
#[derive(Parser, Debug)]
#[command(author, about, long_about =None ,version =VERSION)]
struct Args {
    /// Name of first network interface
    #[arg(long)]
    pub if1: String,

    /// Ip version of first network interface
    #[arg(long,default_value_t=String::from("off"),value_parser=is_on_off)]
    pub is_if1_ipv6: String,

    /// Name of second network interface
    #[arg(long)]
    pub if2: String,

    /// Ip version of second network interface
    #[arg(long,default_value_t=String::from("off"),value_parser=is_on_off)]
    pub is_if2_ipv6: String,

    /// Log severity
    #[arg(long, default_value_t = String::from("debug"))]
    pub log_level: String,
}

fn is_on_off(s: &str) -> Result<String, String> {
    let val: String = s.parse().map_err(|_| format!("`{s}` isn't a string"))?;
    if val == "on" || val == "off" {
        Ok(val)
    } else {
        Err("Value can be on or off".to_string())
    }
}

fn handling_args() -> Result<Args, Box<dyn Error>> {
    let args: Args = Args::parse();

   
    Ok(args)
}

fn get_interface_ips(
    interface_name: &str,
) -> Result<(Option<IpAddr>, Option<IpAddr>), Box<dyn Error>> {
    let interfaces = datalink::interfaces();

    for interface in interfaces {
        if interface.name == interface_name {
            let mut ipv4_addr = None;
            let mut ipv6_addr = None;

            for ip in &interface.ips {
                match ip.ip() {
                    IpAddr::V4(v4) => {
                        ipv4_addr = Some(IpAddr::V4(v4));
                    }
                    IpAddr::V6(v6) => {
                        ipv6_addr = Some(IpAddr::V6(v6));
                    }
                }
            }

            return Ok((ipv4_addr, ipv6_addr));
        }
    }

    Err("Interface not found".into())
}

fn get_app_ip(interface_name: &str, is_ipv6_on: &str) -> Result<IpAddr, Box<dyn Error>> {
    match get_interface_ips(interface_name) {
        Ok((ipv4, ipv6)) => {
            if is_ipv6_on == "off" {
                if let Some(ipv4) = ipv4 {
                    println!(
                        "IPv4 address associated with interface '{}': {}",
                        interface_name, ipv4
                    );
                    //return Err("IPv4 is not supported".into());
                    Ok(ipv4)
                } else {
                    println!(
                        "No IPv4 address associated with interface '{}'",
                        interface_name
                    );
                    Err("IPv4 is not found".into())
                }
            } else if let Some(ipv6) = ipv6 {
                println!(
                    "IPv6 address associated with interface '{}': {}",
                    interface_name, ipv6
                );
                Ok(ipv6)
            } else {
                println!(
                    "No IPv6 address associated with interface '{}'",
                    interface_name
                );
                Err("IPv6 is not found".into())
            }
        }

        // Default to None for both IPv4 and IPv6 in case of an error
        Err(err) => Err(err),
    }
}

pub fn get_if1_ip() -> Result<IpAddr, Box<dyn Error>> {
    get_app_ip(&CLI_ARGS.if1, &CLI_ARGS.is_if1_ipv6)
}
pub fn get_if2_ip() -> Result<IpAddr, Box<dyn Error>> {
    //get_app_ip(&CLI_ARGS.if2, &CLI_ARGS.is_if2_ipv6)
    /*
    For testing ghaf environment. There are two ip addresses in the same network interface.
    That's why it selects the wrong ip address.
    */
    let ipv4_address = IpAddr::V4(Ipv4Addr::new(192, 168, 100, 1));

    Ok(ipv4_address) 
}
/// Checks if the first network interface has an IPv4 address.
///
/// This function checks if the first network interface (`if1`) has an IPv4 address
/// by calling `get_if1_ip` and inspecting the result.
///
/// # Returns
///
/// Returns `true` if an IPv4 address is associated with `if1`, otherwise `false`
pub fn is_if1_ipv4() -> bool {
    if let Ok(ip_addr) = get_if1_ip() {
        ip_addr.is_ipv4()
    } else {
        false // Error occurred while retrieving IP address
    }
}
/// Checks if the second network interface has an IPv4 address.
///
/// This function checks if the second network interface (`if2`) has an IPv4 address
/// by calling `get_if2_ip` and inspecting the result.
///
/// # Returns
///
/// Returns `true` if an IPv4 address is associated with `if2`, otherwise `false`
pub fn is_if2_ipv4() -> bool {
    if let Ok(ip_addr) = get_if2_ip() {
        ip_addr.is_ipv4()
    } else {
        false // Error occurred while retrieving IP address
    }
}

pub fn get_if1_name() -> Option<&'static str> {
    if CLI_ARGS.if1.is_empty() {
        return None;
    }
    Some(&CLI_ARGS.if1)
}

pub fn get_if2_name() -> Option<&'static str> {
    if CLI_ARGS.if2.is_empty() {
        return None;
    }

    Some(&CLI_ARGS.if2)
}
