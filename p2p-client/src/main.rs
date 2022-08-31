use std::{
    net::{UdpSocket,SocketAddr,SocketAddrV4},
    collections::HashMap,
    sync::{Arc, Mutex}
};
use futures_util::{stream::StreamExt};
use common;
use log;

const SERVER_ADDR:&str = "127.0.0.1:3400";
const BIND_ADDR:&str = "0.0.0.0:3401";
const HOST_NAME:&str = "OK";//要和cert-key-file里使用的一致

const CER:&[u8] = include_bytes!("G:/test.cer");
const KEY:&[u8] = include_bytes!("G:/test.key");

const PEER_ID:&str = "reform";

const RECV_TIMEOUT:u64=10;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    client().await;
    println!("Hello, world!");
}

async fn client(){
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
    
    let peer_client = common::make_client_udp_endpoint(socket, &[CER]).unwrap();
    let peer_client_connect = peer_client.connect(SERVER_ADDR.parse().unwrap(), HOST_NAME).unwrap();
    let quinn::NewConnection { connection,.. } = peer_client_connect.await.unwrap();    
    log::info!("[client] connected: addr={}", connection.remote_address());
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //let stream = bi_streams.next().await.unwrap().unwrap();
    let bi_stream = connection.open_bi().await.unwrap();
    handle_peer_client_connect(bi_stream).await;

}

async fn handle_peer_client_connect((mut send_stream, recv_stream): (quinn::SendStream, quinn::RecvStream)){
    let b = common::msg::Msg::Reg(PEER_ID.as_bytes().to_vec()).body();
    send_stream.write_all(&b).await.unwrap();  
    log::info!("id:{:?}",b);  
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}