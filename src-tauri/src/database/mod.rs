//! 本地 sled 存储

use crate::error::Error;
use crate::{DATABASE_POOLS, DATABASE_URL, MAX_DATABASE_COUNT};
use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;

pub(crate) mod helper;
pub(crate) mod interface;

pub struct Database;

impl Database {
    /// 创建数据库
    pub(crate) async fn create_db() -> Result<(), String> {
        let mut pipeline_db = DATABASE_POOLS.lock().unwrap();
        if pipeline_db.is_none() {
            dotenv().ok();
            // let url = env::var("DATABASE_URL").expect(&format!("`DATABASE_URL` not in `.env` file"));
            let database_pool = MySqlPoolOptions::new()
                .max_connections(MAX_DATABASE_COUNT)
                .connect(DATABASE_URL)
                .await
                .map_err(|err| Error::Error(format!("connect to {DATABASE_URL} error: {:#?} !", err)).to_string())?;
            *pipeline_db = Some(database_pool)
        }

        Ok(())
    }
}
