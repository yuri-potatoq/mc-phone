use std::{io::{self}, sync::{Arc}};
use clap::{Command, arg};

mod error;
mod rcon;
mod password;
mod user;

use error::*;

mod web_server;
use sqlx::SqlitePool;
use web_server::run_server;

use crate::{password::PasswordManager, rcon::RconConnection};
use crate::user::UserManager;

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
                .arg(
                    arg!(--root_password <ROOT_PASSWORD>)
                        .env("ROOT_PASSWORD")
                        .num_args(1)
                )
                .arg_required_else_help(true), 
        );
    
    match cmd.get_matches().subcommand() {
        Some(("server", sub_matches)) => {
            let secret_key = sub_matches
                .get_one::<String>("secret_key")
                .expect("can't get secret-key");
            let host = sub_matches
                .get_one::<String>("host")
                .expect("can't get host");
            let port = sub_matches
                .get_one::<String>("port")
                .expect("can't get host");
            let password = sub_matches
                .get_one::<String>("password")
                .expect("can't get password");
            let root_password = sub_matches
                .get_one::<String>("root_password")
                .expect("can't get root-password");
            
            let pool = SqlitePool::connect("sqlite://mc-phone.db?mode=rwc").await.unwrap();            
            let secret_arc = Arc::new(secret_key.clone());
            let password_manager = PasswordManager::new(Arc::new(pool.clone()), Arc::clone(&secret_arc));
            
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("should be migrate before run server");
            
            let rcon = RconConnection::
                connect(format!("{}:{}", host, port), password.as_str())
                .await.unwrap();
            
            let user_manager = UserManager::new(Arc::new(pool.clone()));
            
            let root_hash = password_manager.hash_password(root_password.clone()).expect("hash root password");            
            user_manager.create_super_user(root_hash.clone()).await.expect("create super user");
            
            run_server(
                pool.clone(),
                password_manager,
                rcon,
                user_manager,
            ).await.unwrap();
            
            Ok(())
        },
        _ => {
            println!("not implemented");
            Ok(())
        }
    }
}

