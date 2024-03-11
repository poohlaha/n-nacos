//! server 日志

use crate::logger::Logger;
use log::error;

pub struct ServerLogger;

impl ServerLogger {
    /// 删除 server 下的所有日志
    pub(crate) fn delete_log_dir(id: &str) -> bool {
        if id.is_empty() {
            error!("delete server log failed, `server_id` or `id` is empty !");
            return false;
        }

        return match Logger::delete_log_dir(vec![id.to_string()]) {
            Ok(_) => true,
            Err(err) => {
                error!("delete server log error: {}", err);
                false
            }
        };
    }
}
