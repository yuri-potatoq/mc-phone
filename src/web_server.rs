use std::{io, sync::Arc, time::Duration};

use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key, get, 
    middleware::Logger, post, 
    web::{self, Data}, 
    App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    error::{
        CrateResult, Error
    }, 
    rcon::RconConnection
};

use crate::password::{PasswordManager};


pub async fn run_server<'a>(
    pool: SqlitePool, 
    secret_key: Arc<String>,
    rcon: RconConnection
) -> io::Result<()> {
    let session_secret_key = Key::generate();    
    let expiration = Duration::from_secs(24 * 60 * 60);
        
    let rcon_app_data = web::Data::new(rcon);
    
    HttpServer::new(move || {
        // keep app_data here to avoid being drop outside
        let pass_manager = web::Data::new(
            PasswordManager::new(Arc::new(pool.clone()), secret_key.clone())
        );
        
        //TODO: make own impl of session store to save into database.
        let session_store = CookieSessionStore::default();
        
        let session_mw =
            SessionMiddleware::builder(session_store, session_secret_key.clone())
                // disable secure cookie for local testing
                .cookie_secure(false)
                // Set a ttl for the cookie if the identity should live longer than the user session
                .session_lifecycle(
                    PersistentSession::default()
                        .session_ttl(expiration.try_into().unwrap()),
                )
                .build();
        let identity_mw = IdentityMiddleware::builder()
            .visit_deadline(Some(expiration))
            .build();
        
        App::new()
            .wrap(Logger::default())
            .app_data(Data::clone(&pass_manager))
            .app_data(Data::clone(&rcon_app_data))
            .wrap(identity_mw)
            .wrap(session_mw)            
            .service(index)
            .service(login)
            .service(logout)
            .service(rcon_command)
    })
    .bind(("127.0.0.1", 6969))
    .unwrap()
    .workers(2)
    .run()
    .await
}



#[get("/")]
async fn index(user: Option<Identity>) -> impl Responder {
    if let Some(user) = user {
        format!("Welcome! {}", user.id().unwrap())
    } else {
        "Welcome Anonymous!".to_owned()
    }
}


#[derive(Deserialize, Serialize)]
struct LoginData {
    user: String,
    password: String
}

#[post("/login")]
async fn login(
    request: HttpRequest, 
    data: web::Json<LoginData>,
    pass_manager: web::Data<PasswordManager>,
) -> impl Responder {    
    let check = pass_manager.verify_user_password(
        data.user.clone(), data.password.clone())
        .await;
    match check {
        Ok(_) => {
            println!("{} logged succefuly", data.user);
        },
        Err(err) => {
            println!("{} error to authorize: {:?}", data.user, err);
            return HttpResponse::Unauthorized()
        }
    }
    
    Identity::login(&request.extensions(), data.user.clone().into()).unwrap();

    HttpResponse::Ok()    
    
}

#[post("/logout")]
async fn logout(user: Identity) -> impl Responder {
    user.logout();
    HttpResponse::NoContent()
}

#[derive(Deserialize, Serialize)]
struct RconCommandRequest {
    command: String,
    args: Vec<String>,
}

#[post("/rcon/command")]
async fn rcon_command(
    user: Option<Identity>, 
    rcon: web::Data<RconConnection>,
    command: web::Json<RconCommandRequest>,
) -> impl Responder {
    let _ = user.expect("logged user");
    
    rcon.exec_command(format!("/{} {}", command.command, command.args[0]))
        .await
        .unwrap();
    
    HttpResponse::NoContent()
}
