//! 本地 sled 存储

use std::env;
use std::sync::{Arc, Mutex};
use dotenvy::dotenv;
use lazy_static::lazy_static;
use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use log::{error, info};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sled::{Db};
use sqlx::mysql::MySqlPoolOptions;
use crate::{DATABASE_POOLS, MAX_DATABASE_COUNT};
use crate::server::pipeline::index::PIPELINE_DB_NAME;

pub(crate) mod interface;

pub struct Database;

lazy_static! {
    static ref PIPELINE_DB: Arc<Mutex<Option<Db>>> = Arc::new(Mutex::new(None));
}

impl Database {
    /// 创建数据库
    pub(crate) fn create(name: &str) -> Result<Db, String> {
        if name.is_empty() {
            return Err(Error::convert_string("create or open database failed, `name` is empty !"));
        }
        // 获取本地临时目录
        let config_dir = Helper::get_project_config_dir(vec![String::from("db"), name.to_string()])?;
        if let Some(config_dir) = config_dir {
            let db = sled::Config::new().path(config_dir).mode(sled::Mode::HighThroughput).open().map_err(|err| Error::Error(err.to_string()).to_string())?;
            return Ok(db);
        }

        return Err(Error::convert_string("get config dir error !"));
    }

    /// 创建 流水线数据库, 需要线程共享
    pub(crate) fn create_pipeline_db(name: &str) -> Result<(), String> {
        let mut pipeline_db = PIPELINE_DB.lock().unwrap();
        if pipeline_db.is_none() {
            let db = Self::create(name)?;
            *pipeline_db = Some(db)
        }

        Ok(())
    }

    /// 更新树
    pub(crate) fn update<T>(database_name: &str, key: &str, data: Vec<T>, error: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize,
    {
        let data = serde_json::to_string(&data).map_err(|err| Error::Error(err.to_string()).to_string())?;
        info!("update data: {}", data);

        if database_name == PIPELINE_DB_NAME {
            Self::create_pipeline_db(database_name)?;
            let mut db = PIPELINE_DB.lock().unwrap();
            if let Some(db) = &mut *db {
                return match db.insert(key, data.into_bytes()) {
                    Ok(_) => Ok(get_success_response(Some(serde_json::Value::Bool(true)))),
                    Err(err) => {
                        info!("update `{}` tree failed: {:#?}", key, err);
                        Ok(get_error_response(error))
                    }
                }
            } else {
                return Ok(get_error_response(error));
            }
        } else {
            let db = Self::create(database_name)?;
            return match db.insert(key, data.into_bytes()) {
                Ok(_) => Ok(get_success_response(Some(serde_json::Value::Bool(true)))),
                Err(err) => {
                    info!("update `{}` tree failed: {:#?}", key, err);
                    Ok(get_error_response(error))
                }
            }
        }
    }


    /// 获取列表
    pub(crate) fn get_list<T>(database_name: &str, key: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize,
    {
        let mut servers: Vec<T> = Vec::new();
        if database_name == PIPELINE_DB_NAME {
            Self::create_pipeline_db(database_name)?;
            let mut db = PIPELINE_DB.lock().unwrap();
            if let Some(db) = &mut *db {
                let data = db.get(key).map_err(|err| Error::Error(err.to_string()).to_string())?;
                if let Some(data) = data {
                    servers = serde_json::from_slice(&data).map_err(|err| Error::Error(err.to_string()).to_string())?;
                }
            } else {
                info!("get `{}` list empty! ", key);
                return Ok(get_success_response(Some(Value::Array(Vec::new()))));
            }
        } else {
            let db = Database::create(database_name)?;
            let data = db.get(key).map_err(|err| Error::Error(err.to_string()).to_string())?;
            if let Some(data) = data {
                servers = serde_json::from_slice(&data).map_err(|err| Error::Error(err.to_string()).to_string())?;
            } else {
                info!("get `{}` list empty! ", key);
                return Ok(get_success_response(Some(Value::Array(Vec::new()))));
            }
        }

        if servers.is_empty() {
            info!("列表为空!");
            return Ok(get_success_response(Some(serde_json::Value::Array(Vec::new()))));
        }

        let data = serde_json::to_value(servers).map_err(|err| Error::Error(err.to_string()).to_string())?;
        return Ok(get_success_response(Some(data)));
    }

    /// 删除
    pub(crate) fn delete(database_name: &str, key: &str) -> Result<HttpResponse, String> {
        if database_name == PIPELINE_DB_NAME {
            Self::create_pipeline_db(database_name)?;
            let mut db = PIPELINE_DB.lock().unwrap();
            if let Some(db) = &mut *db {
                return match db.remove(key) {
                    Ok(_) => Ok(get_success_response(None)),
                    Err(err) => {
                        error!("delete `{}` data from db error: {:#?}", key, err);
                        Ok(get_error_response("删除数据失败"))
                    }
                }
            } else {
                let msg = format!("delete `{}` data from db failed !", key);
                error!("{}", &msg);
                return Ok(get_error_response("删除数据失败"))
            }
        } else {
            let db = Database::create(database_name)?;
            return match db.remove(key) {
                Ok(_) => Ok(get_success_response(None)),
                Err(err) => {
                    error!("delete `{}` data from db error: {:#?}", key, err);
                    Ok(get_error_response("删除数据失败"))
                }
            }
        }

    }

    /// 创建数据库
    pub(crate) async fn create_db() -> Result<(), String> {
        let mut pipeline_db = DATABASE_POOLS.lock().unwrap();
        if pipeline_db.is_none() {
            dotenv().ok();
            let url = env::var("DATABASE_URL").expect(&format!("`DATABASE_URL` not in `.env` file"));
            let database_pool = MySqlPoolOptions::new().max_connections(MAX_DATABASE_COUNT)
                .connect(&url).await.map_err(|err| Error::Error(format!("connect to {url} error: {:#?} !", err)).to_string())?;
            *pipeline_db = Some(database_pool)
        }

        Ok(())
    }

    /// 增加，修改，删除 - 需要写sql语句
    pub(crate) async fn execute<T>(sql: &str) -> Result<(), String> {
        let pool = DATABASE_POOLS.lock().unwrap();
        if let Some(pool) = &*pool {
           sqlx::query(sql).execute(pool).await.map_err(|err|Error::Error(err.to_string()).to_string())?;
        }

        Ok(())
    }

}
