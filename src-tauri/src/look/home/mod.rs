/**!
  首页, 包括最近查看｜ 图片 | 文稿 | 下载的文件,
  windows: C:\Users\<你的用户名>\AppData\Roaming\Microsoft\Windows\Recent
  mac: ~/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.RecentDocuments.sfl2(macos < 12可用)
  只显示最近前30条记录
*/
use crate::error::Error;
use crate::helper::index::Helper;
use crate::look::cache::{FileCache, FileMeta};
use crate::prepare::{get_success_response_by_value, HttpResponse};
use crate::utils::file::FileUtils;
use crate::utils::Utils;
use chrono::{DateTime, TimeZone, Utc};
use futures::future::join_all;
use handlers::file::FileHandler;
use log::{error, info};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// 缩略图目录
const THUMBNAIL_DIR: &str = "thumbnail";

// 缩略图大小, Finder 中常见缩略图大小，推荐默认值
const THUMBNAIL_SIZE: u32 = 128;

// 查询前 50 条记录
const LIMIT_SIZE: usize = 50;

pub struct Look {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FileQuery {
    #[serde(rename = "currentPage")]
    pub current_page: usize, // 页码, 从 1 开始
    #[serde(rename = "pageSize")]
    pub page_size: usize, // 每页数量
    #[serde(rename = "fileName")]
    pub file_name: String, // 名称(模糊匹配)
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>, // 根据修改时间｜创建时间正序号或倒序
    pub refresh: bool, // 是否强制刷新
}

impl Look {
    // 获取缩略图目录(thumb_64 | thumb_128 | thumb_256)
    fn get_thumbnail_dir(size: u32) -> Option<PathBuf> {
        let dir = Helper::get_project_config_dir(vec![format!("{}_{}", THUMBNAIL_DIR, size)]);
        dir.unwrap_or_else(|err| {
            error!("get thumbnail dir error: {}", err);
            None
        })
    }

    // 生成缩略图, 调用系统的 qlmanage 来生成, 有权限问题
    #[allow(dead_code)]
    fn generate_thumbnail(file_path: &str, file_name: &str, size: u32) -> Result<String, String> {
        let out_dir = Self::get_thumbnail_dir(size);

        if let Some(out_dir) = out_dir {
            // 查看目录下缩略图是否存在
            let exist_file_path = out_dir.join(format!("{}.png", file_name));
            if exist_file_path.exists() {
                info!("generate thumbnail failed, `{}` exists !", file_path);
                return Ok(String::from(file_path));
            }

            // 调用 qlmanage
            let status = Command::new("qlmanage")
                .arg("-t") // 生成缩略图
                .arg("-s")
                .arg(size.to_string()) // 指定大小（像素）
                .arg("-o")
                .arg(&out_dir) // 输出目录
                .arg(&file_path) // 要生成缩略图的文件
                .status()
                .map_err(|err| Error::Error(err.to_string()).to_string())?;

            if !status.success() {
                info!("generate thumbnail `{}` error !", file_path);
                return Ok(String::from(file_path));
            }

            // 构造输出文件名(qlmanage 会用原文件名 + .png)
            let file_name = Path::new(file_path).file_name().and_then(|s| s.to_str()).unwrap_or("");

            let path = out_dir.join(format!("{}.png", file_name));
            if !path.exists() {
                return Ok(String::new());
            }

            info!("generate thumbnail success, path: {:?} !", path);
            return Ok(path.to_string_lossy().into_owned());
        }

        Ok(String::new())
    }

    fn extract_value(line: &str) -> String {
        line.split('"').filter(|s| !s.trim().is_empty() && *s != ":" && *s != ",").nth(1).unwrap_or_default().to_string()
    }

    // 最近查看
    pub async fn get_recent_used(file_query: &FileQuery) -> Result<HttpResponse, String> {
        let mut size = file_query.page_size;
        if size == 0 {
            size = LIMIT_SIZE;
        }

        let search_name = &file_query.file_name;

        // 用户目录
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;

        let mut query = String::new();
        if !search_name.is_empty() {
            query = format!("kMDItemFSName == \"*{}*\"", search_name);
        }

        query.push_str("kMDItemLastUsedDate != NULL && kMDItemLastUsedDate >= $time.today(-15d) && !(kMDItemContentTypeTree == 'com.apple.application-bundle' || kMDItemContentTypeTree == 'com.apple.installer-package-archive' || kMDItemContentTypeTree == 'public.executable' || kMDItemContentTypeTree == 'com.apple.disk-image')");
        let folders = vec!["Documents", "Downloads", "Desktop", "Pictures"];
        let mut cmd = Command::new("mdfind");
        cmd.arg(query.clone());

        let handles: Vec<_> = folders
            .into_iter()
            .map(|folder| {
                let home = home.clone();
                let query = query.to_string();
                let folder_name = folder.to_string();

                async_std::task::spawn_blocking(move || {
                    let mut path = PathBuf::from(&home);
                    path.push(&folder_name);

                    let output = Command::new("mdfind").arg(&query).arg("-onlyin").arg(&path).output().map_err(|err| Error::Error(err.to_string()).to_string());
                    match output {
                        Ok(output) => {
                            if output.status.success() {
                                let output = String::from_utf8_lossy(&output.stdout).into_owned();
                                info!("mdfind folder name: {}, output: {}", &folder_name, &output);
                                (folder.to_string(), output)
                            } else {
                                error!("mdfind error, status: {}", output.status);
                                (folder.to_string(), String::new())
                            }
                        }
                        Err(err) => {
                            error!("mdfind output error: {:?}", err);
                            (folder.to_string(), String::new())
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        let handles = join_all(handles).await;

        let mut data: Vec<(String, String)> = Vec::new();
        for handle in handles {
            data.push(handle)
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let results = Arc::new(Mutex::new(Vec::new()));

        // 使用并行任务解析数据
        data.par_iter().for_each(|(_, stdout)| {
            // stdout.lines() 返回的是 普通迭代器（同步的）
            // par_bridge() 是 rayon 提供的工具，它把一个普通迭代器变成并行迭代器
            stdout.lines().par_bridge().for_each(|path| {
                if counter.load(Ordering::Relaxed) >= size {
                    return;
                }

                if fs::metadata(path).is_err() {
                    return;
                }

                let mdls = Command::new("mdls")
                    .args([
                        "-name",
                        "kMDItemDisplayName",
                        "-name",
                        "kMDItemContentType",
                        "-name",
                        "kMDItemKind",
                        "-name",
                        "kMDItemContentCreationDate",
                        "-name",
                        "kMDItemLastUsedDate",
                        "-name",
                        "kMDItemLogicalSize",
                        "-name",
                        "kMDItemPath",
                        path,
                    ])
                    .output();

                if let Ok(output) = mdls {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let mut info = FileMeta {
                        file_key: path.to_string(),
                        file_name: String::new(),
                        file_path: path.to_string(),
                        file_content_type: String::new(),
                        file_kind: String::new(),
                        file_thumbnail_path: String::new(),
                        file_size: None,
                        file_created: String::new(),
                        file_updated: String::new(),
                        file_type: String::new(),
                        file_hash: "".to_string(),
                    };

                    for line in text.lines() {
                        if line.contains("kMDItemDisplayName") {
                            info.file_name = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemContentType") {
                            info.file_content_type = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemKind") {
                            info.file_kind = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemLastUsedDate") {
                            let file_updated = line.split('=').nth(1).unwrap_or("").trim().to_string();
                            info.file_updated = Self::parse_date(&file_updated);
                        } else if line.contains("kMDItemContentCreationDate") {
                            let file_created = line.split('=').nth(1).unwrap_or("").trim().to_string();
                            info.file_created = Self::parse_date(&file_created);
                        } else if line.contains("kMDItemLogicalSize") {
                            let size_str = line.split('=').nth(1).unwrap_or("").trim().to_string();
                            let size: u64 = size_str.parse::<u64>().unwrap_or(0);
                            info.file_size = Some(FileUtils::convert_size(size))
                        }
                    }

                    if !info.file_created.is_empty() {
                        let timestamp = Self::get_timestamp(&info.file_created);
                        info.file_hash = FileCache::generate_file_hash(&info.file_path, timestamp.to_string().as_str());
                    }

                    // 过滤类型：排除 app/exe/dylib 安装类，保留图片、压缩包、文本等
                    let lower_kind = info.file_kind.to_lowercase();
                    if lower_kind.contains("image") || lower_kind.contains("图像") {
                        // 生成base64
                        let contents = FileHandler::read_file_buffer(path);
                        match contents {
                            Ok(contents) => {
                                info.file_thumbnail_path = Utils::generate_image(contents);
                            }
                            Err(err) => {
                                error!("read `{}` error: {:#?}", path, err);
                            }
                        }
                    }

                    if counter.fetch_add(1, Ordering::SeqCst) < size {
                        let mut lock = results.lock().unwrap();
                        lock.push(info);
                    }
                }
            })
        });

        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        // 按时间排序
        results.sort_by(|a, b| FileCache::parse_datetime(&b.file_updated).cmp(&FileCache::parse_datetime(&a.file_updated)));
        get_success_response_by_value(results)
    }

    fn parse_date(date_str: &str) -> String {
        DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z")
            .map(|dt| dt.with_timezone(&Utc))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|e| {
                error!("Failed to parse date: {}, error: {}", date_str, e);
                Utc.timestamp_opt(0, 0).unwrap().format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .to_string()
    }

    fn get_timestamp(date_str: &str) -> i64 {
        DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z").map(|dt| dt.timestamp()).unwrap_or(0)
    }

    // 读取文稿
    pub fn read_documents(file_query: &FileQuery) -> Result<HttpResponse, String> {
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut path = PathBuf::from(&home);
        path.push("Documents");
        let file_info = FileCache::query(&path, "Documents", file_query);
        get_success_response_by_value(file_info)
    }

    // 读取图片
    pub fn read_pictures(file_query: &FileQuery) -> Result<HttpResponse, String> {
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut path = PathBuf::from(&home);
        path.push("Pictures");
        let file_info = FileCache::query(&path, "Pictures", file_query);
        get_success_response_by_value(file_info)
    }

    // 读取桌面
    pub fn read_desktop(file_query: &FileQuery) -> Result<HttpResponse, String> {
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut path = PathBuf::from(&home);
        path.push("Desktop");
        let file_info = FileCache::query(&path, "Desktop", file_query);
        get_success_response_by_value(file_info)
    }

    // 读取下载
    pub fn read_download(file_query: &FileQuery) -> Result<HttpResponse, String> {
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut path = PathBuf::from(&home);
        path.push("Downloads");
        let file_info = FileCache::query(&path, "Downloads", file_query);
        get_success_response_by_value(file_info)
    }

    // 刷新
    pub async fn refresh(dir: &str) {
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string()).unwrap_or(String::new());
        let mut path = PathBuf::from(&home);
        if !path.exists() {
            error!("user home dir `{}` is not exists", home);
            return;
        }

        path.push(dir);
        if !path.exists() {
            error!("refresh dir dir `{:?}` is not exists", path);
            return;
        }

        FileCache::refresh(&path, dir);
    }
}
