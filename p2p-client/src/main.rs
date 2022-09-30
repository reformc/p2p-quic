use std::{net::{UdpSocket,SocketAddr}};
use futures_util::stream::StreamExt;
use common;
use clap::Parser;
use tokio::net::TcpStream;
use std::error::Error;

//const SERVER_ADDR:&str = "127.0.0.1:3400";
//const BIND_ADDR:&str = "0.0.0.0:3401";
const HOST_NAME:&str = "reform";//要和cert-key-file里使用的一致

const CER:&[u8] = include_bytes!("G:/reform.cer");
const KEY:&[u8] = include_bytes!("G:/reform.key");

const KEEPALIVE_INTERVAL_MILLIS:u64=5000;
const IDLE_TIMEOUT_MILLIS:u32=15_000;//秒

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
    //common::socsk5_server::spawn_socks_server(5898).await.unwrap();
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
    tokio::spawn(common::socsk5_server::spawn_socks_server(args.socks5_server));//socks5服务
    let bind_addr = format!("0.0.0.0:{}",&args.quic_port);
    log::info!("\nbind:\t{}\nserver:\t{}\nlocal:\t{}",bind_addr,args.server,common::msg::id_str(args.id.as_bytes()));
    let socket = UdpSocket::bind(&bind_addr).expect("couldn't bind to address");
    drop(bind_addr);
    //let transport_config_server = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    //let incoming = common::make_server_udp_endpoint(socket.try_clone().unwrap(),HOST_NAME,CER.to_vec(),KEY.to_vec(),Some(transport_config_server)).unwrap();
    let s = socket.try_clone().unwrap();
    tokio::spawn(async move{server(args.socks5_server, s).await;});//p2p as server
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



async fn handle_peer_client_connect(socks5_client_port:u16,self_id:&str,another_id:&str,socket:UdpSocket,(mut send_stream, mut recv_stream): (quinn::SendStream, quinn::RecvStream)){
    send_stream.write_all(&(common::msg::Msg::Reg(self_id.as_bytes().to_vec()).body())).await.unwrap();
    if another_id ==""{//不连接其他节点
        let self_id = self_id.to_string();
        tokio::spawn(async move{handle_peer_client_connect_send_keepalive(&self_id, send_stream).await;});
    }else{
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        send_stream.write_all(&(common::msg::Msg::Req(another_id.as_bytes().to_vec()).body())).await.unwrap();
        log::info!("{:?}request:{}",common::msg::Msg::Req(another_id.as_bytes().to_vec()).body(),another_id);
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
                        log::info!("recv:len:{},type:{},body:{:?}",buffer[0],buffer[1],&buffer[2..len]);
                        //log::info!("recv:len:{},type:{},body:{}",buffer[0],buffer[1],std::str::from_utf8(&buffer[2..len]).unwrap_or("not utf8"));
                        match buffer[1]{
                            common::msg::MSG_HAN_S=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                log::info!("receive a request,another addr:{}",addr.to_string());
                                handshake_as_s(socket.try_clone().unwrap(),SocketAddr::V4(addr)).await;
                            }
                            common::msg::MSG_HAN_C=>{
                                let addr = common::addr::bytes_to_socketaddrv4(&buffer[2..len]);
                                let socket_c = socket.try_clone().unwrap();
                                log::info!("recv response:[{}]{}",another_id,addr);
                                tokio::spawn(async move{
                                    loop{
                                        if let Err(e) = handshake_as_c(socks5_client_port,socket_c.try_clone().unwrap(), SocketAddr::V4(addr)).await{
                                            log::info!("{}",e);
                                        }
                                    }
                                });
                                continue
                            },
                            common::msg::MSG_ERR=>{
                                log::info!("server_recv_stream read err:{}",common::msg::id_str(&buffer[2..len]));
                                break;
                            }
                            _ =>{
                                log::info!("transfer?");
                            }
                        }
                    },
                    None=> {
                        log::debug!("server close");
                        break
                    }
                }
            },
            Err(e) =>{
                log::info!("handle_peer_client_connect err:{}",e);
                break
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

//向c端发送一次性请求
async fn handshake_as_s(socket:UdpSocket,addr:SocketAddr){
    tokio::spawn(async move {
        let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
        let client = common::make_client_udp_endpoint(socket, &[CER],Some(transport_config_client)).unwrap();
        let conn = client.connect(addr,HOST_NAME).unwrap().await;
        match conn{
            Ok(quinn::NewConnection { connection: _,.. })=>{
                log::info!("handshake as s to {} success",addr.to_string());
            },
            Err(e)=>{log::info!("handshake as s to{} err:{}",addr.to_string(),e)}
        }
    });
}

async fn handshake_as_c(socks5_client_port:u16,socket:UdpSocket,another_addr:SocketAddr)->Result<(),Box<dyn Error>>{
    log::info!("handshak as c to {}",another_addr.to_string());
    let transport_config_client = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let client = common::make_client_udp_endpoint(socket, &[CER],Some(transport_config_client))?;
    let peer_client_connect = client.connect(another_addr,HOST_NAME)?;    
    let quinn::NewConnection { connection,.. } = peer_client_connect.await?;
    let (mut _send_stream,_recv_stream) = connection.open_bi().await?;
    /*tokio::spawn(async move{
        loop{
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            if let Err(e)=common::socsk5_server::client(format!("127.0.0.1:{}",socks5_client_port), common::socsk5_server::SOCKS_USER,  common::socsk5_server::SOCKS_PASS, "py.recgg.cn", 443).await{
                log::info!("keepalive err:{}",e);
                break;
            }else{
                log::info!("socks keepalive success");
            };
        }
    });*/
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}",socks5_client_port)).await?;
    log::info!("transfer start,socket client:{}",socks5_client_port);
    while let Ok((tcpstream,socket_addr))=listener.accept().await{
        log::info!("sockes5 client accept {}",socket_addr);
        let connection_cell = connection.clone();
        tcp_quic(tcpstream, connection_cell.open_bi().await.unwrap()).await;
        /*
        tokio::spawn(async move{
            let (mut read_tcp,mut write_tcp) = tcpstream.into_split();
            let (mut send_quic,mut recv_quic)= connection_cell.open_bi().await.unwrap();
            tokio::join!(
                async move{tokio::io::copy(&mut read_tcp, &mut send_quic).await.unwrap()},
                async move{tokio::io::copy(&mut recv_quic, &mut write_tcp).await.unwrap()}
            );
        });*/
    };
    Ok(())
}

async fn tcp_quic(tcpstream:TcpStream,connection_cell:(quinn::SendStream, quinn::RecvStream)){
    let (mut read_tcp,mut write_tcp) = tcpstream.into_split();
    let (mut send_quic,mut recv_quic)= connection_cell;    
    tokio::join!(
        async move{tokio::io::copy(&mut read_tcp, &mut send_quic).await.unwrap()},
        async move{tokio::io::copy(&mut recv_quic, &mut write_tcp).await.unwrap()}
    );
    /*
    tokio::spawn(async move{
        let mut buf = [0;64*1024];
        read_tcp.readable().await.unwrap();
        let n = read_tcp.try_read(&mut buf).unwrap();
        log::info!("{}:{:?}",n,&buf[..n]);
        send_quic.write_all(&buf[..n]).await.unwrap();
    });
    tokio::spawn(async move{
        let mut buf = [0;64*1024];
        let n =recv_quic.read(&mut buf).await.unwrap().unwrap();
        log::info!("{}:{:?}",n,&buf[..n]);
        write_tcp.writable().await.unwrap();
        write_tcp.try_write(&buf[..n]).unwrap();
    });
    */
}

async fn server(socks5_server_port:u16,udp_socket:UdpSocket){
    let transport_config_server = common::transport_config(KEEPALIVE_INTERVAL_MILLIS, IDLE_TIMEOUT_MILLIS);
    let mut incoming = common::make_server_udp_endpoint(udp_socket,HOST_NAME,CER.to_vec(),KEY.to_vec(),Some(transport_config_server)).unwrap();
    //tokio::spawn(async move{server(args.socks5_server,incoming).await;});//p2p as server
    loop{
        match incoming.next().await {
        Some(conn)=>{
            //log::info!("{:?} incoming",conn.remote_address());
            tokio::spawn(async move{handle_connection(socks5_server_port,conn).await});
        },
        None=>{continue;}
        };
    }
}


async fn handle_connection(socks5_server_port:u16,conn: quinn::Connecting){
    log::info!("peer connection,socks5:{}",socks5_server_port);
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
            log::info!("handle_connection err,mabe handshake from peer as server:{}",e);
        }
    }
}

async fn handle_request(socks5_server_port:u16,stream: (quinn::SendStream, quinn::RecvStream)){
    log::info!("handle_request {}",socks5_server_port);
    let conn = TcpStream::connect(format!("127.0.0.1:{}",socks5_server_port)).await.unwrap();
    tcp_quic(conn, stream).await;
    /*
    let (mut read_tcp, mut write_tcp) = conn.into_split();
    let (mut send_quic,mut recv_quic): (quinn::SendStream, quinn::RecvStream) = stream;
    tokio::spawn(async move{
        tokio::io::copy(&mut read_tcp,&mut send_quic).await.unwrap_or(0);
    });
    tokio::spawn(async move{
        tokio::io::copy(&mut recv_quic,&mut write_tcp).await.unwrap_or(0);
    });
    */
}
