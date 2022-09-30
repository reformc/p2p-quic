use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    author="reform <reformgg@gmail.com>", 
    version="0.1.0",
    about="制作quic证书",
)]
struct Args {
    /// 域名。
    #[clap(long,short,default_value = "reform")]
    host: String,
    /// 文件名。
    #[clap(long,short,default_value = "G:/")]
    path: String
}

fn main() {
    let args = Args::parse();
    let path = 
    match &args.path as &str{
        ""=>{dirs::home_dir().unwrap()},
        _=>{std::path::PathBuf::from(&args.path)}
    };
    configure_server(&args.host,path);
}

fn configure_server(host:&str,mut path:std::path::PathBuf) {
    let cert = rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    //let priv_key = rustls::PrivateKey(priv_key);
    //let cert_chain = vec![rustls::Certificate(cert_der.clone())];
    path.push("reform.cer");
    println!("{}",path.as_path().display());
    //println!("{:?}",&cert_der);
    //println!("{:?}",&priv_key);
    std::fs::write(&path, cert_der).unwrap();
    path.set_file_name("reform.key");
    std::fs::write(&path,priv_key).unwrap();
}