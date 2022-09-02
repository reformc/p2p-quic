pub const MSG_REG:u8=1;//注册
pub const MSG_REQ:u8=2;//请求连接其他peer
pub const MSG_HAN_S:u8=3;//向被请求方peer发送请求方的peer信息。
pub const MSG_HAN_C:u8=4;//向请求方peer发送被请求方的peer信息
pub const MSG_KEEPALIVE:u8=5;
pub const MSG_ERR:u8=255;

use crate::addr;

/*
pub fn msg_reg(mut id:Vec<u8>)->Vec<u8>{
    let mut bytes = vec!();
    bytes.push(id.len() as u8 + 2);
    bytes.push(MSG_REG);
    bytes.append(&mut id);
    bytes
}

pub fn msg_keepalive(mut id:Vec<u8>)->Vec<u8>{
    let mut bytes = vec!();
    bytes.push(id.len() as u8 + 2);
    bytes.push(MSG_KEEPALIVE);
    bytes.append(&mut id);
    bytes
}

pub fn msg_handshake_server(addr:std::net::SocketAddrV4)->Vec<u8>{    
    let mut bytes = vec!();
    let mut addr_bytes = addr::socketaddrv4_to_bytes(addr);
    bytes.push(addr_bytes.len() as u8 + 2);
    bytes.push(MSG_HAN_S);
    bytes.append(&mut addr_bytes);
    bytes
}

pub fn msg_handshake_client(addr:std::net::SocketAddrV4)->Vec<u8>{    
    let mut bytes = vec!();
    let mut addr_bytes = addr::socketaddrv4_to_bytes(addr);
    bytes.push(addr_bytes.len() as u8 + 2);
    bytes.push(MSG_HAN_C);
    bytes.append(&mut addr_bytes);
    bytes
}

pub fn msg_request(mut id:Vec<u8>)->Vec<u8>{    
    let mut bytes = vec!();
    bytes.push(id.len() as u8 + 2);
    bytes.push(MSG_REQ);
    bytes.append(&mut id);
    bytes
}
*/

pub enum Msg{
    HanC(std::net::SocketAddrV4),
    HanS(std::net::SocketAddrV4),
    Reg(Vec<u8>),
    Req(Vec<u8>),
    Keepalive(Vec<u8>),
    Err(Vec<u8>)
}

impl Msg{
    pub fn body(&self)->Vec<u8>{
        match self{
            Msg::HanC(addr)=>{
                Msg::msg_buff(addr::socketaddrv4_to_bytes(addr.clone()),MSG_HAN_C)
            },
            Msg::HanS(addr)=>{
                Msg::msg_buff(addr::socketaddrv4_to_bytes(addr.clone()),MSG_HAN_S)
            },
            Msg::Reg(id)=>{
                Msg::msg_buff(id.clone(), MSG_REG)
            },
            Msg::Req(id)=>{
                Msg::msg_buff(id.clone(), MSG_REQ)
            },
            Msg::Keepalive(id)=>{
                Msg::msg_buff(id.clone(), MSG_KEEPALIVE)
            },
            Msg::Err(body)=>{
                Msg::msg_buff(body.clone(),MSG_ERR)
            }
        }
    }

    fn msg_buff(mut buff:Vec<u8>,msg_type:u8)->Vec<u8>{
        let mut bytes = vec!();
        bytes.push(buff.len() as u8 + 2);
        bytes.push(msg_type);
        bytes.append(&mut buff);
        bytes
    }
}

pub fn id_str(buff:&[u8])->String{
    match std::str::from_utf8(buff){
        Ok(v)=>{v.to_string()}
        Err(_)=>{format!("{:?}",buff)}
    }
}