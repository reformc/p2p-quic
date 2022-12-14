use std::{
    net::{UdpSocket,SocketAddr,SocketAddrV4},
    collections::HashMap,
    sync::{Arc, Mutex}
};
use quinn;
use futures_util::{stream::StreamExt};
use tokio::{self, 
    sync::mpsc::{self,Sender,Receiver},
    sync::broadcast
};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    author="reform <reformgg@gmail.com>", 
    version="0.1.0",
    about="socks5 by p2p over quic"
)]
struct Args {
    /// 监听端口。
    #[clap(long,short,default_value = "3400")]
    port: u16
}

const HOST_NAME:&str = "reform";//要和cert-key-file里使用的一致


pub const CER:&[u8] = include_bytes!("G:/reform.cer");
pub const KEY:&[u8] = include_bytes!("G:/reform.key");

const KEEPALIVE_INTERVAL_MILLIS:u64=3000;
const IDLE_TIMEOUT_MILLIS:u32=10_000;


#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    server().await;
    //test().await;
    println!("Hello, world!");
}
/*
#[allow(unused)]
async fn test(){
    let socket = UdpSocket::bind(BIND_ADDR).expect("couldn't bind to address");
    let mut incoming = common::make_server_udp_endpoint(socket.try_clone().unwrap(),HOST_NAME,CER.to_vec(),KEY.to_vec()).unwrap();
    tokio::spawn(async move {
        let quinn::NewConnection { connection, .. } = incoming.next().await.unwrap().await.unwrap();
        println!(
            "[server] incoming connection: addr={},{}",
            connection.remote_address(),
            connection.stable_id()
        );
    });
    let client = common::make_client_udp_endpoint(socket, &[CER]).unwrap();
    let connect = client.connect("127.0.0.1:3400".parse().unwrap(), HOST_NAME).unwrap();
    let quinn::NewConnection { connection, .. } = connect.await.unwrap();
    println!("[client] connected: addr={}", connection.remote_address());
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}

*/
async fn server(){
    let args = Args::parse();
    let bind_addr = format!("0.0.0.0:{}",args.port.to_string());
    let socket = UdpSocket::bind(&bind_addr).expect("couldn't bind to address");
    let peers = Peers::new();
    let transport_config = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let mut incoming = common::make_server_udp_endpoint(socket.try_clone().unwrap(),HOST_NAME,CER.to_vec(),KEY.to_vec(),Some(transport_config)).unwrap();
    //tokio::spawn(client(socket));
    log::info!("listen on {}",&bind_addr);
    loop{
        match incoming.next().await {
        Some(conn)=>{
            //log::info!("{:?} incoming",conn.remote_address());
            let peers_cell = peers.clone();
            tokio::spawn(async move{handle_connection(conn,peers_cell).await});
        },
        None=>{continue;},
        };
    }
}

async fn handle_connection(conn: quinn::Connecting,peers:Peers){
    match conn.await{
        Ok(conn)=>{            
            let quinn::NewConnection {
                connection,
                mut bi_streams,
                ..
            } = conn;
            match connection.remote_address(){
                SocketAddr::V4(socketv4addr)=>{
                    //socketv4addr.ip().octets();
                    //socketv4addr.port().to_be_bytes();
                    log::debug!("{}",socketv4addr.to_string());
                },
                SocketAddr::V6(socketv6addr)=>{
                    log::debug!("{}",socketv6addr.to_string());
                }
            }
            log::info!("[92]{} incomming",connection.remote_address());
            loop{
                match bi_streams.next().await{
                    Some(stream)=>{
                        match stream{
                            Ok(stream)=>{
                                let connection_cell = connection.clone();
                                let peers_cell = peers.clone();
                                tokio::spawn(async move {handle_request(connection_cell,peers_cell,stream).await});
                            }
                            Err(e)=>{
                                log::debug!("{}",e);
                                return
                            }
                        }
                    },
                    None=>{
                        log::debug!("93{} close",connection.remote_address());
                    }
                }
            }
        },
        Err(e)=>{
            log::info!("{}",e);
        }
    }
}

async fn handle_request(connection:quinn::Connection,peers:Peers,stream: (quinn::SendStream, quinn::RecvStream))->Peer{  
    Peer::new(connection,peers,stream)
}

#[derive(Clone)]
struct Peers{
    peers:Arc<Mutex<HashMap<Vec<u8>,Peer>>>
}

impl Peers{
    fn new()->Peers{
        Peers { peers: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn reg(&self,id:Vec<u8>,peer:Peer)->Result<(),String>{
        let mut peers = self.peers.lock().unwrap();
        match (*peers).get(&id){
            Some(_)=>{Err("id already exists".to_string())},
            None=>{
                log::info!("{} online",common::msg::id_str(&id));
                (*peers).insert(id, peer);
                Ok(())
            }
        }
    }

    pub fn log_out(&self,id:&Vec<u8>){
        let mut peers = self.peers.lock().unwrap();
        (*peers).remove(id);
        log::info!("{} offline",common::msg::id_str(&id))
    }

    pub fn get_peer(&self,id:&Vec<u8>)->Option<Peer>{
        let peers = self.peers.lock().unwrap();
        match (*peers).get(id){
            Some(v)=>{Some(v.clone())},
            None=>{None}
        }
    }
}

#[derive(Clone)]
struct Id{
    id:Arc<Mutex<Vec<u8>>>
}

impl Id{
    fn new()->Id{
        Id{id:Arc::new(Mutex::new(Vec::new()))}
    }

    fn get(&self)->Vec<u8>{
        let id = self.id.lock().unwrap();
        (*id).clone()
    }

    fn set(&self,id_bytes:&Vec<u8>){
        let mut id = self.id.lock().unwrap();
        (*id)=id_bytes.clone();
    }
}


#[derive(Clone)]
struct Peer{
    id:Id,
    connection:quinn::Connection,
    self_sender:Sender<Vec<u8>>,
    peers:Peers
}

impl Peer{
    fn new(connection:quinn::Connection,peers:Peers,(send_stream, recv_stream): (quinn::SendStream, quinn::RecvStream))->Peer{
        let (self_sender,self_receiver) = mpsc::channel(100);
        let peer = Peer{id:Id::new(),connection,peers,self_sender};
        let (tx1, rx1) = broadcast::channel(2);
        let tx2 = tx1.clone();
        let rx2=tx1.subscribe();
        let peer1 = peer.clone();
        let peer2 = peer.clone();
        tokio::spawn(async move {peer2.sender_send(send_stream,self_receiver,tx1,rx1).await;});
        tokio::spawn(async move {peer1.reciver_recv(recv_stream,tx2,rx2).await;});        
        peer
        //Peer { id: (), addr: (), sender: (), dial_sender: (), offline_sender: () }
    }

    //向其他peer发起连接请求。
    async fn handshak_anthor(&self,id:&Vec<u8>)->Result<(),String>{
        let self_addr = self.socket_addr_v4()?;
        match self.peers.get_peer(id){
            Some(peer)=>{
                self.handshake_to_c(peer.handshake_to_s(self_addr).await?).await?;
                log::info!("{}: {} request {}: {}",common::msg::id_str(&self.id.get()),self_addr,common::msg::id_str(&id),peer.connection.remote_address());
            },
            None=>{
                self.self_sender.send(common::msg::Msg::Err("remote id does not exist".as_bytes().to_vec()).body()).await.unwrap();
                let err_msg = format!("{}: {} request fail remote id {} does not exist",common::msg::id_str(&self.id.get()),self_addr,common::msg::id_str(&id));
                log::info!("{}",err_msg);
                return Err(err_msg)
            }
        }
        Ok(())
    }
    
    async fn handshake_to_s(&self,addr:SocketAddrV4)->Result<SocketAddrV4,String>{
        /*let mut bytes = vec!();
        let mut addr_bytes = common::addr::socketaddrv4_to_bytes(addr);
        bytes.push(addr_bytes.len() as u8 + 1);
        bytes.push(MSG_HAN_S);
        bytes.append(&mut addr_bytes);*/
        log::info!("han_s{:?}",common::msg::Msg::HanS(addr).body());
        match self.self_sender.send(common::msg::Msg::HanS(addr).body()).await{
            Ok(_)=>{},
            Err(e)=>{return Err(format!("{}",e))}
        };
        self.socket_addr_v4()
    }
    async fn handshake_to_c(&self,addr:SocketAddrV4)->Result<(),String>{
        /*let mut bytes = vec!();
        let mut addr_bytes = common::addr::socketaddrv4_to_bytes(addr);
        bytes.push(addr_bytes.len() as u8 + 1);
        bytes.push(MSG_HAN_C);
        bytes.append(&mut addr_bytes);*/
        log::info!("han_c{:?}",common::msg::Msg::HanC(addr).body());
        match self.self_sender.send(common::msg::Msg::HanC(addr).body()).await{
            Ok(_)=>{},
            Err(e)=>{return Err(format!("{}",e))}
        };
        Ok(())
    }

    fn socket_addr_v4(&self)->Result<SocketAddrV4,String>{
        match self.connection.remote_address(){
            SocketAddr::V4(self_addr)=>{Ok(self_addr)},
            SocketAddr::V6(_)=>{Err("not ipv4 addr".to_string())}
        }
    }
    
    async fn sender_send(&self,mut send_stream:quinn::SendStream,mut receive_channel:Receiver<Vec<u8>>,close_send:broadcast::Sender<bool>,mut close_receive:broadcast::Receiver<bool>){//向peer发送信息
        loop{
        tokio::select! {
                msg =receive_channel.recv()=>{
                    match msg{
                        Some(v)=>{
                            send_stream.write_all(&v).await.unwrap();
                        },
                        None=>{
                            close_send.send(false).unwrap();
                            return;
                        }
                    }
                },
                _=close_receive.recv()=>{
                return
                }
            
            }
        }
    }

    async fn reciver_recv(&self,mut recv_stream:quinn::RecvStream,close_send:broadcast::Sender<bool>,mut close_receive:broadcast::Receiver<bool>){//接收peer的信息
        let mut buffer = [0; 64*1024];
        let close_send1 = close_send.clone();
        let _p = CallOnDrop(move||{
            log::debug!("{} return",common::msg::id_str(&self.id.get()));
            close_send1.send(false).unwrap();
        });
        loop{
            tokio::select! {
                result = recv_stream.read(&mut buffer) => match result {
                    Ok(msg) => {
                        match msg{
                            Some(len)=>{
                                if len <2{
                                    log::debug!("lenth is less than 2");
                                    continue
                                }
                                log::debug!("recv:len:{},type:{},body:{}",buffer[0],buffer[1],std::str::from_utf8(&buffer[2..len]).unwrap_or("not utf8"));
                                match buffer[1]{
                                    common::msg::MSG_REG=>{
                                        match self.peers.reg(buffer[2..len].to_vec(),self.clone()){
                                            Ok(_)=>{
                                                self.id.set(&buffer[2..len].to_vec());
                                                let mut log_out_channel = close_send.subscribe();
                                                let peer_close = self.clone();
                                                let id = buffer[2..len].to_vec();
                                                tokio::spawn(async move{
                                                    match log_out_channel.recv().await{
                                                        Ok(_)=>{
                                                            peer_close.peers.log_out(&id);
                                                        }
                                                        Err(e)=>{
                                                            log::debug!("{:?} close channel receive Err:{}",common::msg::id_str(&id),e);
                                                            return
                                                        }
                                                    }
                                                });
                                            },
                                            Err(str)=>{self.self_sender.send(str.into_bytes()).await.unwrap();}
                                        };
                                    }//注册
                                    common::msg::MSG_REQ=>{
                                        match self.handshak_anthor(&buffer[2..len].to_vec()).await{
                                            Ok(_)=>{log::info!("success request,from:{:?},to:{:?}",common::msg::id_str(&self.id.get()),common::msg::id_str(&buffer[2..len]))},
                                            Err(str)=>{log::info!("fail request,from:{:?},to:{:?},errmsg:{}",common::msg::id_str(&self.id.get()),common::msg::id_str(&buffer[2..len]),str)}
                                        };
                                    },
                                    common::msg::MSG_KEEPALIVE=>{
                                        log::debug!("{} keepalive",common::msg::id_str(&buffer[2..len]));
                                    }
                                    _ =>{}
                                }
                            },
                            None=> {//流关闭
                                log::debug!("recv_stream is close,addr:{},id:{}",self.connection.remote_address(),recv_stream.id());
                                return
                            }
                        }
                    },
                    Err(e) =>{
                        log::debug!("read recv stream {} err:{}",self.connection.remote_address(),e);
                        return
                    },
                },
                _ = close_receive.recv()=>{
                    log::debug!("close receive");
                    return
                }
            }
        }
    }    

}

pub struct CallOnDrop<F: Fn()>(F);

impl <F: Fn()>Drop for CallOnDrop<F>{
    fn drop(&mut self){
        (self.0)();
    }
}