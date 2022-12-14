#[forbid(unsafe_code)]
use fast_socks5::{
    server::{Config, SimpleUserPassword, Socks5Server, Socks5Socket},
    Result
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;


const REQUEST_TIMEOUT:u64=10;
pub const SOCKS_USER:&str="alsdfjlasdfa";
pub const SOCKS_PASS:&str="asdfasdfasdf";

pub async fn spawn_socks_server(port:u16) -> Result<()> {
    let mut config = Config::default();
    config.set_request_timeout(REQUEST_TIMEOUT);
    config.set_skip_auth(false);
    config.set_authentication(SimpleUserPassword { username:SOCKS_USER.to_string(), password:SOCKS_PASS.to_string() });

    let mut listener = Socks5Server::bind(format!("0.0.0.0:{}",port)).await?;
    listener.set_config(config);

    let mut incoming = listener.incoming();

    //info!("Listen for socks connections @ {}", format!("0.0.0.0:{}",port);
    while let Some(socket_res) = incoming.next().await {
        match socket_res {
            Ok(socket) => {
                spawn_and_log_error(socket.upgrade_to_socks5());
            }
            Err(_) => {
                continue
            }
        }
    }
    Ok(())
}

fn spawn_and_log_error<F, T>(fut: F) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = Result<Socks5Socket<T>>> + Send + 'static,
    T: AsyncRead + AsyncWrite + Unpin,
{
    tokio::task::spawn(async move {
        //fut.await.unwrap();
        if let Err(e) = fut.await {
            log::info!("socks5 err:{}",e);
            //error!("{:#}", &e);
        }
    })
}

pub async fn client (socks_server:String,user:&str,pass:&str,target_addr:&str,target_port:u16)->Result<(),String>{
    let mut config = fast_socks5::client::Config::default();
    config.set_skip_auth(false);
    let socks = fast_socks5::client::Socks5Stream::connect_with_password(
        socks_server,
        target_addr.to_string(),
        target_port,
        user.to_string(),
        pass.to_string(),
        config,
    )
    .await;
    match socks{
        Ok(_)=>{Ok(())},
        Err(e)=>{Err(format!("{}",e))}
    }
}

