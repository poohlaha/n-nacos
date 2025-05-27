/**!
  缓存到磁盘
*/
use crate::error::Error;
use crate::helper::index::Helper;
use crate::look::home::FileQuery;
use crate::utils::file::FileUtils;
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone};
use handlers::file::FileHandler;
use log::{error, info};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

// 缓存目录
const CACHE_FILE_DIR: &str = "cache-file";

// 缓存时间
pub const CACHE_TTL_SECONDS: i64 = 300; // 5 分钟

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    created: String,      // 缓存创建时间
    files: Vec<FileMeta>, // 缓存数据
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    #[serde(rename = "key")]
    pub file_key: String, // key
    #[serde(rename = "fileName")]
    pub file_name: String, // 文件名
    #[serde(rename = "filePath")]
    pub file_path: String, // 文件路径
    #[serde(rename = "fileContentType")]
    pub file_content_type: String, // 内容类型
    #[serde(rename = "fileKind")]
    pub file_kind: String, // 类型描述
    #[serde(rename = "fileThumbnailPath")]
    pub file_thumbnail_path: String, // 缩略图路径
    #[serde(rename = "fileSize")]
    pub file_size: Option<String>, // 文件大小
    #[serde(rename = "fileType")]
    pub file_type: String, // 文件类型, 目录 | 文件
    #[serde(rename = "fileCreated")]
    pub file_created: String, // 创建时间
    #[serde(rename = "fileUpdated")]
    pub file_updated: String, // 修改时间
    #[serde(rename = "fileHash")]
    pub file_hash: String, // 文件 Hash 值
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub total: usize, // 总数
    pub files: Vec<FileMeta>,
}

impl FileCache {
    // 获取缓存目录
    fn get_cache_dir(dir: &str) -> Option<PathBuf> {
        let dir = Helper::get_project_config_dir(vec![CACHE_FILE_DIR.to_string(), dir.to_string()]);
        dir.unwrap_or_else(|err| {
            error!("get look cache dir error: {}", err);
            None
        })
    }

    // 判断缓存是不是过期
    fn is_cache_expired(cache: &FileCache) -> bool {
        match NaiveDateTime::parse_from_str(&cache.created, "%Y-%m-%d %H:%M:%S") {
            Ok(naive_time) => {
                // 更安全地转换为本地时间
                if let Some(cached_time) = Local.from_local_datetime(&naive_time).single() {
                    let now = Local::now();
                    let elapsed = now.signed_duration_since(cached_time);
                    elapsed > Duration::seconds(CACHE_TTL_SECONDS)
                } else {
                    true // 无法唯一匹配时区时间，默认视为过期
                }
            }
            Err(_) => true, // 如果解析失败，默认视为过期
        }
    }

    // 获取缓存文件路径
    fn get_cache_file_path(dir: &str) -> Option<String> {
        let cache_dir = Self::get_cache_dir(dir);
        if cache_dir.is_none() {
            error!("read cache dir failed !");
            return None;
        }

        let file_name = format!("{}.json", dir);
        let mut cache_dir = cache_dir.unwrap();
        cache_dir = cache_dir.join(file_name);
        Some(cache_dir.to_string_lossy().to_string())
    }

    // 读取缓存
    fn read_cache(dir: &str) -> Option<(String, FileCache)> {
        let cache_file_path = Self::get_cache_file_path(dir);
        if cache_file_path.is_none() {
            error!("read cache file name failed !");
            return None;
        }

        let cache_file_path = cache_file_path.unwrap();
        let path = PathBuf::from(&cache_file_path);
        if !path.exists() {
            info!("no cache file `{}` found !", cache_file_path);
            return None;
        }

        let content = FileHandler::read_file_string(&cache_file_path);
        let content = match content {
            Ok(content) => Some(content),
            Err(err) => {
                error!("read `{}` error: {:?}", cache_file_path, err);
                None
            }
        };

        if let Some(content) = content {
            let cache: Result<FileCache, String> = serde_json::from_str(&content).map_err(|err| Error::Error(err.to_string()).to_string());
            return match cache {
                Ok(file) => {
                    // 判断缓存有没有过期
                    if Self::is_cache_expired(&file) {
                        // 过期删除文件
                        match FileHandler::delete_file(&cache_file_path) {
                            Ok(_) => {}
                            Err(err) => {
                                error!("delete_file `{}` error: {}", cache_file_path, err);
                            }
                        }

                        return None;
                    }

                    return Some((cache_file_path, file));
                }
                Err(err) => {
                    error!("serde json `{}` error: {:?}", cache_file_path, err);
                    None
                }
            };
        }

        None
    }

    // 读取目录
    fn read_file_dir(path: &PathBuf) -> Result<FileCache, String> {
        let entries = fs::read_dir(path).map_err(|err| Error::Error(err.to_string()).to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

        let results: Vec<FileMeta> = entries
            .into_par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let file_path = path.to_string_lossy().to_string();
                let metadata = entry.metadata().map_err(|err| Error::Error(err.to_string()).to_string());
                let metadata = match metadata {
                    Ok(metadata) => Some(metadata),
                    Err(err) => {
                        error!("read metadata error: {:#?}", err);
                        None
                    }
                };

                if metadata.is_none() {
                    return None;
                }

                let metadata = metadata.unwrap();
                let filename = path.clone().file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();

                if filename.ends_with("Library.photoslibrary") {
                    return None;
                }

                let file_created = metadata.created().ok().and_then(|t| Self::system_time_to_string(t).ok()).unwrap_or_else(|| "".to_string());
                let file_updated = metadata.modified().ok().and_then(|t| Self::system_time_to_string(t).ok()).unwrap_or_else(|| "".to_string());

                let mut meta = FileMeta::default();
                let timestamp = metadata.created().ok().and_then(|t| match t.duration_since(std::time::UNIX_EPOCH).map_err(|err| Error::Error(err.to_string()).to_string()) {
                    Ok(since) => Some(since.as_nanos().to_string()),
                    Err(err) => {
                        error!("get file {} hash error: {:?}", meta.file_path, err);
                        None
                    }
                });

                if let Some(timestamp) = timestamp {
                    meta.file_hash = Self::generate_file_hash(&file_path, &timestamp);
                }

                meta.file_key = path.to_string_lossy().to_string();
                meta.file_name = filename.clone();
                meta.file_path = file_path.clone();
                meta.file_size = Some(FileUtils::convert_size(metadata.len()));
                meta.file_created = file_created;
                meta.file_created = file_updated;

                if path.is_dir() {
                    meta.file_kind = "DIR".to_string();
                    meta.file_type = "DIR".to_string();
                    return Some(meta);
                }

                // 使用 infer 来识别 MIME 类型
                let kind = infer::get_from_path(&path).ok().flatten().map(|info| info.mime_type().to_string()).unwrap_or_else(|| {
                    // 退化方案：用扩展名推断
                    path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_string()
                });

                if kind.is_empty() || kind.to_lowercase() == "photoslibrary" || kind.to_lowercase() == "dmg" {
                    return None;
                }

                // 过滤可执行文件（macOS 上 `.app` 是目录，不会出现在这里；.exe/.bin 类扩展可拦）
                if kind.contains("application/octet-stream") || kind.contains("x-executable") {
                    return None;
                }

                meta.file_kind = kind.clone();
                meta.file_type = "FILE".to_string();
                Some(meta)
            })
            .collect();

        Ok(FileCache {
            created: Self::system_time_to_string(SystemTime::now()).unwrap_or(String::new()),
            files: results,
        })
    }

    // 生成缓存
    fn generate_cache_file(dir: &str, file_cache: &FileCache) {
        if dir.is_empty() || file_cache.files.is_empty() {
            return;
        }

        // 判断缓存是不是存在
        let cache = FileCache::read_cache(dir);
        if cache.is_none() {
            let cache_file_path = Self::get_cache_file_path(dir);
            if let Some(cache_file_path) = cache_file_path {
                let content = match serde_json::to_string_pretty(&file_cache) {
                    Ok(content) => Some(content),
                    Err(err) => {
                        error!("serde to json str error: {:#?}", err);
                        None
                    }
                };

                if let Some(content) = content {
                    match FileHandler::write_to_file_when_clear(&cache_file_path, &content) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("write to file `{}` error: {:#?}", cache_file_path, err);
                        }
                    }
                }
            }
        }
    }

    fn system_time_to_string(time: SystemTime) -> Result<String, std::time::SystemTimeError> {
        let datetime: DateTime<Local> = time.into();
        Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    pub fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
    }

    // 获取文件列表
    fn get_list(file: &FileCache, query: &FileQuery) -> Vec<FileMeta> {
        let files = file.files.clone();
        if files.is_empty() {
            return Vec::new();
        }

        let mut results = files
            .into_par_iter()
            .filter(|file| if query.file_name.is_empty() { true } else { file.file_name.to_lowercase().contains(&query.file_name) })
            .collect::<Vec<FileMeta>>();

        if results.is_empty() {
            return Vec::new();
        }

        match query.sort_by.as_deref() {
            Some("modified_desc") => results.sort_by(|a, b| Self::parse_datetime(&b.file_updated).cmp(&Self::parse_datetime(&a.file_updated))),
            Some("modified_asc") => results.sort_by(|a, b| Self::parse_datetime(&a.file_updated).cmp(&Self::parse_datetime(&b.file_updated))),
            Some("created_desc") => results.sort_by(|a, b| Self::parse_datetime(&b.file_created).cmp(&Self::parse_datetime(&a.file_created))),
            Some("created_asc") => results.sort_by(|a, b| Self::parse_datetime(&a.file_created).cmp(&Self::parse_datetime(&b.file_created))),
            _ => results.sort_by(|a, b| Self::parse_datetime(&b.file_updated).cmp(&Self::parse_datetime(&a.file_updated))), // 默认 modified_desc
        }

        // 分页
        let start = (query.current_page.saturating_sub(1)) * query.page_size;
        // let end = start + query.page_size;
        results.into_iter().skip(start).take(query.page_size).collect()
    }

    // 查询
    pub fn query(path: &PathBuf, dir: &str, query: &FileQuery) -> FileInfo {
        let mut file_cache = FileCache::default();

        // 判断缓存是不是存在
        if !query.refresh {
            let cache = FileCache::read_cache(&dir);
            if let Some((_, cache)) = cache {
                if !file_cache.files.is_empty() {
                    file_cache = cache;
                }
            }
        }

        if file_cache.files.is_empty() {
            // 读取目录
            return match Self::read_file_dir(path) {
                Ok(file_cache) => {
                    // 生成缓存文件
                    Self::generate_cache_file(dir, &file_cache);
                    let files = Self::get_list(&file_cache, &query);
                    FileInfo { total: file_cache.files.len(), files }
                }
                Err(_) => FileInfo::default(),
            };
        }

        // 读取缓存
        let files = Self::get_list(&file_cache, query);
        FileInfo { total: file_cache.files.len(), files }
    }

    // 强制缓存
    pub fn refresh(path: &PathBuf, dir: &str) {
        // 清除缓存
        let cache_file_path = Self::get_cache_file_path(dir);
        if let Some(cache_file_path) = cache_file_path {
            // 删除文件
            match FileHandler::delete_file(&cache_file_path) {
                Ok(_) => {}
                Err(err) => {
                    error!("delete_file `{}` error: {}", cache_file_path, err);
                }
            }
        }

        // 读取目录
        match Self::read_file_dir(path) {
            Ok(file_cache) => {
                // 生成缓存文件
                Self::generate_cache_file(dir, &file_cache);
            }
            Err(_) => {}
        };
    }

    // 根据文件路径 + 时间 生成文件 hash
    pub fn generate_file_hash(file_path: &str, timestamp: &str) -> String {
        let input = format!("{}-{}", file_path, timestamp);
        let digest = md5::compute(input);
        format!("file-{:x}", digest)
    }
}
