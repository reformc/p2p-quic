fn main() {
    configure_server("OK","G:/test");
    println!("Hello, world!");
}

fn configure_server(host:&str,path:&str) {
    let cert = rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    //let priv_key = rustls::PrivateKey(priv_key);
    //let cert_chain = vec![rustls::Certificate(cert_der.clone())];3
    println!("{:?}\n{:?}",&cert_der,&priv_key);
    std::fs::write(&format!("{}.cer",path), cert_der).unwrap();
    std::fs::write(&format!("{}.key",path),priv_key).unwrap();
}