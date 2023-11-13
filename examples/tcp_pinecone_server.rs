/*
    Copyright 2022-2023 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
///cargo build --example tcp-pinecone-server
use element_packet_forwarder::fwd_tcp;
use element_packet_forwarder::shared_state;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_task_management;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
use std::env;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let shared_state = SharedState::new().await;
    let mut fake_udp_payload = vec![0; 34];
    fake_udp_payload[32] = 0x60;
    fake_udp_payload[33] = 0xf;
    shared_state.set_tcp_src_port_nw_one(&fake_udp_payload);

    let (_tracing_res, _pinecone_res, _task_mngmt_res) = join!(
        start_tracing_engine(),
        fwd_tcp::start_tcp_pinecone_server(NwId::One, NwId::One, shared_state.clone()),
        start_task_management(shared_state.clone())
    );
    Ok(())
}
