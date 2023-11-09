//! 文件操作

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub struct FileHandler;

impl FileHandler {
    /// 查找文件
    pub fn find(directory: Option<&Path>, filename: &Path) -> Result<PathBuf, io::Error> {
        let candidate = match directory {
            None => PathBuf::from(filename),
            Some(directory) => directory.join(filename),
        };

        match fs::metadata(&candidate) {
            Ok(metadata) => {
                if metadata.is_file() {
                    return Ok(candidate);
                }
            }
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(error);
                }
            }
        }

        if let Some(directory) = directory.clone() {
            if let Some(parent) = directory.parent() {
                Self::find(Some(parent), filename)
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "path not found"))
            }
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "path not found"))
        }
    }

    /// 打开文件
    pub fn open_file(file_path: &str) -> Result<File, io::Error> {
        return match File::open(&file_path) {
            Ok(file) => Ok(file),
            Err(error) => Err(error),
        };
    }

    /// 读取文件, 获取内容
    pub fn read_file(file_path: &str) -> Result<String, io::Error> {
        let mut file = Self::open_file(file_path)?;
        let mut contents = String::new();
        return match file.read_to_string(&mut contents) {
            Ok(_) => Ok(contents),
            Err(error) => Err(error),
        };
    }
}
