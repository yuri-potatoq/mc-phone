use std::sync::Arc;

use sqlx::SqlitePool;

use crate::error::{CrateResult, Error};

pub(crate) struct UserManager {
    pool: Arc<SqlitePool>,
}


impl UserManager {
    pub(crate) fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            pool
        }
    }
    
    pub(crate) async fn new_user(&self, nick: String, password: String) -> CrateResult<()> {
        sqlx::query("INSERT INTO rcon_users(game_nick, password) VALUES ($1, $2);")
            .bind(nick)
            .bind(password)
            .execute(Arc::as_ref(&self.pool))
            .await
            .map_err(Error::cant_create_user)?;
        
        Ok(())
    }
    
    pub(crate) async fn add_user_permissions(&self, nick: String, permissions: Vec<String>) -> CrateResult<()> {
        for p in permissions {
            sqlx::query("
                INSERT INTO users_permissions(user_id, command) VALUES(
                  (SELECT id FROM rcon_users WHERE game_nick = $1),
                  $2
                );
                ")
                .bind(nick.clone())
                .bind(p.clone())
                .execute(Arc::as_ref(&self.pool))
                    .await
                    .map_err(Error::cant_create_user)?;
        }            
        
        Ok(())
    }
    
    pub(crate) async fn create_super_user(&self, root_password: String) -> CrateResult<()> {
        let tx = self.pool.begin().await.unwrap();
        
        let result = sqlx::query("            
            INSERT INTO rcon_users(game_nick, password) VALUES(
                'admin', $1
            ) ON CONFLICT DO UPDATE SET password = $1 WHERE game_nick = 'admin';
        
            INSERT OR IGNORE INTO users_permissions(user_id, command) VALUES(
                (SELECT id FROM rcon_users WHERE game_nick = 'admin'),
                'admin'
            );
            ")
            .bind(root_password.clone())
            .execute(Arc::as_ref(&self.pool))
            .await;
                
        match result {
            Ok(_) => {
                tx.commit().await.unwrap();
                Ok(())
            },
            Err(err) => {
                tx.rollback().await.unwrap();
                Err(Error::cant_create_user(err))
            }
        }
    }
    
    pub(crate) async fn has_permissions(&self, nick: String, permission: String) -> CrateResult<()> {
        let result = sqlx::query("
            SELECT * FROM users_permissions
            WHERE
                user_id = (SELECT id FROM rcon_users WHERE game_nick = $1)
                AND users_permissions.command = $2
            ")
            .bind(&nick)
            .bind(&permission)
            .fetch_optional(Arc::as_ref(&self.pool))
                .await
                .map_err(Error::cant_create_user)?;           
        
        if let Some(_) = result {
            Ok(())
        }else {
            Err(Error::DontHavePermission { raw_err: nick })
        }
    }
}


//TODO: use #[sqlx::test] https://docs.rs/sqlx/latest/sqlx/attr.test.html#supported-databases
#[cfg(test)]
mod user_manager_test {
    use sqlx::{pool::PoolOptions, Sqlite};

    use super::*;
    use std::sync::OnceLock;
    static POOL: OnceLock<SqlitePool> = OnceLock::new();
    
    async fn migrate() {        
        let pool = match POOL.get() {
            Some(p) => p,
            None => {
                println!("creating new pool");
                let pool = SqlitePool::connect("sqlite::memory:")
                    .await
                    .expect("should create new pool");
                
                if let Err(_) = POOL.set(pool) {
                    return;
                }else {
                    POOL.get().unwrap()
                }
            }
        };
        
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .expect("should run the migrations");
    }
        
    #[tokio::test]
    async fn create_and_add_permissions() {
        migrate().await;
        
        let pool = Arc::new(POOL.get().expect("should get POOL").clone());
        let manager = UserManager::new(pool.clone());
        
        manager.new_user("steve".to_string(), "hasshed@password".to_string()).await.unwrap();
        manager.add_user_permissions("steve".to_string(), vec!["say".to_string()]).await.unwrap();
        
        let permissions: (String,) = sqlx::query_as("SELECT command FROM users_permissions")
            .bind("steve".to_string())
            .fetch_one(Arc::as_ref(&pool))
            .await
            .unwrap();
        
        assert_eq!(permissions.0, "say");
    }
}