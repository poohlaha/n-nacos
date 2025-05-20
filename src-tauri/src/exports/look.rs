//! Quick Look导出

use crate::look::home::{FileQuery, Look};
use crate::prepare::HttpResponse;
use crate::task::Task;

/// 获取最近使用记录
#[tauri::command]
pub async fn get_recent_used(file_query: FileQuery) -> Result<HttpResponse, String> {
    Task::task_param_future::<FileQuery, _, _>(file_query, |file_query| async move { Look::get_recent_used(&*file_query).await }).await
}

/// 获取文稿文件
#[tauri::command]
pub async fn get_document_list(file_query: FileQuery) -> Result<HttpResponse, String> {
    // Task::task(|| Look::read_documents()).await
    Task::task_param(file_query, |file_query| Look::read_documents(&*file_query)).await
}

/// 获取图片文件
#[tauri::command]
pub async fn get_pictures_list(file_query: FileQuery) -> Result<HttpResponse, String> {
    Task::task_param(file_query, |file_query| Look::read_pictures(&*file_query)).await
}

/// 获取桌面文件
#[tauri::command]
pub async fn get_desktop_list(file_query: FileQuery) -> Result<HttpResponse, String> {
    Task::task_param(file_query, |file_query| Look::read_desktop(&*file_query)).await
}

/// 获取下载文件
#[tauri::command]
pub async fn get_download_list(file_query: FileQuery) -> Result<HttpResponse, String> {
    Task::task_param(file_query, |file_query| Look::read_download(&*file_query)).await
}
