use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};
use std::net::IpAddr;


#[derive(Eq, Hash, PartialEq,Debug,Clone)]
struct PortIpPort{

   pub nw_one_ip:IpAddr,
   pub nw_one_src_port:u16,
   pub nw_two_src_port:u16,
}



type DataqT=VecDeque<Vec<u8>>;

#[derive(Debug,PartialEq)]
struct TcpData{
    dataq:DataqT,
    is_connected:bool
}



///tcp forwarding route map for incoming packets
/// key: (network 1 ip,network 1 src port num,network 2 src port num)
/// value : tcp data
type TcpFwdRouteMap=HashMap<PortIpPort,TcpData>;





#[derive(Debug, Clone)]
pub struct SharedState{
    /// tcp incoming data and route info from network one 
    tcp_con_in_nw_one:Arc<Mutex<TcpFwdRouteMap>>,

    /// tcp outgoing data and route info to network one 
    tcp_con_out_nw_one:Arc<Mutex<TcpFwdRouteMap>>,

    /// tcp incoming data and route info from network two 
    tcp_con_in_nw_two:Arc<Mutex<TcpFwdRouteMap>>,

    /// tcp outgoing data and route info to network two 
    tcp_con_out_nw_two:Arc<Mutex<TcpFwdRouteMap>>,
    /// tcp server port for network one
    tcp_src_port_nw_one:Arc<AtomicU16>,
    /// udp pinecone incoming packets from network two
    udp_pinecone_incoming_data_nw_two:Arc<Mutex<DataqT>>,
    //  udp pinecone outgoing packets to network one 
    udp_pinecone_outgoing_data_nw_one:Arc<Mutex<DataqT>>
}



impl SharedState{

    pub async fn new() -> Self {
        Self {
            tcp_con_in_nw_one: Arc::new(Mutex::new(HashMap::new())),
            tcp_con_out_nw_one: Arc::new(Mutex::new(HashMap::new())),
            tcp_con_in_nw_two: Arc::new(Mutex::new(HashMap::new())),
            tcp_con_out_nw_two: Arc::new(Mutex::new(HashMap::new())),
            tcp_src_port_nw_one: Arc::new(AtomicU16::new(0)),
            udp_pinecone_incoming_data_nw_two: Arc::new(Mutex::new(VecDeque::new())),
            udp_pinecone_outgoing_data_nw_one: Arc::new(Mutex::new(VecDeque::new()))
        }
    }
   
    pub async fn insert_tcp_incoming_data(
        &self,
        nw_id:u8,
        nw_one_ip: IpAddr,
        nw_one_src_port:u16,
        nw_two_src_port:u16,
        in_data:Vec<u8> 
    ) -> bool {

        let key: PortIpPort = PortIpPort{
            nw_one_ip,
            nw_one_src_port,
            nw_two_src_port,
        };

        let mut tcp_con = if nw_id == 1 {
            self.tcp_con_in_nw_one.lock().await
        } else {
            self.tcp_con_in_nw_two.lock().await
        };


        match tcp_con.get_mut(&key) {
            None => {

                let mut new_dataq=VecDeque::new();
                new_dataq.push_back(in_data);
                let in_data= TcpData{

                    dataq:new_dataq,
                    is_connected:true
                };
                
               if tcp_con.insert(key, in_data).is_none(){
                    tracing::trace!("First data is added to shared state,nw_id:{}",nw_id);
               }
               
               
            }
            Some(new_message) => {
                new_message.dataq.push_back(in_data);
            }
        }
        true
    }


    pub async fn get_tcp_incoming_data(
        &self,
        nw_id:u8,
        nw_one_ip: IpAddr,
        nw_one_src_port:u16,
        nw_two_src_port:u16
    ) -> Option<Vec<u8>> {

        let key: PortIpPort = PortIpPort{
            nw_one_ip,
            nw_one_src_port,
            nw_two_src_port,
        };

        let mut tcp_con = if nw_id == 1 {
            self.tcp_con_in_nw_one.lock().await
        } else {
            self.tcp_con_in_nw_two.lock().await
        };

        if let Some(new_message)= tcp_con.get_mut(&key)  {
          
                if !new_message.dataq.is_empty(){

                   return new_message.dataq.pop_front();
                  
                }     
                
        }

        tracing::error!("There is no incoming data ->nw_id:{},route_info:{:?}",nw_id,key);
        None
    }



    // Get the tcp source port number field
    pub async fn get_tcp_src_port_nw_one(&self) -> u16 {
        self.tcp_src_port_nw_one.load(Ordering::Relaxed)
    }    
 
    // Set the tcp source port number field
    pub async fn set_tcp_src_port_nw_one(&self, port_num: u16) {
        self.tcp_src_port_nw_one.store(port_num, Ordering::Relaxed)
    }

}



#[cfg(test)]
mod tests {

    use crate::shared_state::SharedState;
    use std::net::{IpAddr,Ipv4Addr};
    use crate::shared_state::*;
    use tracing_test::traced_test;
    #[tokio::test]
    async fn set_get_tcp_src_port_nw_one() {
        let state = SharedState::new().await;

       state.set_tcp_src_port_nw_one(12).await;
       let port_num=state.get_tcp_src_port_nw_one().await;
        assert_eq!(port_num, 12);
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_1_first_data_insert(){
        let state = SharedState::new().await;
        
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port:25212,
            nw_two_src_port:233
        };

        let new_data=vec![2,5,8,9,0,2,4];
        state.insert_tcp_incoming_data(1, 
                                       tcp_route_info.nw_one_ip, 
                                       tcp_route_info.nw_one_src_port, 
                                       tcp_route_info.nw_two_src_port, 
                                       new_data.clone()).await;
        
        let tcp_con = state.tcp_con_in_nw_one.lock().await;
        println!("tcp_con key:{:?}",tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().dataq.contains(&new_data),true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected,true);   
    }   


    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_and_two_multiple_first_data_insert(){
        let state = SharedState::new().await;
        
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port:25212,
            nw_two_src_port:233
        };

        let new_data=vec![2,5,8,9,0,2,4];

        let tcp_route_info_2=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2)),
            nw_one_src_port:100,
            nw_two_src_port:400
        };

        let new_data_2=vec![1,4,2,6,2,0,5];

        state.insert_tcp_incoming_data(1, 
                                       tcp_route_info.nw_one_ip, 
                                       tcp_route_info.nw_one_src_port, 
                                       tcp_route_info.nw_two_src_port, 
                                       new_data.clone()).await;
        
        state.insert_tcp_incoming_data(1, 
                                        tcp_route_info_2.nw_one_ip, 
                                        tcp_route_info_2.nw_one_src_port, 
                                        tcp_route_info_2.nw_two_src_port, 
                                        new_data_2.clone()).await;
        
        let tcp_con = state.tcp_con_in_nw_one.lock().await;

        println!("tcp_con key:{:?}",tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().dataq.contains(&new_data),true);
        assert_eq!(tcp_con.get(&tcp_route_info_2).unwrap().dataq.contains(&new_data_2),true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected,true);   
        assert_eq!(tcp_con.get(&tcp_route_info_2).unwrap().is_connected,true);   

    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_multiple_data_insert_for_same_route(){
        let state = SharedState::new().await;
        
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port:25212,
            nw_two_src_port:233
        };

        let new_data=vec![2,5,8,9,0,2,4];
        let new_data_2=vec![1,2,4,2];

        state.insert_tcp_incoming_data(1, 
                                       tcp_route_info.nw_one_ip, 
                                       tcp_route_info.nw_one_src_port, 
                                       tcp_route_info.nw_two_src_port, 
                                       new_data.clone()).await;
        
        state.insert_tcp_incoming_data(1, 
                                        tcp_route_info.nw_one_ip, 
                                        tcp_route_info.nw_one_src_port, 
                                        tcp_route_info.nw_two_src_port, 
                                        new_data_2.clone()).await;
        
        let tcp_con = state.tcp_con_in_nw_one.lock().await;
        println!("tcp_con key:{:?}",tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().dataq.contains(&new_data),true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().dataq.contains(&new_data_2),true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected,true);
    }



    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_two_1_first_data_insert(){
        let state = SharedState::new().await;
        
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 10, 15)),
            nw_one_src_port:11,
            nw_two_src_port:15
        };

        let new_data=vec![1,5,24,2];
        state.insert_tcp_incoming_data(2, 
                                       tcp_route_info.nw_one_ip, 
                                       tcp_route_info.nw_one_src_port, 
                                       tcp_route_info.nw_two_src_port, 
                                       new_data.clone()).await;
        
        let tcp_con = state.tcp_con_in_nw_two.lock().await;
        println!("tcp_con key:{:?}",tcp_con);
        assert_eq!(tcp_con.contains_key(&tcp_route_info), true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().dataq.contains(&new_data),true);
        assert_eq!(tcp_con.get(&tcp_route_info).unwrap().is_connected,true);
    }   

    #[tokio::test]
    #[traced_test]

    async fn incoming_tcp_data_to_nw_one_1_get_data(){
        let state = SharedState::new().await;
       
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port:25212,
            nw_two_src_port:233
        };
        let new_data=vec![2,5,8,9,0,2,4];

        let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.clone().nw_one_ip,  tcp_route_info.clone().nw_one_src_port,  tcp_route_info.clone().nw_two_src_port).await;
        assert_eq!(None,got_data);

        state.insert_tcp_incoming_data(1, 
            tcp_route_info.clone().nw_one_ip, 
            tcp_route_info.clone().nw_one_src_port, 
            tcp_route_info.clone().nw_two_src_port, 
            new_data.clone()).await;
        
         let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.nw_one_ip,  tcp_route_info.nw_one_src_port,  tcp_route_info.nw_two_src_port).await;

         assert_eq!(new_data,got_data.unwrap());
    }

    #[tokio::test]
    #[traced_test]
    async fn incoming_tcp_data_to_nw_one_multiple_get_data(){
        let state = SharedState::new().await;
       
        let tcp_route_info=PortIpPort{
            nw_one_ip:IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            nw_one_src_port:25212,
            nw_two_src_port:233
        };
        let new_data=vec![2,5,8,9,0,2,4];
        let new_data_2=vec![5,3,6,2,5,2,22,4,5,231];
        let new_data_3=vec![0,3,1,9,8,7,7,9,0,4];

        state.insert_tcp_incoming_data(1, 
            tcp_route_info.clone().nw_one_ip, 
            tcp_route_info.clone().nw_one_src_port, 
            tcp_route_info.clone().nw_two_src_port, 
            new_data.clone()).await;

            state.insert_tcp_incoming_data(1, 
                tcp_route_info.clone().nw_one_ip, 
                tcp_route_info.clone().nw_one_src_port, 
                tcp_route_info.clone().nw_two_src_port, 
                new_data_2.clone()).await;

          state.insert_tcp_incoming_data(1, 
            tcp_route_info.clone().nw_one_ip, 
            tcp_route_info.clone().nw_one_src_port, 
            tcp_route_info.clone().nw_two_src_port, 
            new_data_3.clone()).await;
            
        
         let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.clone().nw_one_ip,  tcp_route_info.clone().nw_one_src_port,  tcp_route_info.clone().nw_two_src_port).await; 
         assert_eq!(new_data,got_data.unwrap());
         let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.clone().nw_one_ip,  tcp_route_info.clone().nw_one_src_port,  tcp_route_info.clone().nw_two_src_port).await; 
         assert_eq!(new_data_2,got_data.unwrap());
         let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.clone().nw_one_ip,  tcp_route_info.clone().nw_one_src_port,  tcp_route_info.clone().nw_two_src_port).await; 
         assert_eq!(new_data_3,got_data.unwrap());
         let got_data: Option<Vec<u8>>  = state.get_tcp_incoming_data(1, tcp_route_info.clone().nw_one_ip,  tcp_route_info.clone().nw_one_src_port,  tcp_route_info.clone().nw_two_src_port).await; 
         assert_eq!(None,got_data);
    }

}



