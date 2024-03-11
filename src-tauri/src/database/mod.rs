//! 本地 sled 存储

use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use log::info;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) mod interface;

pub struct Database;

impl Database {
    /// 创建数据库
    pub(crate) fn create(name: &str) -> Result<sled::Db, String> {
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

    /// 更新树
    pub(crate) fn update<T>(database_name: &str, key: &str, data: Vec<T>, error: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize,
    {
        let db = Self::create(database_name)?;
        let data = serde_json::to_string(&data).map_err(|err| Error::Error(err.to_string()).to_string())?;
        info!("update data: {}", data);
        match db.insert(key, data.into_bytes()) {
            Ok(_) => Ok(get_success_response(Some(serde_json::Value::Bool(true)))),
            Err(err) => {
                info!("update `{}` tree failed: {:#?}", key, err);
                Ok(get_error_response(error))
            }
        }
    }

    /// 获取列表
    pub(crate) fn get_list<T>(database_name: &str, key: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize,
    {
        let db = Database::create(database_name)?;
        let servers = db.get(key);
        return match servers {
            Ok(servers) => {
                if let Some(servers) = servers {
                    let data: Vec<T> = serde_json::from_slice(&servers).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    let data = serde_json::to_value(data).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    return Ok(get_success_response(Some(data)));
                }

                info!("列表为空!");
                Ok(get_success_response(Some(serde_json::Value::Array(Vec::new()))))
            }
            Err(err) => {
                info!("get server from db failed: {:#?}", err);
                Ok(get_error_response("获取列表失败"))
            }
        };
    }

    /// 删除
    pub(crate) fn delete(database_name: &str, key: &str) -> Result<HttpResponse, String> {
        let db = Database::create(database_name)?;
        match db.remove(key) {
            Ok(_) => Ok(get_success_response(None)),
            Err(err) => {
                info!("get server from db failed: {:#?}", err);
                Ok(get_error_response("删除数据失败"))
            }
        }
    }
}
