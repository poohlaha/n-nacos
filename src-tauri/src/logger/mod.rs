//! 记录日志

use crate::error::Error;
use crate::helper::index::Helper;
use handlers::file::FileHandler;
use log::{error, info};
use std::path::PathBuf;

pub(crate) mod pipeline;
pub(crate) mod server;

const LOG_NAME: &str = "logs";

pub struct Logger;

impl Logger {
    /// 获取日志目录
    pub(crate) fn get_log_dir(dirs: Vec<String>) -> Option<PathBuf> {
        let mut log_dirs = dirs.clone();
        log_dirs.insert(0, String::from(LOG_NAME));

        let dir = Helper::get_project_config_dir(log_dirs);
        return dir.unwrap_or_else(|err| {
            info!("get log dir error: {}", err);
            None
        });
    }

    /// 删除日志目录
    pub(crate) fn delete_log_dir(dirs: Vec<String>) -> Result<bool, String> {
        let mut log_dirs = dirs.clone();
        log_dirs.insert(0, String::from(LOG_NAME)); // 在最前面插入
        let path = Helper::get_project_config_dir(log_dirs)?;
        if let Some(path) = path {
            if !path.exists() {
                let msg = format!("delete log dirs failed, path: {:#?} is not exists !", path);
                error!("{}", &msg);
                return Err(Error::convert_string(&msg));
            }

            let path_str = path.to_string_lossy().to_string();
            info!("delete log dirs: {}", path_str);
            return match FileHandler::delete_dirs(vec![path_str]) {
                Ok(_) => Ok(true),
                Err(err) => {
                    let msg = format!("delete log dirs error: {}", err);
                    error!("{}", &msg);
                    Err(Error::convert_string(&msg))
                }
            };
        }

        return Err(Error::convert_string(&"delete log dirs failed, path is not exists !"));
    }
}
