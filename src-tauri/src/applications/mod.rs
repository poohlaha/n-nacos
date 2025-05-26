/*!
  应用程序
*/

use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_success_response_by_value, HttpResponse};
use crate::utils::Utils;
use handlers::file::FileHandler;
use log::{error, info};
use plist::Value;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct Applications;

// 缩略图目录
const APPLICATIONS_ICON_DIR: &str = "applicationIcons";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationApp {
    icon: String,  // 应用程序图标
    path: PathBuf, // 应用程序位置
    name: String,  // 应用程序名称
}

impl Applications {
    // 获取应用程序图标目录
    fn get_application_icons_dir(app_name: &str) -> Option<PathBuf> {
        let dir = Helper::get_project_config_dir(vec![format!("{}", APPLICATIONS_ICON_DIR), app_name.to_string()]);
        dir.unwrap_or_else(|err| {
            error!("get application icons dir error: {}", err);
            None
        })
    }

    // 根据 icns 生成 png文件, 使用 macos 自带的 sips 工具
    // sips -s format png MyIcon.icns --out MyIcon.png
    // 提取其中最大分辨率的图标: `iconutil --convert iconset MyIcon.icns`, 然后在 `MyIcon.iconset/` 中挑选最大尺寸的 PNG 图像
    fn generate_png(file_path: &str, app_name: &str) -> Result<String, String> {
        let out_dir = Self::get_application_icons_dir(&app_name);
        if out_dir.is_none() {
            return Ok(String::new());
        }

        if let Some(out_dir) = out_dir {
            let path = PathBuf::from(file_path);
            let filename = path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
            if filename.is_empty() {
                error!("generate png `{}` failed, filename is empty !", file_path);
                return Ok(String::new());
            }

            let filename = filename.replace(".icns", ".png");

            // 判断临时文件是否存在
            let temp_file_path = out_dir.join(filename);
            if temp_file_path.exists() {
                return Ok(temp_file_path.to_string_lossy().to_string());
            }

            // 不存在则转换
            let temp_file = temp_file_path.to_string_lossy().to_string();
            // 调用 sips
            let status = Command::new("sips")
                .arg("-s")
                .arg("format")
                .arg("png")
                .arg(file_path)
                .arg("-o")
                .arg(&temp_file)
                .status()
                .map_err(|err| Error::Error(err.to_string()).to_string())?;

            if status.success() {
                info!("generate `{}` for png `{}` success !", file_path, temp_file);
                return Ok(temp_file);
            }
        }

        error!("generate png `{}` error !", file_path);
        Ok(String::new())
    }

    // 获取本机上的所有应用程序
    pub fn get_application_list() -> Result<HttpResponse, String> {
        let applications_dir = PathBuf::from("/Applications");
        let entries = fs::read_dir(applications_dir).map_err(|err| Error::Error(err.to_string()).to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

        let apps: Vec<ApplicationApp> = entries
            .iter()
            .filter_map(|entry| {
                let path = entry.path();
                // let file_path = path.to_string_lossy().to_string();
                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        let icon = Self::get_app_icon_path(&path).ok()?;
                        let mut application = ApplicationApp {
                            icon: String::new(),
                            path: path.clone(),
                            name: name.to_string(),
                        };

                        let icon_path = Self::generate_png(&icon, name).ok()?;
                        // 生成base64
                        let contents = FileHandler::read_file_buffer(&icon_path);
                        match contents {
                            Ok(contents) => {
                                application.icon = Utils::generate_image(contents);
                            }
                            Err(err) => {
                                error!("read `{}` error: {:#?}", icon_path, err);
                            }
                        }

                        return Some(application);
                    }
                }

                None
            })
            .collect();

        get_success_response_by_value(apps)
    }

    // 获取 app 应用图标, 读取 `Info.plist` 中的字段 `CFBundleIconFile` 的值, 然后在 Resources 中查找
    fn get_app_icon_path(app_path: &PathBuf) -> Result<String, String> {
        let plist_path = app_path.join("Contents").join("Info.plist");
        let plist_data = fs::read(plist_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let plist: Value = plist::from_bytes(&plist_data).map_err(|err| Error::Error(err.to_string()).to_string())?;

        let dir = plist.as_dictionary();
        if let Some(dir) = dir {
            let value = dir.get("CFBundleIconFile");
            if let Some(value) = value {
                let icon_file = value.as_string();
                if let Some(icon_file) = icon_file {
                    // 添加 `.icns` 后缀（Info.plist 里通常不带）
                    let mut icon_name = icon_file.to_string();
                    if !icon_name.ends_with(".icns") {
                        icon_name.push_str(".icns");
                    }

                    let icon_path = app_path.join("Contents").join("Resources").join(icon_name);
                    if icon_path.exists() {
                        return Ok(icon_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(String::new())
    }
}
