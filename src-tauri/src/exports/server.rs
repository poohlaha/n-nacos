//! 导出服务器方法

use crate::database::interface::Treat2;
use crate::prepare::HttpResponse;
use crate::server::index::Server;
use crate::task::Task;

/// 获取服务器列表
#[tauri::command]
pub async fn get_server_list() -> Result<HttpResponse, String> {
    Task::task_param_future::<Server, _, _>(Server::default(), |server| async move {
        Server::get_list(&server).await
    }).await
}

/// 插入服务器
#[tauri::command]
pub async fn insert_server(server: Server) -> Result<HttpResponse, String> {
    Task::task_param_future::<Server, _, _>(server, |server| async move {
        Server::insert(&*server).await
    }).await
}

/// 更新服务器
#[tauri::command]
pub async fn update_server(server: Server) -> Result<HttpResponse, String> {
    Task::task_param_future::<Server, _, _>(server, |server| async move {
        Server::update(&*server).await
    }).await
}

/// 删除服务器
#[tauri::command]
pub async fn delete_server(id: String) -> Result<HttpResponse, String> {
    let mut server = Server::default();
    server.id = id.to_string();
    Task::task_param_future::<Server, _, _>(server, |server| async move {
        Server::delete(&*server).await
    }).await
}

/// 获取服务器详情
#[tauri::command]
pub async fn get_server_detail(id: String) -> Result<HttpResponse, String> {
    let mut server = Server::default();
    server.id = id.to_string();
    Task::task_param_future::<Server, _, _>(server, |server| async move {
        Server::get_by_id(&*server).await
    }).await
}
