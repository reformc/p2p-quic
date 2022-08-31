use std::net::{SocketAddrV4,Ipv4Addr};

#[allow(unused)]
pub fn socketaddrv4_to_bytes(addr:SocketAddrV4)->Vec<u8>{
    let mut res = addr.ip().octets().to_vec();
    let mut port_vec = addr.port().to_be_bytes().to_vec();
    res.append(&mut port_vec);
    res
}

#[allow(unused)]
pub fn bytes_to_socketaddrv4(bytes:&[u8])->SocketAddrV4{
    if bytes.len()<6{
        panic!("error bytes to socketAddrV4:{:?}",bytes);
    }
    let number = ((bytes[4] as u16) << 8) | bytes[5] as u16;
    SocketAddrV4::new(Ipv4Addr::new(bytes[0],bytes[1],bytes[2],bytes[3]),number)
}
