/**!
  首页, 包括最近查看和下载的文件,
  windows: C:\Users\<你的用户名>\AppData\Roaming\Microsoft\Windows\Recent
  mac: ~/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.RecentDocuments.sfl2(macos < 12可用)
  只显示最近前30条记录
*/
use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_error_response, get_success_response_by_value, HttpResponse};
use crate::utils::Utils;
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Look {
    #[serde(rename = "fileName")]
    pub file_name: String, // 文件名
    #[serde(rename = "filePath")]
    pub file_path: String, // 文件路径
    #[serde(rename = "fileContentType")]
    pub file_content_type: String, // 内容类型
    #[serde(rename = "fileKind")]
    pub file_kind: String, // 类型描述
    #[serde(rename = "fileLastUsedDate")]
    pub file_last_used_date: String, // 最近使用时间
    #[serde(rename = "fileThumbnailPath")]
    pub file_thumbnail_path: String, // 缩略图路径
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
        let out_dir = Look::get_thumbnail_dir(size);

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

    // 查找 mac
    pub async fn find_in_mac() -> Result<HttpResponse, String> {
        // 用户目录
        let home = std::env::var("HOME").map_err(|err| Error::Error(err.to_string()).to_string())?;
        /* sfltool
        let sfl2_path = format!(
            "{}/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.RecentDocuments.sfl2",
            home
        );
         */

        let query = "kMDItemLastUsedDate != NULL && kMDItemLastUsedDate >= $time.today(-7d) && !(kMDItemContentTypeTree == 'com.apple.application-bundle' || kMDItemContentTypeTree == 'com.apple.installer-package-archive' || kMDItemContentTypeTree == 'public.executable' || kMDItemContentTypeTree == 'com.apple.disk-image')";

        let folders = vec!["Documents", "Downloads", "Desktop", "Pictures"];
        let mut cmd = Command::new("mdfind");
        cmd.arg(query);

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
        data.par_iter().for_each(|(folder, stdout)| {
            // stdout.lines() 返回的是 普通迭代器（同步的）
            // par_bridge() 是 rayon 提供的工具，它把一个普通迭代器变成并行迭代器
            stdout.lines().par_bridge().for_each(|path| {
                if counter.load(Ordering::Relaxed) >= LIMIT_SIZE {
                    return;
                }

                if fs::metadata(path).is_err() {
                    return;
                }

                let mdls = Command::new("mdls")
                    .args(["-name", "kMDItemDisplayName", "-name", "kMDItemContentType", "-name", "kMDItemKind", "-name", "kMDItemLastUsedDate", "-name", "kMDItemPath", path])
                    .output();

                if let Ok(output) = mdls {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let mut info = Look {
                        file_name: String::new(),
                        file_path: path.to_string(),
                        file_content_type: String::new(),
                        file_kind: String::new(),
                        file_last_used_date: String::new(),
                        file_thumbnail_path: String::new(),
                    };

                    for line in text.lines() {
                        if line.contains("kMDItemDisplayName") {
                            info.file_name = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemContentType") {
                            info.file_content_type = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemKind") {
                            info.file_kind = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        } else if line.contains("kMDItemLastUsedDate") {
                            info.file_last_used_date = line.split('=').nth(1).unwrap_or("").trim().to_string();
                        }
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

                    if counter.fetch_add(1, Ordering::SeqCst) < LIMIT_SIZE {
                        let mut lock = results.lock().unwrap();
                        lock.push(info);
                    }
                }
            })
        });

        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        // 按时间排序
        results.sort_by(|a, b| b.file_last_used_date.cmp(&a.file_last_used_date));
        get_success_response_by_value(results)
    }
}
