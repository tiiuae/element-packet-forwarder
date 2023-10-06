#![doc(html_root_url = "https://docs.rs/element-packet-forwarder/0.1.0")]
#![warn(rust_2018_idioms, missing_docs)]
//#![deny(warnings, dead_code, unused_imports, unused_mut)]

//! [![github]](https://github.com/enesoztrk/element-packet-forwarder)&ensp;[![crates-io]](https://crates.io/crates/element-packet-forwarder)&ensp;[![docs-rs]](https://docs.rs/element-packet-forwarder)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K
//!
//! <br>
//!
//! Packet forwarder app to run element app on ghaf project.
//!
//! <br>
//!
//! ## Usage
//!
//! ```no_run
//! use element_packet_forwarder::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let shared_state = SharedState::new().await;
//!
//!    let (_pinecone_res, _proxy_res, _tracing_res) = join!(
//!         fwd_udp::start_pinecone_udp_mcast(shared_state.clone()),
//!         start_proxy(shared_state.clone()),
//!         start_tracing_engine()
//!     );
//! }
//! ```
//!
//!
//! ## Readme Docs
//!
//! You can find the crate's readme documentation on the
//! [crates.io] page, or alternatively in the [`README.md`] file on the GitHub project repo.
//!
//! [crates.io]: https://crates.io/crates/element-packet-forwarder
//! [`README.md`]: https://github.com/enesoztrk/element-packet-forwarder
//!

/// command line parsing and handling module
pub mod cli;
/// forwarding tcp packets between networks module
pub mod fwd_tcp;
/// forwarding udp packets between networks module
pub mod fwd_udp;
/// Shared data between tasks module
pub mod shared_state;

use crate::shared_state::*;
use tokio::time::{sleep, Duration};

pub async fn start_proxy(state: SharedState) -> Result<(), Box<dyn std::error::Error>> {
    //proxy task to check and forward data packets
    let proxy_task_handle = tokio::spawn(async move {
        proxy_process(state).await;
    });

    proxy_task_handle.await.expect("proxy task function error");
    Ok(())
}

async fn proxy_process(state: SharedState) {
    
    tracing::error!("Proxy process has started");
    
    loop {
        let is_connected=state.is_udp_pinecone_connected(1).await;
        tracing::debug!("Hey, I am proxy process:{}",is_connected);
        sleep(Duration::from_millis(1000)).await;
    }
}

pub async fn start_tracing_engine() -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

async fn log_payload(str: &str, data: &Vec<u8>) {
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
