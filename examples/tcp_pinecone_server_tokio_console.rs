/*
    Copyright 2022-2023 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
///RUSTFLAGS="--cfg tokio_unstable" cargo build --example tcp-pinecone-server-tokio-console
use element_packet_forwarder::fwd_tcp;
use element_packet_forwarder::shared_state;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_task_management;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
use std::env;
use std::error::Error;
use tracing_subscriber::prelude::*;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let console_layer = console_subscriber::spawn();
    tracing_subscriber::registry()
        .with(console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
        )
        .init();

    //console_subscriber::init();
    let shared_state = SharedState::new().await;
    let mut fake_udp_payload = vec![0; 34];
    fake_udp_payload[32] = 0x60;
    fake_udp_payload[33] = 0xf;
    shared_state.set_tcp_src_port_nw_one(&fake_udp_payload);

    start_tcp_pinecone_server_tracing(shared_state).await;

    Ok(())
}

#[tracing::instrument]
async fn start_tcp_pinecone_server_tracing(shared_state: SharedState) {
    let _ = join!(
        start_task_management(shared_state.clone()),
        fwd_tcp::start_tcp_pinecone_server(NwId::One, NwId::One, shared_state)
    );
}
