use element_packet_forwarder::fwd_udp;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_proxy;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let shared_state = SharedState::new().await;

    let (_pinecone_res, _proxy_res, _tracing_res) = join!(
        start_tracing_engine(),
        fwd_udp::start_pinecone_udp_mcast(shared_state.clone()),
        start_proxy(shared_state.clone())
    );

    Ok(())
}
