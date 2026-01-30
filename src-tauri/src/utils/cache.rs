/*!
  保存/修改整个文件
*/

use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use handlers::file::FileHandler;
use log::{error, info};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;

pub struct CacheHelper;

impl CacheHelper {
    // 获取缓存目录
    fn get_cache_file(file_name: &str) -> Option<String> {
        let dir = Helper::get_project_config_dir(vec![]);
        let dir = dir.unwrap_or_else(|err| {
            error!("get cache dir error: {}", err);
            None
        });

        if dir.is_none() {
            return None;
        }

        if let Some(dir) = dir {
            let file_name = format!("{}", file_name);
            let setting_file_path = dir.join(file_name);
            return Some(setting_file_path.to_string_lossy().to_string());
        }

        None
    }

    // 保存
    pub fn save<T>(param: &T, file_name: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
    {
        let file_path = Self::get_cache_file(file_name);
        if let Some(file_path) = file_path {
            let content = match serde_json::to_string_pretty(&param) {
                Ok(content) => Some(content),
                Err(err) => {
                    error!("serde to cache json str error: {:#?}", err);
                    None
                }
            };

            if let Some(content) = content {
                match FileHandler::write_to_file_when_clear(&file_path, &content) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("write to file `{}` error: {:#?}", file_path, err);
                    }
                }
            }

            return Ok(get_success_response(None));
        }

        Ok(get_error_response("Failed to write cache config, no config dir found !"))
    }

    fn get_config<T>(file_name: &str) -> Option<T>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
    {
        let file_path = Self::get_cache_file(file_name);
        if let Some(file_path) = file_path {
            let path = PathBuf::from(&file_path);
            if !path.exists() {
                info!("no cache file `{}` found !", file_path);
                return None;
            }

            let content = FileHandler::read_file_string(&file_path);
            let content = match content {
                Ok(content) => Some(content),
                Err(err) => {
                    error!("read `{}` error: {:?}", file_path, err);
                    None
                }
            };

            if let Some(content) = content {
                if content.is_empty() {
                    return None;
                }

                let result: Result<T, String> = serde_json::from_str(&content).map_err(|err| Error::Error(err.to_string()).to_string());

                return match result {
                    Ok(result) => Some(result),
                    Err(err) => {
                        let msg = format!("failed to deserialize robot config: {:#?}", err);
                        info!("{}", msg);
                        None
                    }
                };
            }
        }

        None
    }

    pub fn get<T>(file_name: &str) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
    {
        let result = Self::get_config::<T>(file_name);
        if let Some(result) = result {
            return get_success_response_by_value(Some(result));
        }

        Ok(get_success_response(None))
    }
}
