//! Quick Look导出

use crate::look::home::Look;
use crate::prepare::HttpResponse;
use crate::task::Task;

/// 获取最近使用记录
#[tauri::command]
pub async fn get_recent_used() -> Result<HttpResponse, String> {
    Task::task_param_future::<Look, _, _>(Look::default(), |_| async move { Look::get_recent_used().await }).await
}

/// 获取文稿文件
#[tauri::command]
pub async fn get_document_list() -> Result<HttpResponse, String> {
    Task::task(|| Look::read_documents()).await
}

/// 获取图片文件
#[tauri::command]
pub async fn get_pictures_list() -> Result<HttpResponse, String> {
    Task::task(|| Look::read_pictures()).await
}

/// 获取桌面文件
#[tauri::command]
pub async fn get_desktop_list() -> Result<HttpResponse, String> {
    Task::task(|| Look::read_desktop()).await
}

/// 获取下载文件
#[tauri::command]
pub async fn get_download_list() -> Result<HttpResponse, String> {
    Task::task(|| Look::read_download()).await
}
