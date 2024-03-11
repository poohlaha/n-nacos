//! 记录流水线日志, 日志目录 {server_id/id}

use crate::error::Error;
use crate::logger::Logger;
use handlers::file::FileHandler;
use log::{error, info};
use std::path::PathBuf;

pub struct PipelineLogger;
impl PipelineLogger {
    /// 通过 ID 删除单条流水线日志
    pub(crate) fn delete_log_by_id(server_id: &str, id: &str) -> bool {
        info!("begin to delete pipeline log dir ...");
        if server_id.is_empty() || id.is_empty() {
            error!("delete pipeline log failed, `server_id` or `id` is empty !");
            return false;
        }

        return match Logger::delete_log_dir(vec![server_id.to_string(), id.to_string()]) {
            Ok(_) => {
                info!("delete pipeline log dir success ...");
                true
            }
            Err(err) => {
                error!("delete pipeline log error: {}", err);
                info!("delete pipeline log dir failed ...");
                false
            }
        };
    }

    /// 获取流水线日志
    pub(crate) fn get_log_by_id(server_id: &str, id: &str) -> Option<PathBuf> {
        if server_id.is_empty() || id.is_empty() {
            error!("get pipeline log failed, `server_id` or `id` is empty !");
            return None;
        }

        Logger::get_log_dir(vec![server_id.to_string(), id.to_string()])
    }

    /// 保存流水线日志
    pub(crate) fn save_log(msg: &str, server_id: &str, id: &str, order: u32) -> bool {
        let dir = Self::get_log_by_id(server_id, id);
        if let Some(dir) = dir {
            if !dir.exists() {
                info!("save pipeline log failed, log dir: {:#?} is not exists !", dir);
                return false;
            }

            let log_file_name = format!("{}.log", order);
            let file_path = dir.join(&log_file_name);

            return match FileHandler::write_file_string_pre_line(file_path.as_path().to_string_lossy().to_string().as_str(), msg) {
                Ok(_) => true,
                Err(err) => {
                    info!("save pipeline log error: {}", err);
                    false
                }
            };
        } else {
            info!("save pipeline log failed, log dir is none !");
            return false;
        }
    }

    /// 读取流水线日志
    pub(crate) fn read_log(server_id: &str, id: &str, order: u32) -> Result<String, String> {
        let dir = Self::get_log_by_id(server_id, id);
        info!("pipeline log dir: {:#?}", dir);
        if let Some(dir) = dir {
            let log_file_name = format!("{}.log", order);
            let file_path = dir.join(&log_file_name);
            info!("pipeline log path: {:#?}", file_path);
            if !file_path.exists() {
                return Err(Error::convert_string(&format!("can not find pipeline log file: {:#?} !", file_path)));
            }

            return FileHandler::read_file_string(file_path.as_path().to_string_lossy().to_string().as_str());
        }

        return Err(Error::convert_string("can not find pipeline log dir !"));
    }
}
