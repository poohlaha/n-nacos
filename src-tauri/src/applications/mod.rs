/*!
  应用程序
*/

use crate::error::Error;
use crate::helper::index::Helper;
use crate::monitor::Monitor;
use crate::prepare::{get_success_response_by_value, HttpResponse};
use crate::utils::Utils;
use handlers::file::FileHandler;
use log::{error, info};
use plist::{Dictionary, Value};
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
    #[serde(rename = "realName")]
    real_name: String, // 应用程序真正名字

    #[serde(rename = "processIds")]
    process_ids: Vec<u32>, // 应用程序进程
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
                        let (icon, app_name) = Self::get_app_icon_and_name(&path).ok()?;

                        let mut application = ApplicationApp::default();
                        application.path = path.clone();
                        application.name = name.to_string();
                        application.real_name = app_name;

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

                        // 判断是不是在进行
                        let app_path = path.to_string_lossy().to_string();
                        application.process_ids = Monitor::new().find_app_process_ids(&application.name, Some(app_path.clone()), Some(String::from("/Applications")));
                        return Some(application);
                    }
                }

                None
            })
            .collect();

        get_success_response_by_value(apps)
    }

    // 读取 Info.plist
    fn get_info_p_list_direction(app_path: &PathBuf) -> Result<Value, String> {
        let plist_path = app_path.join("Contents").join("Info.plist");
        let plist_data = fs::read(plist_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let plist: Value = plist::from_bytes(&plist_data).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(plist)
    }

    // 使用 encoding_rs 读取 UTF-16LE 或 UTF-16BE 文件
    fn read_utf16_auto(path: &PathBuf) -> Option<String> {
        let data = match fs::read(path) {
            Ok(data) => data,
            Err(err) => {
                error!("read `{:#?}` error: {:#?}", path, err);
                Vec::new()
            }
        };

        if data.is_empty() {
            return None;
        }

        // 尝试 UTF-16LE
        let (cow, _, had_errors) = encoding_rs::UTF_16LE.decode(&data);
        if !had_errors {
            return Some(cow.into_owned());
        }

        // 如果失败，尝试 UTF-16BE
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(&data);
        Some(cow.into_owned())
    }

    // 读取 InfoPlist.strings
    fn get_info_p_list_strings_values(app_path: &PathBuf, name: &str) -> String {
        let plist_path = app_path.join("Contents").join("Resources").join(name).join("InfoPlist.strings");
        if !plist_path.exists() {
            info!("`{:?} not exists", plist_path);
            return String::new();
        }

        let data = match FileHandler::read_file_string(plist_path.to_string_lossy().to_string().as_str()) {
            Ok(data) => data,
            Err(err) => {
                error!("read `{:#?}` error: {:#?}", plist_path, err);

                // 重新使用 encoding_rs 读取 UTF-16LE 或 UTF-16BE 文件
                let value = Self::read_utf16_auto(&plist_path);
                if let Some(value) = value {
                    value
                } else {
                    String::new()
                }
            }
        };

        if data.is_empty() {
            return String::new();
        }

        for line in data.lines() {
            if line.contains("CFBundleDisplayName") || line.contains("CFBundleName") {
                let parts: Vec<_> = line.split('=').collect();
                if parts.len() == 2 {
                    let val = parts[1].trim().trim_matches(';').trim_matches('"');
                    return val.to_string();
                }
            }
        }

        String::new()
    }

    // 获取 app 应用图标, 读取 `Info.plist` 中的字段 `CFBundleIconFile` 的值, 然后在 Resources 中查找
    fn get_app_icon_and_name(app_path: &PathBuf) -> Result<(String, String), String> {
        let plist: Value = Self::get_info_p_list_direction(&app_path)?;

        let dir = plist.as_dictionary();
        if let Some(dir) = dir {
            // 获取 app icon
            let mut app_icon_path = String::new();
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
                        app_icon_path = icon_path.to_string_lossy().to_string()
                    }
                }
            }

            let app_name = Self::get_app_name(app_path, dir)?;
            return Ok((app_icon_path, app_name));
        }

        Ok((String::new(), String::new()))
    }

    /*
     获取 app 名字
      pInfo.list:
      CFBundleDisplayName：用户可见的显示名称（可选）
      CFBundleName：应用的名称（如果没有 CFBundleDisplayName，可用它）
      CFBundleExecutable：可执行文件名

      但取中文名一般位于以下文件的 `CFBundleDisplayName` 和 `CFBundleName`:
      应用程序/Contents/Resources/zh_CN.lproj/InfoPlist.strings
      应用程序/Contents/Resources/zh-Hans.lproj/InfoPlist.strings
      应用程序/Contents/Resources/Base.lproj/InfoPlist.strings

    */

    // 但取中文名
    fn get_app_name(app_path: &PathBuf, dir: &Dictionary) -> Result<String, String> {
        // 1. 优先获取中文名 InfoPlist.strings
        let lproj_dirs = ["zh_CN.lproj", "zh-Hans.lproj", "zh-Hans-CN.lproj", "Base.lproj"];
        for lproj in &lproj_dirs {
            let strings_path = app_path.join(lproj);
            info!("path: {}", strings_path.to_string_lossy());
            let name = Self::get_info_p_list_strings_values(app_path, lproj);
            if !name.is_empty() {
                return Ok(name);
            }
        }

        // 2. 回退到 Info.plist
        let mut name = Self::get_value_from_bundle(dir, "CFBundleDisplayName");
        if name.is_empty() {
            name = Self::get_value_from_bundle(dir, "CFBundleName");
        }

        if name.is_empty() {
            return Ok(String::new());
        }

        Ok(name)
    }

    // 获取 `CFBundle` 开头的字段值
    fn get_value_from_bundle(dir: &Dictionary, name: &str) -> String {
        let value = dir.get(name);
        if let Some(value) = value {
            let value = value.as_string();
            if let Some(value) = value {
                return value.to_string();
            }
        }

        String::new()
    }

    // 通过进程ID 杀死 APP
    pub fn kill_app_by_process_ids(pids: &Vec<u32>) -> Result<HttpResponse, String> {
        let result = Monitor::new().kill_process_list(pids);
        get_success_response_by_value(result)
    }

    // 通过 name 和 path 获取进程列表
    pub fn get_app_process_id(name: &str, path: Option<String>) -> Result<HttpResponse, String> {
        info!("get app process id from `{}`, `{:?}`", name, path);
        let process_ids = Monitor::new().find_app_process_ids(&name, path, Some(String::from("/Applications")));
        get_success_response_by_value(process_ids)
    }
}
