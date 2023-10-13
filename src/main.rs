use element_packet_forwarder::fwd_tcp;
use element_packet_forwarder::fwd_udp;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_task_management;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
use std::error::Error;
//#[tokio::main]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), Box<dyn Error>> {
    let shared_state = SharedState::new().await;

    let (_tracing_res, _pinecone_udp_res, _proxy_res) = join!(
        start_tracing_engine(),
        fwd_udp::start_pinecone_udp_mcast(shared_state.clone()),
        start_task_management(shared_state.clone())
    );

    Ok(())
}
