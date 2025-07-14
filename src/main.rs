
mod error;
use error::*;

mod rcon;
use rcon::*;


fn main() -> CrateResult<()> {    
    let mut conn = RconConnection::connect("0.0.0.0:25575", "my-ugly@secret")?;
    
    conn.exec_command("/say hello".to_string())?;
    Ok(())
}
