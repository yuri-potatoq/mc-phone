
use std::net::TcpStream;
use std::io::prelude::{Read as IORead, Write as IOWrite};


pub fn read_u32(data: &[u8]) -> Option<u32> {
    if data.len() != 4 {
        return None;
    }
    let mut arr = [0; 4];
    arr.copy_from_slice(&data[..4]);
    return Some(u32::from_le_bytes(arr));
}

enum RCONPacketKind {
    Auth,
    AuthResponse, 
    ExecCommand,     
    ResponseValue,
}

impl Into<u32> for RCONPacketKind {
    fn into(self) -> u32 {
        use RCONPacketKind::*;
        match self {
            Auth => 3,
            AuthResponse => 2,
            ExecCommand => 2,
            ResponseValue => 0,
        }
    }
}


struct RCONPacket<'a> {
    size: u32,
    id: u32,
    kind: u32,
    body: &'a [u8]
}

impl<'a> RCONPacket<'a> {
    fn new(id: u32, kind: RCONPacketKind, body: &'a str) -> Self {        
        RCONPacket {
            id,
            body: body.as_bytes(),
            size: 4 + 4 + body.len() as u32 + 2,            
            kind: kind.into(),
        }
    }

    fn write_stream(&self, stream: &mut TcpStream) -> Result<(), ()> {
        let mut packet: Vec<u8> = Vec::new();        
        
        packet.extend(&self.size.to_le_bytes());        // Size (4 bytes)
        packet.extend(&self.id.to_le_bytes());          // ID (4 bytes)
        packet.extend(&self.kind.to_le_bytes()); // Type (4 bytes)
        packet.extend(self.body);                       // Body (14 bytes)
        packet.push(0);                            // Null terminator
        packet.push(0);                            // Second null byte
        
        match stream.write_all(&packet) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("{:?}", e);
                Err(())
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let pass = "my-ugly@secret";
    let packet = RCONPacket::new(1, RCONPacketKind::AuthResponse, pass);
    
    
    // RCON connection
    let mut stream = TcpStream::connect("0.0.0.0:25575")?;

    // let id: i32 = 1;
    // let packet_type: i32 = 3; // SERVERDATA_AUTH
    // let body = b"my-ugly@secret";

    // // Calculate size: ID + Type + Body + \0 + \0 = 4 + 4 + 14 + 1 + 1 = 24
    // let size: i32 = 4 + 4 + body.len() as i32 + 2;


    // stream.write_all(&auth_packet)?;
    packet.write_stream(&mut stream).unwrap();

    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer[..])?;

    println!("Readed {}", n);
    for byte in &buffer[..n] {
        print!("{:02X} ", byte);
    }
    println!();

    let response_size = read_u32(&buffer[..4]).unwrap();
    let response_id = read_u32(&buffer[4..8]).unwrap();
    let response_type = read_u32(&buffer[8..12]).unwrap();
    //let response_body = read_u32(&buffer[16..]).unwrap();


    println!("Size: {}", response_size);
    println!("Id: {}", response_id);
    println!("Type: {}", response_type);
    // println!("Body: {}", response_body);
    Ok(())
}
