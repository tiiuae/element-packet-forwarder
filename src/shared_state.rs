/*
    Copyright 2022-2023 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::cli::get_if2_name;
use std::collections::{HashMap, VecDeque};
use std::net::Ipv6Addr;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::{self, JoinHandle};
const TOTAL_NUM_NW: usize = 2;
const UDP_CONN_MAX_TICK: u8 = 3;
#[derive(PartialEq, Clone, Debug, Copy)]

/// Enum for network id
///
///
pub enum NwId {
    ///Network id 1
    One = 0,
    ///Network id 2
    Two = 1,
}

impl From<usize> for NwId {
    fn from(value: usize) -> Self {
        match value {
            0 => NwId::One,
            1 => NwId::Two,
            _ => panic!("Invalid usize value for NwId"),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub struct PortIpPort {
    pub nw_one_ip: IpAddr,
    pub nw_one_src_port: u16,
    pub nw_two_src_port: u16,
}

type DataqT = VecDeque<Vec<u8>>;
const UDP_PINECONE_PAYLOAD_SIZE: usize = 34;
type PineconeUdpDataT = Vec<u8>;
#[derive(Debug, PartialEq, Clone)]
struct TcpData {
    dataq: DataqT,
    is_connected: bool,
}

///tcp forwarding route map for incoming packets
/// key: (network 1 ip,network 1 src port num,network 2 src port num)
/// value : tcp data
type TcpFwdRouteMap = HashMap<PortIpPort, TcpData>;

type TaskHandle = Option<task::JoinHandle<()>>;

#[derive(Debug)]
pub struct TcpFwdRouteTask {
    pub server_wr: TaskHandle,
    pub server_rd: TaskHandle,
    pub fwd_process_handle: TaskHandle,
    pub client_wr: TaskHandle,
    pub client_rd: TaskHandle,
}

type TcpFwdRouteTaskMap = HashMap<PortIpPort, TcpFwdRouteTask>;

#[derive(Debug, Clone)]
pub struct SharedState {
    /// tcp incoming data and route info
    tcp_con_in: [Arc<Mutex<TcpFwdRouteMap>>; TOTAL_NUM_NW],

    /// tcp outgoing data and route info
    tcp_con_out: [Arc<Mutex<TcpFwdRouteMap>>; TOTAL_NUM_NW],

    /// tcp tasks and route info
    tcp_route_task_map: [Arc<Mutex<TcpFwdRouteTaskMap>>; TOTAL_NUM_NW],

    /// tcp pinecone server port for network one
    tcp_src_port_nw_one: Arc<AtomicU16>,

    /// tcp pinecone server ip for network two
    tcp_dest_ip_nw_two: Arc<Mutex<IpAddr>>,

    ///tcp pinecone server terminate signal for network one
    is_tcp_server_termination_signal_got_nw_one: Arc<AtomicBool>,
    /// udp pinecone incoming packets handling
    udp_pinecone_in: Vec<Arc<Mutex<PineconeUdpDataT>>>,

    /// tcp pinecone server task handle
    tcp_pinecone_server_main_task_handle: Arc<Mutex<TaskHandle>>,

    /// udp pinecone outgoing packets handling
    udp_pinecone_out: Vec<Arc<Mutex<PineconeUdpDataT>>>,

    /// is udp pinecone  data exchange still available?
    udp_pinecone_network_conn_tick: [Arc<AtomicU8>; TOTAL_NUM_NW],

    ///
    terminated_task_handles: Arc<Mutex<Vec<task::AbortHandle>>>,
}

impl SharedState {
    pub async fn new() -> Self {
        Self {
            tcp_con_in: Default::default(),
            tcp_con_out: Default::default(),
            tcp_route_task_map: Default::default(),
            tcp_src_port_nw_one: Arc::new(AtomicU16::new(0)),
            udp_pinecone_in: {
                let mut v = Vec::with_capacity(TOTAL_NUM_NW);
                (0..TOTAL_NUM_NW)
                    .for_each(|_| v.push(Arc::new(Mutex::new(vec![0; UDP_PINECONE_PAYLOAD_SIZE]))));
                v
            },
            udp_pinecone_out: {
                let mut v = Vec::with_capacity(TOTAL_NUM_NW);
                (0..TOTAL_NUM_NW)
                    .for_each(|_| v.push(Arc::new(Mutex::new(vec![0; UDP_PINECONE_PAYLOAD_SIZE]))));
                v
            },
            udp_pinecone_network_conn_tick: [
                Arc::new(AtomicU8::new(UDP_CONN_MAX_TICK + 1)),
                Arc::new(AtomicU8::new(UDP_CONN_MAX_TICK + 1)),
            ],
            is_tcp_server_termination_signal_got_nw_one: Arc::new(AtomicBool::new(false)),
            tcp_dest_ip_nw_two: Arc::new(Mutex::new(IpAddr::V6(Ipv6Addr::new(
                0, 0, 0, 0, 0, 0, 0, 0,
            )))),
            terminated_task_handles: Arc::new(Mutex::new(Vec::new())),
            tcp_pinecone_server_main_task_handle: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub async fn insert_tcp_incoming_data(
        &self,
        nw_id: NwId,
        route_info: PortIpPort,
        in_data: Vec<u8>,
    ) -> bool {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_in[index].lock().await;
        match tcp_con.get_mut(&route_info) {
            None => {
                let mut new_dataq = VecDeque::new();
                new_dataq.push_back(in_data);
                let in_data = TcpData {
                    dataq: new_dataq,
                    is_connected: true,
                };

                if tcp_con.insert(route_info, in_data).is_none() {
                    tracing::trace!("First data is added to shared state,nw_id:{}", index);
                }
            }
            Some(new_message) => {
                new_message.dataq.push_back(in_data);
            }
        }
        true
    }
    pub async fn remove_tcp_incoming_route(&self, nw_id: NwId, route_info: PortIpPort) -> bool {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_in[index].lock().await;
        tcp_con.remove(&route_info);
        true
    }
    pub async fn insert_tcp_outgoing_data(
        &self,
        nw_id: NwId,
        route_info: PortIpPort,
        in_data: Vec<u8>,
    ) -> bool {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_out[index].lock().await;
        match tcp_con.get_mut(&route_info) {
            None => {
                let mut new_dataq = VecDeque::new();
                new_dataq.push_back(in_data);
                let in_data = TcpData {
                    dataq: new_dataq,
                    is_connected: true,
                };
                if tcp_con.insert(route_info, in_data).is_none() {
                    tracing::trace!("First data is added to shared state,nw_id:{}", nw_id as u16);
                }
            }
            Some(new_message) => {
                new_message.dataq.push_back(in_data);
            }
        }
        true
    }

    pub async fn remove_tcp_outgoing_route(&self, nw_id: NwId, route_info: PortIpPort) -> bool {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_out[index].lock().await;
        tcp_con.remove(&route_info);
        true
    }

    fn get_tcp_fwd_route_data(
        &self,
        nw_id: usize,
        tcp_con_map: &mut HashMap<PortIpPort, TcpData>,
        nw_one_ip: IpAddr,
        nw_one_src_port: u16,
        nw_two_src_port: u16,
    ) -> Option<Vec<u8>> {
        let key: PortIpPort = PortIpPort {
            nw_one_ip,
            nw_one_src_port,
            nw_two_src_port,
        };

        if let Some(new_message) = tcp_con_map.get_mut(&key) {
            if !new_message.dataq.is_empty() {
                return new_message.dataq.pop_front();
            }
        }

        /*tracing::debug!(
            "There is no incoming data ->nw_id:{},route_info:{:?}",
            nw_id,
            key
        );*/
        None
    }

    pub async fn get_tcp_incoming_data(
        &self,
        nw_id: NwId,
        route_info: PortIpPort,
    ) -> Option<Vec<u8>> {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_in[index].lock().await;

        self.get_tcp_fwd_route_data(
            index,
            tcp_con.deref_mut(),
            route_info.nw_one_ip,
            route_info.nw_one_src_port,
            route_info.nw_two_src_port,
        )
    }

    pub async fn get_tcp_outgoing_data(
        &self,
        nw_id: NwId,
        route_info: PortIpPort,
    ) -> Option<Vec<u8>> {
        let index = nw_id as usize;
        let mut tcp_con = self.tcp_con_out[index].lock().await;

        self.get_tcp_fwd_route_data(
            index,
            tcp_con.deref_mut(),
            route_info.nw_one_ip,
            route_info.nw_one_src_port,
            route_info.nw_two_src_port,
        )
    }

    // Get the tcp source port number field
    pub async fn get_tcp_src_port_nw_one(&self, nw_id: NwId) -> u16 {
        self.tcp_src_port_nw_one.load(Ordering::Relaxed)
    }

    // Set the tcp source port number field
    pub fn set_tcp_src_port_nw_one(&self, udp_pinecone_in_data: &[u8]) {
        let port_num: u16 = u16::from_le_bytes([
            udp_pinecone_in_data[UDP_PINECONE_PAYLOAD_SIZE - 1],
            udp_pinecone_in_data[UDP_PINECONE_PAYLOAD_SIZE - 2],
        ]);
        tracing::debug!("Port num is set:{}", port_num);
        self.tcp_src_port_nw_one.store(port_num, Ordering::Relaxed)
    }

    ///insert udp pinecone incoming data
    pub async fn insert_udp_incoming_pinecone_data(
        &self,
        nw_id: usize,
        in_data: PineconeUdpDataT,
    ) -> bool {
        //we have to allow receiving udp pinecone packets only from network id 1
        assert!(nw_id == 1);
        let mut udp_map = self.udp_pinecone_in[nw_id].lock().await;
        if in_data.len() == UDP_PINECONE_PAYLOAD_SIZE {
            self.set_tcp_src_port_nw_one(&in_data);
            let _ = std::mem::replace(udp_map.deref_mut(), in_data);
            tracing::trace!("udp incoming pinecone data is inserted: {:?}", udp_map);
            return true;
        }
        tracing::error!(
            "udp pinecone incoming data payload size is not {},which is {}",
            UDP_PINECONE_PAYLOAD_SIZE,
            in_data.len()
        );
        false
    }

    ///get udp pinecone incoming data
    pub async fn get_udp_incoming_pinecone_data(&self, nw_id: usize) -> Option<PineconeUdpDataT> {
        //we have to allow receiving udp pinecone packets only from network id 1
        assert!(nw_id == 1);
        let mut udp_map = self.udp_pinecone_in[nw_id].lock().await;
        let is_all_zeros = udp_map.iter().all(|&x| x == 0);

        if !is_all_zeros {
            tracing::trace!("udp incoming pinecone data is got: {:?}", udp_map);
            return Some(std::mem::replace(
                udp_map.deref_mut(),
                vec![0; UDP_PINECONE_PAYLOAD_SIZE],
            ));
        }
        //tracing::error!("udp incoming pinecone data is not available or consumed");

        None
    }

    pub async fn is_udp_pinecone_connected(&self, nw_id: usize) -> bool {
        let udp_pinecone_tick: u8 =
            self.udp_pinecone_network_conn_tick[nw_id].load(Ordering::Relaxed);

        if udp_pinecone_tick > UDP_CONN_MAX_TICK {
            //udp connection is lost, trigger the port changed state to stop related tasks
            self.tcp_src_port_nw_one.store(0, Ordering::Relaxed);
        }

        udp_pinecone_tick < UDP_CONN_MAX_TICK
    }

    pub async fn udp_pinecone_feed_tick(&self, nw_id: usize) {
        let mut udp_pinecone_tick: u8;
        {
            udp_pinecone_tick = self.udp_pinecone_network_conn_tick[nw_id].load(Ordering::Relaxed);
        }

        if udp_pinecone_tick > UDP_CONN_MAX_TICK {
            udp_pinecone_tick = UDP_CONN_MAX_TICK;
        }

        self.udp_pinecone_network_conn_tick[nw_id].store(udp_pinecone_tick + 1, Ordering::Relaxed);
    }

    pub async fn udp_pinecone_reset_tick(&self, nw_id: usize) {
        self.udp_pinecone_network_conn_tick[nw_id].store(0, Ordering::Relaxed);
    }

    pub async fn is_tcp_server_pinecone_term_signal_available(&self) -> bool {
        let is_available;
        {
            is_available = self
                .is_tcp_server_termination_signal_got_nw_one
                .load(Ordering::Relaxed);
        }

        if is_available {
            self.is_tcp_server_termination_signal_got_nw_one
                .store(false, Ordering::Relaxed);
        }

        is_available
    }

    pub async fn send_tcp_server_pinecone_term_signal(&self) {
        self.is_tcp_server_termination_signal_got_nw_one
            .store(true, Ordering::Relaxed);
    }

    ///get tcp pinecone server ip
    pub async fn get_tcp_pinecone_dest_sock_addr(&self, nw_id: NwId) -> String {
        assert!(nw_id == NwId::Two);
        let tcp_pinecone_ip = self.tcp_dest_ip_nw_two.lock().await;
        let if_name = get_if2_name().unwrap();
        let tcp_pinecone_port = self.get_tcp_src_port_nw_one(nw_id).await;

        if is_if2_ipv4(){
            tcp_pinecone_ip.to_string() +":" + tcp_pinecone_port.to_string().as_str()
        }
        else{
            tcp_pinecone_ip.to_string() + "%" + if_name + ":" + tcp_pinecone_port.to_string().as_str()

        }
    }
    ///get tcp pinecone server ip
    pub async fn set_tcp_pinecone_dest_ip_addr(&self, nw_id: NwId, addr: IpAddr) {
        assert!(nw_id == NwId::Two);
        let mut tcp_pinecone_ip = self.tcp_dest_ip_nw_two.lock().await;
        *tcp_pinecone_ip = addr;
    }

    pub async fn add_new_tcp_conn_route(
        &self,
        recv_nw_id: NwId,
        route_info: PortIpPort,
        server_wr: TaskHandle,
        server_rd: TaskHandle,
        fwd_process_handle: TaskHandle,
        client_wr: TaskHandle,
        client_rd: TaskHandle,
    ) -> bool {
        let index = recv_nw_id as usize;
        let mut tcp_task_map = self.tcp_route_task_map[index].lock().await;

        match tcp_task_map.get_mut(&route_info) {
            None => {
                let route_task_handles = TcpFwdRouteTask {
                    server_wr,
                    server_rd,
                    fwd_process_handle,
                    client_wr,
                    client_rd,
                };
                tcp_task_map.insert(route_info, route_task_handles);
            }
            Some(_) => {
                tracing::error!(
                    "Route is already available and cannot be added,route info:{:?},nw id:{}",
                    route_info,
                    index
                );
                return false;
            }
        }
        true
    }

    pub async fn send_term_signal_for_tcproute(
        &self,
        recv_nw_id: NwId,
        route_info: PortIpPort,
    ) -> bool {
        let index = recv_nw_id as usize;
        let mut tcp_task_map = self.tcp_route_task_map[index].lock().await;

        match tcp_task_map.get_mut(&route_info) {
            None => {
                tracing::error!(
                    "Route is not available and cannot be terminated,route info:{:?},nw id:{}",
                    route_info,
                    index
                );
                return false;
            }
            Some(task_handles) => {
                let mut term_task_handles_vec = self.terminated_task_handles.lock().await;

                if let Some(handle) = &task_handles.server_wr {
                    term_task_handles_vec.push(handle.abort_handle());
                }
                if let Some(handle) = &task_handles.server_rd {
                    term_task_handles_vec.push(handle.abort_handle());
                }
                if let Some(handle) = &task_handles.fwd_process_handle {
                    term_task_handles_vec.push(handle.abort_handle());
                }
                if let Some(handle) = &task_handles.client_rd {
                    term_task_handles_vec.push(handle.abort_handle());
                }
                if let Some(handle) = &task_handles.client_wr {
                    term_task_handles_vec.push(handle.abort_handle());
                }

                tcp_task_map.remove(&route_info);
            }
        }

        true
    }

    pub async fn check_term_signal_tasks(&self) {
        let mut task_handles = self.terminated_task_handles.lock().await;

        if !task_handles.is_empty() {
            for task_handle in task_handles.iter() {
                task_handle.abort();
            }

            task_handles.clear();
        }
    }

    pub async fn send_term_signal_all_task_handles(&self) {
        for i in 0..TOTAL_NUM_NW {
            let tcp_task_map = self.tcp_route_task_map[i].lock().await;

            for (route, _) in tcp_task_map.iter() {
                self.send_term_signal_for_tcproute(i.into(), *route).await;
            }
        }

        let mut term_task_handles_vec = self.terminated_task_handles.lock().await;
        let tcp_pinecone_server_main_task_handle =
            self.tcp_pinecone_server_main_task_handle.lock().await;

        if let Some(handle) = tcp_pinecone_server_main_task_handle.deref() {
            term_task_handles_vec.push(handle.abort_handle());
        }
        tracing::info!("All task are terminated");
    }

    pub async fn update_tcp_pinecone_server_main_task_handle(&self, handle: TaskHandle) {
        let mut tcp_pinecone_server_main_task_handle =
            self.tcp_pinecone_server_main_task_handle.lock().await;
        *tcp_pinecone_server_main_task_handle = handle;
    }
}

#[cfg(test)]
mod tests {

    use crate::shared_state::SharedState;
    use crate::shared_state::*;
    use rand::Rng;
    use std::{
        net::{IpAddr, Ipv4Addr},
        ops::Deref,
    };
    use tracing_test::traced_test;
    #[tokio::test]
    async fn set_get_tcp_src_port_nw_one() {
        let state = SharedState::new().await;
        let port_num: u16 = 0x9f93;
        let mut udp_data_in = vec![0; UDP_PINECONE_PAYLOAD_SIZE];
        udp_data_in[UDP_PINECONE_PAYLOAD_SIZE - 2] = 0x9f;
        udp_data_in[UDP_PINECONE_PAYLOAD_SIZE - 1] = 0x93;

        state.set_tcp_src_port_nw_one(&udp_data_in);
        let port_num_result = state.get_tcp_src_port_nw_one(NwId::One).await;
        assert_eq!(port_num, port_num_result);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_1_first_data_insert() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;

        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port: 25212,
            nw_two_src_port: 233,
        };

        let new_data = vec![2, 5, 8, 9, 0, 2, 4];
        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        let tcp_con = state.tcp_con_in[NWID as usize].lock().await;
        println!("tcp_con key:{:?}", tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data),
            true
        );
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected, true);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_with_multiple_route_first_data_insert() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;
        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port: 25212,
            nw_two_src_port: 233,
        };

        let new_data = vec![2, 5, 8, 9, 0, 2, 4];

        let tcp_route_info_2 = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2)),
            nw_one_src_port: 100,
            nw_two_src_port: 400,
        };

        let new_data_2 = vec![1, 4, 2, 6, 2, 0, 5];

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info_2, new_data_2.clone())
            .await;

        let tcp_con = state.tcp_con_in[NWID as usize].lock().await;

        println!("tcp_con key:{:?}", tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data),
            true
        );
        assert_eq!(
            tcp_con
                .get(&tcp_route_info_2)
                .unwrap()
                .dataq
                .contains(&new_data_2),
            true
        );
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected, true);
        assert_eq!(tcp_con.get(&tcp_route_info_2).unwrap().is_connected, true);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_multiple_data_insert_for_same_route() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;

        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port: 25212,
            nw_two_src_port: 233,
        };

        let new_data = vec![2, 5, 8, 9, 0, 2, 4];
        let new_data_2 = vec![1, 2, 4, 2];

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data_2.clone())
            .await;

        let tcp_con = state.tcp_con_in[NWID as usize].lock().await;
        println!("tcp_con key:{:?}", tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data),
            true
        );
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data_2),
            true
        );
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected, true);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_two_1_first_data_insert() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::Two;
        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 15)),
            nw_one_src_port: 11,
            nw_two_src_port: 15,
        };

        let new_data = vec![1, 5, 24, 2];
        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        let tcp_con = state.tcp_con_in[NWID as usize].lock().await;
        println!("tcp_con key:{:?}", tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data),
            true
        );
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected, true);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_1_get_data() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;

        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port: 25212,
            nw_two_src_port: 233,
        };
        let new_data = vec![2, 5, 8, 9, 0, 2, 4];

        let got_data: Option<Vec<u8>> = state
            .get_tcp_incoming_data(NWID, tcp_route_info.clone())
            .await;
        assert_eq!(None, got_data);

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        let got_data: Option<Vec<u8>> = state.get_tcp_incoming_data(NWID, tcp_route_info).await;

        assert_eq!(new_data, got_data.unwrap());
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_multiple_get_data() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;

        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port: 25212,
            nw_two_src_port: 233,
        };
        let new_data = vec![2, 5, 8, 9, 0, 2, 4];
        let new_data_2 = vec![5, 3, 6, 2, 5, 2, 22, 4, 5, 231];
        let new_data_3 = vec![0, 3, 1, 9, 8, 7, 7, 9, 0, 4];

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data.clone())
            .await;

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data_2.clone())
            .await;

        state
            .insert_tcp_incoming_data(NWID, tcp_route_info, new_data_3.clone())
            .await;

        let got_data: Option<Vec<u8>> = state.get_tcp_incoming_data(NWID, tcp_route_info).await;
        assert_eq!(new_data, got_data.unwrap());
        let got_data: Option<Vec<u8>> = state.get_tcp_incoming_data(NWID, tcp_route_info).await;
        assert_eq!(new_data_2, got_data.unwrap());
        let got_data: Option<Vec<u8>> = state.get_tcp_incoming_data(NWID, tcp_route_info).await;
        assert_eq!(new_data_3, got_data.unwrap());
        let got_data: Option<Vec<u8>> = state.get_tcp_incoming_data(NWID, tcp_route_info).await;
        assert_eq!(None, got_data);
    }

    #[tokio::test]
    #[traced_test]
    async fn outgoing_tcp_data_to_nw_two_1_first_data_insert() {
        let state = SharedState::new().await;
        const NWID: NwId = NwId::One;
        let tcp_route_info = PortIpPort {
            nw_one_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 15)),
            nw_one_src_port: 11,
            nw_two_src_port: 15,
        };

        let new_data = vec![1, 5, 24, 2];
        state
            .insert_tcp_outgoing_data(NWID, tcp_route_info, new_data.clone())
            .await;

        let tcp_con = state.tcp_con_out[NWID as usize].lock().await;
        println!("tcp_con key:{:?}", tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(
            tcp_con
                .get(&tcp_route_info)
                .unwrap()
                .dataq
                .contains(&new_data),
            true
        );
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected, true);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_udp_pinecone_insert_data_to_nw1_with_udp_pinecone_payload_size() {
        let state = SharedState::new().await;
        const NWID: usize = 1;

        // Create a random number generator
        let mut rng = rand::thread_rng();
        let mut new_data = Vec::with_capacity(UDP_PINECONE_PAYLOAD_SIZE);
        // Fill the vector with random u8 values
        for _ in 0..UDP_PINECONE_PAYLOAD_SIZE {
            let random_value: u8 = rng.gen();
            new_data.push(random_value);
        }
        let new_data_clone = new_data.clone();

        let is_success = state
            .insert_udp_incoming_pinecone_data(NWID, new_data)
            .await;
        let udp_con = state.udp_pinecone_in[NWID].lock().await;
        assert!(is_success);
        assert_eq!(udp_con.as_ref(), new_data_clone);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_udp_pinecone_insert_data_to_nw1_with_diff_udp_pinecone_payload_size() {
        let state = SharedState::new().await;
        const NWID: usize = 1;
        let wrong_size = UDP_PINECONE_PAYLOAD_SIZE + 1;
        // Create a random number generator
        let mut rng = rand::thread_rng();
        let mut new_data = Vec::with_capacity(wrong_size);
        // Fill the vector with random u8 values
        for _ in 0..wrong_size {
            let random_value: u8 = rng.gen();
            new_data.push(random_value);
        }
        let new_data_clone = new_data.clone();

        let is_success = state
            .insert_udp_incoming_pinecone_data(NWID, new_data)
            .await;
        let udp_con = state.udp_pinecone_in[NWID].lock().await;
        assert!(!is_success);
        assert_ne!(udp_con.as_ref(), new_data_clone);
    }

    #[tokio::test]
    #[traced_test]
    #[should_panic]
    async fn incoming_udp_pinecone_insert_data_to_nw2() {
        let state = SharedState::new().await;
        const NWID: usize = 2;
        // Create a random number generator
        let mut rng = rand::thread_rng();
        let mut new_data = Vec::with_capacity(UDP_PINECONE_PAYLOAD_SIZE);
        // Fill the vector with random u8 values
        for _ in 0..UDP_PINECONE_PAYLOAD_SIZE {
            let random_value: u8 = rng.gen();
            new_data.push(random_value);
        }

        let _ = state
            .insert_udp_incoming_pinecone_data(NWID, new_data)
            .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_udp_pinecone_get_data_from_nw1_with_udp_pinecone_payload_size() {
        let state = SharedState::new().await;
        const NWID: usize = 1;
        // Create a random number generator
        let mut rng = rand::thread_rng();
        let mut new_data = Vec::with_capacity(UDP_PINECONE_PAYLOAD_SIZE);
        // Fill the vector with random u8 values
        for _ in 0..UDP_PINECONE_PAYLOAD_SIZE {
            let random_value: u8 = rng.gen();
            new_data.push(random_value);
        }

        {
            let mut udp_in = state.udp_pinecone_in[NWID].lock().await;
            *udp_in = new_data.clone();
        }

        let received_data = state.get_udp_incoming_pinecone_data(NWID).await;

        assert_eq!(received_data, Some(new_data));
        let received_data = state.get_udp_incoming_pinecone_data(NWID).await;
        assert_eq!(received_data, None);

        // println!("udp_in : {:?}",udp_in);
    }
}
