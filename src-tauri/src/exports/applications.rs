//! 导出应用程序方法

use crate::applications::Applications;
use crate::prepare::HttpResponse;
use crate::task::Task;

#[tauri::command]
pub async fn get_application_list() -> Result<HttpResponse, String> {
    Task::task(|| Applications::get_application_list()).await
}
