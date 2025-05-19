//! Quick Look导出

use crate::look::home::Look;
use crate::prepare::HttpResponse;
use crate::task::Task;

/// 获取最近使用记录
#[tauri::command]
pub async fn get_recent_used() -> Result<HttpResponse, String> {
    Task::task_param_future::<Look, _, _>(Look::default(), |_| async move { Look::find_in_mac().await }).await
}
