use std::{io::{self}, sync::Arc};
use clap::{Command, arg};

mod error;
mod rcon;
mod password;

use error::*;

mod web_server;
use sqlx::SqlitePool;
use web_server::run_server;

use crate::rcon::RconConnection;



#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    
    
    let cmd = clap::Command::new("mc-phone")
        .about("cli tools to connection with RCON")
        .subcommand(
            Command::new("server")
                .about("init web server to connect with RCON")                
                .arg(
                    arg!(--host <HOST>)
                        .env("RCON_HOST")
                        .num_args(1)
                )
                .arg(
                    arg!(--port <PORT>)
                        .env("RCON_PORT")
                        .num_args(1)
                )
                .arg(
                    arg!(--password <PASSWORD>)
                        .env("RCON_PASS")
                        .num_args(1)
                )
                .arg(
                    arg!(--secret_key <SECRET_KEY>)
                        .env("SECRET_KEY")
                        .num_args(1)
                )
                .arg_required_else_help(true), 
        );
    
    match cmd.get_matches().subcommand() {
        Some(("server", sub_matches)) => {
            let secret_key = sub_matches
                .get_one::<String>("secret_key")
                .unwrap();
            let host = sub_matches
                .get_one::<String>("host")
                .unwrap();
            let port = sub_matches
                .get_one::<String>("port")
                .unwrap();
            let password = sub_matches
                .get_one::<String>("password")
                .unwrap();
            
            let pool = SqlitePool::connect("sqlite://mc-phone.db?mode=rwc").await.unwrap();
            
            let rcon = RconConnection::
                connect(format!("{}:{}", host, port), password.as_str())
                .await.unwrap();
            
            run_server(
                pool,
                Arc::new(secret_key.clone()),
                rcon
            ).await.unwrap();
            
            Ok(())
        },
        _ => {
            println!("not implemented");
            Ok(())
        }
    }
}

