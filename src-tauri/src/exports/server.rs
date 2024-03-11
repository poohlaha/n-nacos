//! 导出服务器方法

use crate::database::interface::Treat;
use crate::prepare::HttpResponse;
use crate::server::index::Server;
use crate::task::Task;

/// 获取服务器列表
#[tauri::command]
pub async fn get_server_list() -> Result<HttpResponse, String> {
    Task::task(|| {
        let server = Server::default();
        Server::get_list(&server)
    })
    .await
}

/// 插入服务器
#[tauri::command]
pub async fn insert_server(server: Server) -> Result<HttpResponse, String> {
    Task::task_param::<Server, _>(server, |server| Server::insert(&server)).await
}

/// 更新服务器
#[tauri::command]
pub async fn update_server(server: Server) -> Result<HttpResponse, String> {
    Task::task_param::<Server, _>(server, |server| Server::update(&server)).await
}

/// 删除服务器
#[tauri::command]
pub async fn delete_server(id: String) -> Result<HttpResponse, String> {
    let mut server = Server::default();
    server.id = id.to_string();
    Task::task_param(server, |server| Server::delete(server)).await
}

/// 获取服务器详情
#[tauri::command]
pub async fn get_server_detail(id: String) -> Result<HttpResponse, String> {
    let mut server = Server::default();
    server.id = id.to_string();
    Task::task_param(server, |server| Server::get_by_id(&server)).await
}
