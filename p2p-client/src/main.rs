use std::net::{UdpSocket,SocketAddr};
use futures_util::stream::StreamExt;
use common;
use clap::Parser;
use tokio::net::TcpStream;

//const SERVER_ADDR:&str = "127.0.0.1:3400";
//const BIND_ADDR:&str = "0.0.0.0:3401";
const HOST_NAME:&str = "OK";//要和cert-key-file里使用的一致

const CER:&[u8] = include_bytes!("G:/test.cer");
const KEY:&[u8] = include_bytes!("G:/test.key");

const KEEPALIVE_INTERVAL_MILLIS:u64=3000;
const IDLE_TIMEOUT_MILLIS:u32=10_000;

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
    #[clap(long,short,default_value = "9361")]
    quic_port:u16,
    ///本机id
    #[clap(long,short,default_value = "default")]
    id:String,    
    ///对方id
    #[clap(long,short,default_value = "")]
    another:String,
    ///socks5server端口
    #[clap(long,default_value = "9362")]
    socks5_server:u16,
    ///socks5client端口
    #[clap(long,default_value = "9363")]
    socks5_client:u16
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    client().await;
    println!("Hello, world!");
}

async fn client(){
    let mut args = Args::parse();
    if args.another == args.id{
        panic!("you cant not connect to yourself!");
    }
    match &args.id as &str{
        "default"=>{
            args.id = nanoid::nanoid!(9);
        },
        _ =>{}
    }
    tokio::spawn(common::socsk5_server::spawn_socks_server(args.socks5_server));
    let bind_addr = format!("0.0.0.0:{}",&args.quic_port);
    log::info!("\nbind:\t{}\nserver:\t{}\nlocal:\t{}",bind_addr,args.server,common::msg::id_str(args.id.as_bytes()));
    let socket = UdpSocket::bind(&bind_addr).expect("couldn't bind to address");
    drop(bind_addr);
    let transport_config_server = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let incoming = common::make_server_udp_endpoint(socket.try_clone().unwrap(),HOST_NAME,CER.to_vec(),KEY.to_vec(),Some(transport_config_server)).unwrap();
    tokio::spawn(async move{server(args.socks5_server,incoming).await;});
    loop{
    let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let peer_client = common::make_client_udp_endpoint(socket.try_clone().unwrap(), &[CER],Some(transport_config_client)).unwrap();
    let peer_client_connect = peer_client.connect(args.server.parse().unwrap(), HOST_NAME).unwrap();
    let quinn::NewConnection { connection,.. } = peer_client_connect.await.unwrap();
    log::info!("[client] connected: addr={}", connection.remote_address());
    //tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //let stream = bi_streams.next().await.unwrap().unwrap();
    let bi_stream = connection.open_bi().await.unwrap();
    handle_peer_client_connect(args.socks5_client,&args.id,&args.another,socket.try_clone().unwrap(),bi_stream).await;
    log::info!("handle over");
    tokio::time::sleep(tokio::time::Duration::from_secs(50)).await;
    //connection.close(quinn::VarInt::MAX, quinn::ConnectionError::LocallyClosed.to_string().as_bytes())
    }
}



async fn handle_peer_client_connect(client_port:u16,self_id:&str,another_id:&str,socket:UdpSocket,(mut send_stream, mut recv_stream): (quinn::SendStream, quinn::RecvStream)){  
    send_stream.write_all(&common::msg::Msg::Reg(self_id.as_bytes().to_vec()).body()).await.unwrap();
    if another_id ==""{
        let self_id = self_id.to_string();
        tokio::spawn(async move{handle_peer_client_connect_send_keepalive(&self_id, send_stream).await;});
    }else{
        send_stream.write_all(&common::msg::Msg::Req(another_id.as_bytes().to_vec()).body()).await.unwrap();
    }
    let mut buffer = [0; 64*1024];
    loop{
        let result = recv_stream .read(&mut buffer).await; match result {
            Ok(msg) => {
                match msg{                        
                    Some(len)=>{
                        if len <2{
                            continue
                        }
                        match buffer[1]{
                            common::msg::MSG_HAN_S=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                log::info!("receive a request,another addr:{}",addr.to_string());
                                handshake_s(socket.try_clone().unwrap(),SocketAddr::V4(addr)).await;
                            }
                            common::msg::MSG_HAN_C=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                let socket_c = socket.try_clone().unwrap();
                                tokio::spawn(async move{transnation_client(client_port,socket_c, SocketAddr::V4(addr)).await;});
                                continue
                            },
                            common::msg::MSG_ERR=>{
                                log::info!("server_recv_stream read err:{}",common::msg::id_str(&buffer[2..len]));
                                break;
                            }
                            _ =>{}
                        }
                    },
                    None=> log::debug!("server close")
                    }
                },
            Err(e) =>{
                log::info!("{}",e);
                //common::msg::Msg::Err("server recv_stream read err".as_bytes().to_vec())
            }
        };
    }    
}

async fn handle_peer_client_connect_send_keepalive(self_id:&str,mut send_stream:quinn::SendStream){
    loop{
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        match send_stream.write_all(&common::msg::Msg::Keepalive(self_id.as_bytes().to_vec()).body()).await{
            Ok(_)=>{},
            Err(e)=>{
                log::debug!("{e}");
                return;
            }
        };
    }
}
/*
async fn handle_peer_client_connect_recv_stream(socket:UdpSocket,mut recv_stream:quinn::RecvStream){
    let mut buffer = [0; 64*1024];
    log::info!("recv_stream");
    loop{
        let result = recv_stream.read(&mut buffer).await; match result {
            Ok(msg) => {
                match msg{                        
                    Some(len)=>{
                        if len <2{
                            continue
                        }
                        match buffer[1]{
                            common::msg::MSG_HAN_S=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                log::info!("receive a request,another addr:{}",addr.to_string());
                                handshake_s(socket.try_clone().unwrap(),SocketAddr::V4(addr)).await;
                            }//注册
                            common::msg::MSG_HAN_C=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                let socket_c = socket.try_clone().unwrap();
                                tokio::spawn(async move{transnation_client(socket_c, SocketAddr::V4(addr)).await;});
                                break
                                //log::info!("request receive the response,another addr:{}",addr.to_string());
                            },
                            //common::msg::MSG_KEEPALIVE=>{},
                            common::msg::MSG_ERR=>{
                                log::debug!("server_recv_stream read err{}",common::msg::id_str(&buffer[2..len]));
                                break;
                            }
                            _ =>{}
                        }
                    },
                    None=> log::debug!("server close")
                    }                
                },
            Err(e) =>{
                log::info!("{}",e);
                //common::msg::Msg::Err("server recv_stream read err".as_bytes().to_vec())
            }
        };
    }    
    //common::msg::Msg::Err("server close".as_bytes().to_vec())
}
*/
async fn handshake_s(socket:UdpSocket,addr:SocketAddr){
    tokio::spawn(async move {
        let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
        let client = common::make_client_udp_endpoint(socket, &[CER],Some(transport_config_client)).unwrap();
        let conn = client.connect(addr,HOST_NAME).unwrap().await;
        match conn{
            Ok(quinn::NewConnection { connection: _,.. })=>{
                log::info!("handshake to the request {} success",addr.to_string());
            },
            Err(e)=>{log::info!("handshake to the request {} err:{}",addr.to_string(),e)}
        }
    });
}

async fn transnation_client(client_port:u16,socket:UdpSocket,addr:SocketAddr){
    let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let client = common::make_client_udp_endpoint(socket, &[CER],Some(transport_config_client)).unwrap();
    let peer_client_connect = client.connect(addr,HOST_NAME).unwrap();    
    let quinn::NewConnection { connection,.. } = peer_client_connect.await.unwrap();
    let (mut send_stream,_) = connection.open_bi().await.unwrap();
    tokio::spawn(async move{
        loop{
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            send_stream.write_all("keepalive".as_bytes()).await.unwrap();
        }
    });
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}",client_port)).await.unwrap();
    while let Ok((tcpstream,_))=listener.accept().await{
        let connection_cell = connection.clone();
        tokio::spawn(async move{
            let (mut read_tcp,mut write_tcp) = tcpstream.into_split();
            let (mut send_quic,mut recv_quic)= connection_cell.open_bi().await.unwrap();
            tokio::join!(
                async move{tokio::io::copy(&mut read_tcp, &mut send_quic).await.unwrap()},
                async move{tokio::io::copy(&mut recv_quic, &mut write_tcp).await.unwrap()}
            );
        });
    };
}

async fn server(socks5_server_port:u16,mut incoming:quinn::Incoming){
    loop{
        match incoming.next().await {
        Some(conn)=>{
            //log::info!("{:?} incoming",conn.remote_address());
            tokio::spawn(async move{handle_connection(socks5_server_port,conn).await});
        },
        None=>{continue;},
        };
    }
}


async fn handle_connection(socks5_server_port:u16,conn: quinn::Connecting){
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
            log::info!("{} incomming",connection.remote_address());
            loop{
                match bi_streams.next().await{
                    Some(stream)=>{
                        match stream{
                            Ok(stream)=>{
                                tokio::spawn(async move {handle_request(socks5_server_port,stream).await});
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

async fn handle_request(socks5_server_port:u16,stream: (quinn::SendStream, quinn::RecvStream)){  
    let conn = TcpStream::connect(format!("127.0.0.1:{}",socks5_server_port)).await.unwrap();
    let (mut read_tcp, mut write_tcp) = conn.into_split();
    let (mut send_quic,mut recv_quic): (quinn::SendStream, quinn::RecvStream) = stream;
    tokio::spawn(async move{
        tokio::io::copy(&mut read_tcp,&mut send_quic).await.unwrap_or(0);
    });
    tokio::spawn(async move{
        tokio::io::copy(&mut recv_quic,&mut write_tcp).await.unwrap_or(0);
    });
}
