use element_packet_forwarder::fwd_tcp;
use element_packet_forwarder::shared_state::*;
use element_packet_forwarder::start_proxy;
use element_packet_forwarder::start_tracing_engine;
use futures::join;
use std::error::Error;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    console_subscriber::init();

    let shared_state = SharedState::new().await;
    let mut fake_udp_payload=vec![0;34];
    fake_udp_payload[32]=0x60;
    fake_udp_payload[33]=0xf;
    shared_state.set_tcp_src_port_nw_one(&fake_udp_payload);
    let (_pinecone_res, _proxy_res) = join!(
        start_tracing_engine(),
        fwd_tcp::start_tcp_pinecone_server(NwId::One,shared_state)
    );
    

    Ok(())
}
