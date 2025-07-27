use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::Mutex;
use std::io::prelude::{Read as IORead, Write as IOWrite};
use std::sync::Arc;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{CrateResult, Error};


static PACKET_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn read_i32(data: &[u8]) -> Option<i32> {
    if data.len() != 4 {
        return None;
    }
    let mut arr = [0; 4];
    arr.copy_from_slice(&data[..4]);
    return Some(i32::from_le_bytes(arr));
}


/// Id, Type
/// 3 	SERVERDATA_AUTH
/// 2 	SERVERDATA_AUTH_RESPONSE
/// 2 	SERVERDATA_EXECCOMMAND
/// 0 	SERVERDATA_RESPONSE_VALUE
#[derive(Debug)]
pub enum RCONPacketKind {
    Auth,
    AuthResponse, 
    ExecCommand,     
    ResponseValue,
}

impl Into<i32> for RCONPacketKind {
    fn into(self) -> i32 {
        use RCONPacketKind::*;
        match self {
            Auth => 3,
            AuthResponse => 2,
            ExecCommand => 2,
            ResponseValue => 0,
        }
    }
}

pub struct RCONPacket<'a> {
    size: i32,
    id: i32,
    //TODO: provide other way to handle equal kinds like SERVERDATA_AUTH_RESPONSE and SERVERDATA_EXECCOMMAND
    // Both have the same ID, we can differ them by tell which one is a response packet or a request packet
    kind: i32,
    body: &'a [u8]    
}

impl<'a> std::fmt::Display for RCONPacket<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "{{ Size: {}, Id: {}, Kind: {:?}, Body: {:?} }}", 
            self.size, self.id, self.kind, self.body)
    }
}

impl<'a> RCONPacket<'a> {    
    pub fn new<S: Into<&'a str>>(kind: RCONPacketKind, body: S) -> Self {        
        let s = body.into();
        RCONPacket {
            //TODO: how many messages can be generate in a running? Does it overflow the i32?
            id: PACKET_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as i32,
            body: s.as_bytes(),
            // ID: 4 bytes +  Type: 4 bytes + body.len() + \0 + \0
            size: 4 + 4 + s.len() as i32 + 2,            
            kind: kind.into(),
        }
    }
    
    pub fn auth_packet(pass: &'a str) -> Self {
        RCONPacket::new(RCONPacketKind::Auth, pass)
    }

    pub async fn send(&self, stream: Arc<Mutex<TcpStream>>) -> CrateResult<()> {
        let mut stream = stream.lock().await;
        let mut packet: Vec<u8> = Vec::new();        
        
        packet.extend(&self.size.to_le_bytes());   // Size (4 bytes)
        packet.extend(&self.id.to_le_bytes());     // ID (4 bytes)
        packet.extend(&self.kind.to_le_bytes());   // Type (4 bytes)
        packet.extend(self.body);                  // Body (14 bytes)
        packet.push(0);                            // Null terminator
        packet.push(0);                            // Second null byte
        
        for byte in &packet {
            print!("{:02X} ", byte);
        }
        println!();
        
        stream.write_all(&packet).await.map_err(Error::connection_error)?;
        Ok(())
    }
    
    pub async fn recv(&self, stream: Arc<Mutex<TcpStream>>) -> CrateResult<Self> {
        let mut stream = stream.lock().await;   
        let mut buffer = [0; 4096];
        let readed_n = stream.read(&mut buffer[..]).await.map_err(Error::connection_error)?;
        
        println!("Readed {}", readed_n);
        for byte in &buffer[..readed_n] {
            print!("{:02X} ", byte);
        }
        println!();
        
        let readed = Self {
            size: read_i32(&buffer[..4]).unwrap(),
            id: read_i32(&buffer[4..8]).unwrap(),
            kind: read_i32(&buffer[8..12]).unwrap(),
            body: &[0; 0],
        };
        
        println!("Readed {readed}");
        Ok(readed)
    }   
    
    pub async fn send_sync(&self, stream: Arc<Mutex<TcpStream>>) -> CrateResult<Self> {        
        self.send(Arc::clone(&stream)).await?;
        
        let resp = self.recv(Arc::clone(&stream)).await?;
        assert_eq!(resp.id, self.id, "received message with not equal ID: {}", resp);

        Ok(resp)
    }
}

pub struct RconConnection {
    stream: Arc<Mutex<TcpStream>>,
}

impl RconConnection {    
    /// Returns a authenticated session
    pub async fn connect<A: ToSocketAddrs>(addr: A, pass: &str) -> CrateResult<Self> {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(Error::connection_error)?;
                
        let mut conn = Self { stream: Arc::new(Mutex::new(stream)) };
        
        conn.rcon_auth(pass).await?;

        Ok(conn)
    }

    async fn rcon_auth(&self, pass: &str) -> CrateResult<()> {
        RCONPacket::auth_packet(pass)
            .send_sync(Arc::clone(&self.stream))
            .await?;
        Ok(())
    }
    
    pub async fn exec_command(&self, cmd: String) -> CrateResult<()> {
        let resp = RCONPacket::new(RCONPacketKind::ExecCommand, cmd.as_str())
            .send_sync(Arc::clone(&self.stream))
            .await?;
        println!("{resp}");
        Ok(())
    }
}