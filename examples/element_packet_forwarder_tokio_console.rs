use element_packet_forwarder::fwd_udp;
use element_packet_forwarder::fwd_tcp;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_proxy;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
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
        

        start_element_packet_forwarder_tracing().await;

    Ok(())
}


#[tracing::instrument]
async fn start_element_packet_forwarder_tracing() {
      
    let shared_state = SharedState::new().await;

    let ( _pinecone_udp_res) = join!(
        fwd_udp::start_pinecone_udp_mcast(shared_state.clone())    );

}