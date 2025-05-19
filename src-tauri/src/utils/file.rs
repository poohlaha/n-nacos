//! 文件助手

use crate::error::Error;
use crate::helper::index::Helper;
use crate::look::config::HISTORY_FILE;
use chrono::Duration;
use log::info;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileUtils;

impl FileUtils {
    /// 清空上一天的目录
    pub fn clear_yesterdays_dirs(file_path: &PathBuf) -> Result<(), String> {
        info!("clear yesterdays dirs ...");
        let now = chrono::Local::now();
        let yesterday = now - Duration::days(1);
        // let yesterday_start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap().timestamp();
        let yesterday_end = yesterday.date_naive().and_hms_opt(23, 59, 59).unwrap().timestamp();

        let entries = fs::read_dir(file_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let path_str = path.as_path().to_string_lossy().to_string();
                if path_str.ends_with(HISTORY_FILE) {
                    continue;
                }

                let metadata = path.metadata().map_err(|err| Error::Error(err.to_string()).to_string())?;
                let modified_time = metadata.modified().map_err(|err| Error::Error(err.to_string()).to_string())?;

                let modified_time = chrono::DateTime::<chrono::Local>::from(modified_time).timestamp();
                if modified_time <= yesterday_end {
                    if path.is_dir() {
                        fs::remove_dir_all(path).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    } else {
                        fs::remove_file(path).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 创建临时目录
    pub fn create_temp_dir(dir_name: &str, name: &str, need_remove_dir: bool) -> Result<Option<PathBuf>, String> {
        info!("create temp dir ...");
        // 获取路径(数据目录或home)
        let exec_path = Helper::get_project_config_dir(vec![dir_name.to_string()])?;
        info!("uncompress path: {:#?}", exec_path);

        if let Some(exec_path) = exec_path {
            // 清空上一天的目录
            Self::clear_yesterdays_dirs(&exec_path)?;

            let unzip_path = exec_path.join(Path::new(&name));

            if need_remove_dir {
                if unzip_path.exists() {
                    fs::remove_dir_all(unzip_path.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
                }

                fs::create_dir_all(&unzip_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
            }

            return Ok(Some(unzip_path));
        }

        Ok(None)
    }

    /// 格式化 permissions
    pub fn format_permissions(mode: u32) -> String {
        let user = Self::format_mode_part((mode >> 6) & 0o7);
        let group = Self::format_mode_part((mode >> 3) & 0o7);
        let others = Self::format_mode_part(mode & 0o7);
        format!("{}{}{}", user, group, others)
    }

    /// 格式化 mode
    pub fn format_mode_part(part: u32) -> String {
        let r = if (part & 0o4) == 0 { "-" } else { "r" };
        let w = if (part & 0o2) == 0 { "-" } else { "w" };
        let x = if (part & 0o1) == 0 { "-" } else { "x" };
        format!("{}{}{}", r, w, x)
    }

    /// 转换文件大小
    pub fn convert_size(size: u64) -> String {
        if size >= 1024 * 1024 * 1024 {
            format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if size >= 1024 * 1024 {
            format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
        } else if size >= 1024 {
            format!("{:.2} KB", size as f64 / 1024.0)
        } else {
            format!("{} bytes", size)
        }
    }

    /// 获取文件后缀
    pub fn get_file_suffix(file_name: &str) -> String {
        let names: Vec<&str> = file_name.split(".").collect();
        let mut file_suffix = String::new();
        if let Some(suffix) = names.last() {
            file_suffix = suffix.to_lowercase().to_string()
        }

        return file_suffix;
    }
}
