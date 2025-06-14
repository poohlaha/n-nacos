/*!
  系统设置
*/

use crate::error::Error;
use crate::helper::index::Helper;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use handlers::file::FileHandler;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// 缓存目录
const CACHE_FILE: &str = "settings.json";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "titleFontSize")]
    pub(crate) title_font_size: String,

    #[serde(rename = "fontSize")]
    pub(crate) font_size: String,

    #[serde(rename = "descFontSize")]
    pub(crate) desc_font_size: String,

    #[serde(rename = "fontFamily")]
    pub(crate) font_family: String,

    #[serde(rename = "autoStart")]
    pub(crate) auto_start: String,

    #[serde(rename = "theme")]
    pub(crate) theme: String,

    #[serde(rename = "closeType")]
    pub(crate) close_type: String,

    #[serde(rename = "nodeJsDir")]
    pub(crate) node_js_dir: String,
}

impl Settings {
    // 获取缓存目录
    fn get_cache_file() -> Option<String> {
        let dir = Helper::get_project_config_dir(vec![]);
        let dir = dir.unwrap_or_else(|err| {
            error!("get setting dir error: {}", err);
            None
        });

        if dir.is_none() {
            return None;
        }

        if let Some(dir) = dir {
            let file_name = format!("{}", CACHE_FILE);
            let setting_file_path = dir.join(file_name);
            return Some(setting_file_path.to_string_lossy().to_string());
        }

        None
    }

    pub fn save(settings: &Settings) -> Result<HttpResponse, String> {
        let setting_file_path = Self::get_cache_file();
        if let Some(setting_file_path) = setting_file_path {
            let content = match serde_json::to_string_pretty(&settings) {
                Ok(content) => Some(content),
                Err(err) => {
                    error!("serde to json str error: {:#?}", err);
                    None
                }
            };

            if let Some(content) = content {
                match FileHandler::write_to_file_when_clear(&setting_file_path, &content) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("write to file `{}` error: {:#?}", setting_file_path, err);
                    }
                }
            }

            return Ok(get_success_response(None));
        }

        Ok(get_error_response("Failed to write settings, no config dir found !"))
    }

    pub fn get_settings() -> Option<Settings> {
        let setting_file_path = Self::get_cache_file();
        if let Some(setting_file_path) = setting_file_path {
            let path = PathBuf::from(&setting_file_path);
            if !path.exists() {
                info!("no cache file `{}` found !", setting_file_path);
                return None;
            }

            let content = FileHandler::read_file_string(&setting_file_path);
            let content = match content {
                Ok(content) => Some(content),
                Err(err) => {
                    error!("read `{}` error: {:?}", setting_file_path, err);
                    None
                }
            };

            if let Some(content) = content {
                if content.is_empty() {
                    return None;
                }

                let settings: Result<Settings, String> = serde_json::from_str(&content).map_err(|err| Error::Error(err.to_string()).to_string());

                return match settings {
                    Ok(settings) => Some(settings),
                    Err(err) => {
                        let msg = format!("failed to deserialize settings: {:#?}", err);
                        info!("{}", msg);
                        None
                    }
                };
            }
        }

        None
    }

    pub fn get() -> Result<HttpResponse, String> {
        let settings = Self::get_settings();
        if let Some(settings) = settings {
            return get_success_response_by_value(Some(settings));
        }

        Ok(get_success_response(None))
    }
}
