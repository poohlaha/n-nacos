//! 导出监控方法

use crate::error::Error;
use crate::prepare::HttpResponse;
use crate::server::index::Server;
use crate::server::monitor::Monitor;
use std::sync::Arc;
use tauri::AppHandle;

/// 获取监控服务器详情
#[tauri::command]
pub async fn start_monitor(app: AppHandle, id: String) -> Result<HttpResponse, String> {
    let mut server = Server::default();
    server.id = id.to_string();

    let server_cloned = Arc::new(server.clone());
    let app_cloned = Arc::new(app.clone());
    let result = tauri::async_runtime::spawn_blocking(move || Monitor::exec(&*app_cloned, &*server_cloned)).await;

    return match result {
        Ok(res) => res,
        Err(err) => Err(Error::convert_string(&err.to_string())),
    };
}

/// 获取监控服务器详情
#[tauri::command]
pub async fn stop_monitor() -> Result<HttpResponse, String> {
    async_std::task::spawn(async move {
        log::info!("async_std task begin ...");
        Monitor::stop()
    })
    .await
}
