//! 导出应用程序方法

use crate::applications::Applications;
use crate::prepare::HttpResponse;
use crate::task::Task;

#[tauri::command]
pub async fn get_application_list() -> Result<HttpResponse, String> {
    Task::task(|| Applications::get_application_list()).await
}

#[tauri::command]
pub async fn kill_app(pids: Vec<u32>) -> Result<HttpResponse, String> {
    Task::task_param(pids, |pids| Applications::kill_app_by_process_ids(pids)).await
}

#[tauri::command]
pub async fn get_app_process_id(name: String, path: Option<String>) -> Result<HttpResponse, String> {
    Task::task_param((name, path), |(name, path)| Applications::get_app_process_id(name, path.clone())).await
}
