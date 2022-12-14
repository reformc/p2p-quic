//! Commonly used code in most examples.

use quinn::{ClientConfig, Endpoint, Incoming, ServerConfig, EndpointConfig};
use std::{error::Error, net::{SocketAddr, UdpSocket}, sync::Arc};
pub mod addr;
pub mod msg;
pub mod socsk5_server;

/// Constructs a QUIC endpoint configured for use a client only.
///
/// ## Args
///
/// - server_certs: list of trusted certificates.
#[allow(unused)]
pub fn make_client_endpoint(
    bind_addr: SocketAddr,
    server_certs: &[&[u8]],
) -> Result<Endpoint, Box<dyn Error>> {
    let client_cfg = configure_client(server_certs)?;
    let mut endpoint = Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(client_cfg);
    Ok(endpoint)
}

/// Constructs a QUIC endpoint configured for use a client only.
///
/// ## Args
///
/// - server_certs: list of trusted certificates.
#[allow(unused)]
pub fn make_client_udp_endpoint(
    udp_socket: UdpSocket,
    server_certs: &[&[u8]],
    transport_config:Option<Arc<quinn::TransportConfig>>
) -> Result<Endpoint, Box<dyn Error>> {
    let mut client_cfg = configure_client(server_certs)?;
    match transport_config{
        Some(config)=>{
            client_cfg.transport = config;
        },
        None=>{}
    }
    //let mut transport_config = quinn::TransportConfig::default();
    //transport_config.keep_alive_interval(Some(std::time::Duration::from_millis(3000)));
    //transport_config.max_idle_timeout(Some(quinn::IdleTimeout::from(quinn::VarInt::from_u32(10_000))));//毫秒
    //client_cfg.transport = Arc::new(transport_config);
    let mut endpoint = Endpoint::new(EndpointConfig::default(), None, udp_socket)?.0;
    endpoint.set_default_client_config(client_cfg);
    Ok(endpoint)
}

#[allow(unused)]
pub fn transport_config(keepalive_interval_millis:u64,idle_timeout_millis:u32)->Arc<quinn::TransportConfig>{
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.keep_alive_interval(Some(std::time::Duration::from_millis(keepalive_interval_millis)));
    transport_config.max_idle_timeout(Some(quinn::IdleTimeout::from(quinn::VarInt::from_u32(idle_timeout_millis))));//毫秒    
    //transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(10).try_into().unwrap()));
    Arc::new(transport_config)
}

/// Constructs a QUIC endpoint configured to listen for incoming connections on a certain address
/// and port.
///
/// ## Returns
///
/// - a stream of incoming QUIC connections
/// - server certificate serialized into DER format
#[allow(unused)]
pub fn make_server_endpoint(bind_addr: SocketAddr) -> Result<(Incoming, Vec<u8>), Box<dyn Error>> {
    let (server_config, server_cert) = configure_server()?;
    let (_endpoint, incoming) = Endpoint::server(server_config, bind_addr)?;
    Ok((incoming, server_cert))
}

/// Constructs a QUIC endpoint configured to listen for incoming connections on a certain address
/// and port.
///
/// ## Returns
///
/// - a stream of incoming QUIC connections
/// - server certificate serialized into DER format
#[allow(unused)]
pub fn make_server_udp_endpoint(
    udp_socket: UdpSocket,
    host:&str,
    cert_der:Vec<u8>,
    priv_key:Vec<u8>,
    transport_config:Option<Arc<quinn::TransportConfig>>
) -> Result<Incoming, Box<dyn Error>> {
    let (mut server_config, server_cert) = configure_host_server(cert_der,priv_key)?;    
    match transport_config{
        Some(config)=>{
            server_config.transport = config;
        },
        None=>{}
    }
    let (_endpoint, incoming) = Endpoint::new(EndpointConfig::default(),Some(server_config), udp_socket)?;
    Ok((incoming))
}

/// Builds default quinn client config and trusts given certificates.
///
/// ## Args
///
/// - server_certs: a list of trusted certificates in DER format.
fn configure_client(server_certs: &[&[u8]]) -> Result<ClientConfig, Box<dyn Error>> {
    let mut certs = rustls::RootCertStore::empty();
    for cert in server_certs {
        certs.add(&rustls::Certificate(cert.to_vec()))?;
    }

    Ok(ClientConfig::with_root_certificates(certs))
}

/// Returns default server configuration along with its certificate.
#[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_uni_streams(0_u8.into());

    Ok((server_config, cert_der))
}



/// Returns default server configuration along with its certificate.
#[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
fn configure_host_server(cert_der:Vec<u8>,priv_key:Vec<u8>) -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    //let cert = rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
    //let cert_der = cert.serialize_der().unwrap();
    //let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_uni_streams(0_u8.into());

    Ok((server_config, cert_der))
}

#[allow(unused)]
pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];


