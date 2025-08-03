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
use crate::user::{UserManager};


pub async fn run_server<'a>(
    pool: SqlitePool,
    pass_manager: PasswordManager,
    rcon: RconConnection,
    user_manager: UserManager,
) -> io::Result<()> {
    let session_secret_key = Key::generate();    
    let expiration = Duration::from_secs(24 * 60 * 60);
        
    let rcon_app_data = web::Data::new(rcon);
    let user_manager_data = web::Data::new(user_manager);
    
    HttpServer::new(move || {
        // keep app_data here to avoid being drop outside
        let pass_manager = web::Data::new(pass_manager.clone());
        
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
            .app_data(Data::clone(&user_manager_data))
            .wrap(identity_mw)
            .wrap(session_mw)
            .service(index)
            .service(login)
            .service(logout)
            .service(rcon_command)
            .service(create_user)
            .service(add_permissions)
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
    user_manager: web::Data<UserManager>,
) -> impl Responder {
    let identity = user.expect("logged user");
    let nick = identity.id().unwrap();
    
    if let Ok(_) = user_manager.has_permissions(nick.clone(), "admin".to_string()).await {
        rcon.exec_command(format!("/{} {}", command.command, command.args[0]))
            .await
            .unwrap();
        HttpResponse::Ok()
    } else {
        HttpResponse::Unauthorized()
    }
}

#[derive(Deserialize, Serialize)]
struct CreateUserRequest {
    nick: String,
    password: String,
}

#[post("/user/new")]
async fn create_user(
    user: Option<Identity>, 
    command: web::Json<CreateUserRequest>,
    user_manager: web::Data<UserManager>,
    pass_manager: web::Data<PasswordManager>,
) -> impl Responder {
    let _ = user.expect("logged user");
    
    let user_hash = pass_manager.hash_password(command.password.clone()).unwrap();
    match user_manager.new_user(
        command.nick.clone(), 
        user_hash.clone(),
    ).await {
        Ok(_) => {
            HttpResponse::Created()
        },
        Err(err) => {
            println!("{}", err);
            HttpResponse::InternalServerError()
        }
    }
}


#[derive(Deserialize, Serialize)]
struct GrantUserPermissionsRequest {
    nick: String,
    permissions: Vec<String>,
}

#[post("/user/grant/permission")]
async fn add_permissions(
    user: Option<Identity>, 
    command: web::Json<GrantUserPermissionsRequest>,
    user_manager: web::Data<UserManager>,
) -> impl Responder {
    let identity = user.expect("logged user");
    
    let requirer_nick = identity.id().unwrap();
    if let Ok(_) = user_manager.has_permissions(requirer_nick.clone(), "admin".to_string()).await {
        user_manager
            .add_user_permissions(command.nick.clone(), command.permissions.clone())
            .await
            .expect("can't create permission for user");
        
        HttpResponse::Ok()
    } else {
        HttpResponse::Unauthorized()
    }
}
