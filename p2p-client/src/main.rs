use std::{
    net::{UdpSocket,SocketAddr,SocketAddrV4},
    collections::HashMap,
    sync::{Arc, Mutex}
};
use futures_util::{stream::StreamExt};
use common;

const BIND_ADDR:&str = "0.0.0.0:3400";
const HOST_NAME:&str = "OK";//要和cert-key-file里使用的一致

const CER:&[u8] = include_bytes!("G:/test.cer");
const KEY:&[u8] = include_bytes!("G:/test.key");

const RECV_TIMEOUT:u64=10;


fn main() {
    println!("Hello, world!");
}

fn client(){
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
}