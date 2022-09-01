use std::{
    net::{UdpSocket,SocketAddr,SocketAddrV4},
    collections::HashMap,
    sync::{Arc, Mutex}
};
use futures_util::{stream::StreamExt, AsyncWriteExt};
use common;
use log;
use clap::Parser;

//const SERVER_ADDR:&str = "127.0.0.1:3400";
//const BIND_ADDR:&str = "0.0.0.0:3401";
const HOST_NAME:&str = "OK";//要和cert-key-file里使用的一致

const CER:&[u8] = include_bytes!("G:/test.cer");
const KEY:&[u8] = include_bytes!("G:/test.key");

const KEEPALIVE_INTERVAL_MILLIS:u64=3000;
const IDLE_TIMEOUT_MILLIS:u32=10_000;

//const PEER_ID:&str = "reform";

const RECV_TIMEOUT:u64=10;

#[derive(Parser, Debug)]
#[clap(
    author="reform <reformgg@gmail.com>", 
    version="0.1.0",
    about="p2p打洞",
    long_about="p2p打洞端口代理,底层使用quic协议传输,端口代理使用socks5协议。"
)]
struct Args{
    ///服务器地址
    #[clap(long,short,default_value = "127.0.0.1:3400")]
    server:String,
    ///本机quic端口
    #[clap(long,short,default_value = "3401")]
    quic_port:u16,
    ///本机id
    #[clap(long,short,default_value = "reform")]
    id:String,    
    ///对方id
    #[clap(long,short,default_value = "none")]
    another:String
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    client().await;
    println!("Hello, world!");
}

async fn client(){
    let args = Args::parse();
    let socket = UdpSocket::bind(format!("0.0.0.0:{}",&args.quic_port)).expect("couldn't bind to address");    
    let transport_config_server = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let mut incoming = common::make_server_udp_endpoint(socket.try_clone().unwrap(),HOST_NAME,CER.to_vec(),KEY.to_vec(),Some(transport_config_server)).unwrap();
    tokio::spawn(async move {
        let quinn::NewConnection { connection, .. } = incoming.next().await.unwrap().await.unwrap();
        println!(
            "[server] incoming connection: addr={},{}",
            connection.remote_address(),
            connection.stable_id()
        );
    });
    let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let peer_client = common::make_client_udp_endpoint(socket, &[CER],Some(transport_config_client)).unwrap();
    let peer_client_connect = peer_client.connect(args.server.parse().unwrap(), HOST_NAME).unwrap();
    let quinn::NewConnection { connection,.. } = peer_client_connect.await.unwrap();
    log::info!("[client] connected: addr={}", connection.remote_address());
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //let stream = bi_streams.next().await.unwrap().unwrap();
    let bi_stream = connection.open_bi().await.unwrap();
    handle_peer_client_connect(&args.id,bi_stream).await;
    connection.close(quinn::VarInt::MAX, quinn::ConnectionError::LocallyClosed.to_string().as_bytes())
}



async fn handle_peer_client_connect(id:&str,(mut send_stream, recv_stream): (quinn::SendStream, quinn::RecvStream)){
    /*
    let mut buffer = [0; 64*1024];
    loop{
        tokio::select! {
            result = recv_stream.read(&mut buffer) => match result {
                Ok(msg) => {
                    match msg{
                        Some(len)=>{
                            if len <2{
                                continue
                            }
                            match buffer[1]{
                                common::msg::MSG_HAN_S=>{
                                    send_stream.write_all(&b).await.unwrap();
                                }//注册
                                common::msg::MSG_HAN_C=>{
                                    match self.handshak_anthor(&buffer[2..len].to_vec()).await{
                                        Ok(_)=>{},
                                        Err(str)=>{println!("{}",str)}
                                    };
                                },
                                common::msg::MSG_KEEPALIVE=>{}
                                _ =>{}
                            }
                        },
                        None=> return
                    }
                },
                Err(_) =>return,
            },
            //_ = close_receive.recv()=>{
            //    return
            
            _ = tokio::time::sleep(std::time::Duration::from_secs(RECV_TIMEOUT)) => {
                //close_send.send(false).unwrap();
                println!("reading timeout {} s",RECV_TIMEOUT);
            }
        }
    }
    */
  
    let b = common::msg::Msg::Reg(id.as_bytes().to_vec()).body();
    send_stream.write_all(&b).await.unwrap();  
    log::info!("id:{:?}",b); 
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //send_stream.finish().await.unwrap(); 
    //send_stream.write_all(&b).await.unwrap();  

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}

async fn handle_peer_client_connect_sendstream(){

}