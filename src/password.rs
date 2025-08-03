use std::sync::Arc;

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use sqlx::SqlitePool;

use crate::error::{CrateResult, Error};


#[derive(Clone)]
pub(crate) struct PassHasher {
    hash_algo: Arc<Argon2<'static>>,
}

impl PassHasher {    
    
    fn new(secret_key: Arc<String>) -> Self {        
        use argon2::{Algorithm, Version, Params};
        let secret_bytes: Vec<u8> = secret_key.as_bytes().to_vec();

        let argon_setup = Argon2::new_with_secret(
            Box::leak(secret_bytes.into_boxed_slice()),
            Algorithm::Argon2id, 
            Version::default(), 
            Params::DEFAULT).unwrap();
        
        Self {
            hash_algo: Arc::new(argon_setup),
        }
    }
    
    fn hash_password(&self, password: String) -> Result<String, ()> {
        let password_hash = self.hash_algo
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();        
        
        Ok(password_hash)
    }
    
    fn verify_password<'c>(&self, password: String, hash: &'c str) -> Result<(), ()> {
        if let Ok(hashed) = PasswordHash::new(hash) {
            if let Ok(_) = self.hash_algo.verify_password(password.as_bytes(), &hashed) {
                return Ok(())
            }
        }
        
        Err(())
    }
}


#[derive(Clone)]
pub struct PasswordManager {
    pool: Arc<SqlitePool>,
    hasher: PassHasher,
}

impl PasswordManager {
    
    pub(crate) fn new(pool: Arc<SqlitePool>, secret_key: Arc<String>) -> Self {
        Self { pool, hasher: PassHasher::new(secret_key) }
    }
    
    pub(crate) fn hash_password(&self, password: String) -> Result<String, ()> {
        let password_hash = self.hasher
            .hash_password(password)
            .unwrap()
            .to_string();
        
        Ok(password_hash)
    }
    
    pub(crate) async fn verify_user_password(
        &self,
        user_nick: String, 
        password: String
    ) -> CrateResult<()> {
        let row: (String,) = sqlx::query_as("SELECT password FROM rcon_users u WHERE u.game_nick = $1")
            .bind(user_nick)
            .fetch_one(Arc::as_ref(&self.pool))
            .await
            .unwrap();
        
        match self.hasher.verify_password(password, &row.0) {
            Ok(_) => Ok(()),
            Err(_) => {
                Err(Error::PasswordDontMatch{ raw_err: "password is invalid".to_string() })
            }
        }
    }
}


#[cfg(test)]
mod hasher_test {
    use super::*;
    
    const DUMB_SECRET: &'static str = "@test-secret123";
    
    #[test]
    fn hash_and_verify_pass() {
        let hasher = PassHasher::new(Arc::new(DUMB_SECRET.to_string()));
        let pass = String::from("pass@123");
        let hash = hasher.hash_password(pass.clone()).unwrap();
        
        assert_eq!(
            Ok(()),
            hasher.verify_password(pass.clone(), &hash),
        );
    }
}

// #[cfg(test)]
// mod manager_test {
//     use super::*;
    
//     const DUMB_SECRET: &'static str = "@test-secret123";
    
//     #[tokio::test]
//     async fn pass_manager_retrive_test() {
//         let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
//         sqlx::migrate!("./migrations")
//             .run(&pool)
//             .await
//             .unwrap();
        
//         let row: (String,) = sqlx::query_as("SELECT password FROM rcon_users u WHERE u.game_nick = $1")
//             .bind("steve")
//             .fetch_one(&pool)
//             .await
//             .unwrap();
        
//         println!("{}", row.0);
        
//         let manager = PasswordManager::new(Arc::new(pool), Arc::new(DUMB_SECRET.to_string()));
        
        
//     }
// }

