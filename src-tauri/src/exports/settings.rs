//! 导出设置方法

use crate::prepare::HttpResponse;
use crate::setting::Settings;
use crate::task::Task;

/// 保存
#[tauri::command]
pub async fn save_setting(settings: Settings) -> Result<HttpResponse, String> {
    Task::task_param(settings, |settings| Settings::save(&*settings)).await
}

/// 获取
#[tauri::command]
pub async fn get_setting() -> Result<HttpResponse, String> {
    Task::task(|| Settings::get()).await
}
