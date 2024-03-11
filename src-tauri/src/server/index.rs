//! 服务器

use crate::database::interface::{Treat, TreatBody};
use crate::database::Database;
use crate::error::Error;
use crate::logger::server::ServerLogger;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use handlers::utils::Utils;
use log::info;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 存储服务器数据库名称
const SERVER_DB_NAME: &str = "server";

/// 存储服务器名称
const SERVER_NAME: &str = "servers";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub(crate) id: String,
    pub(crate) ip: String,
    pub(crate) port: u32,
    pub(crate) account: String,
    pub(crate) pwd: String,
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) create_time: Option<String>,
    pub(crate) update_time: Option<String>,
}

impl TreatBody for Server {}

impl Treat<HttpResponse> for Server {
    type B = Server;

    /// 获取 服务器列表
    fn get_list(_: &Self::B) -> Result<HttpResponse, String> {
        Database::get_list::<Server>(SERVER_DB_NAME, SERVER_NAME)
    }

    /// 存储 服务器列表
    fn insert(server: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&server, false);
        if let Some(res) = res {
            return Ok(res);
        }

        let mut response = Self::get_list(&server)?;
        info!("insert server response: {:#?}", response);
        if response.code != 200 {
            response.error = String::from("保存服务器失败");
            return Ok(response);
        }

        let mut server_clone = server.clone();
        if server.id.is_empty() {
            server_clone.id = Uuid::new_v4().to_string()
        }

        if server.port == 0 {
            server_clone.port = 22;
        }

        server_clone.create_time = Some(Utils::get_date(None));
        info!("insert server params: {:#?}", server_clone);

        let mut data: Vec<Server> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            data.push(server_clone)
        } else {
            // 查找IP是不是存在
            let serve = data.iter().find(|s| &s.ip == &server.ip);
            // 找到 IP
            if serve.is_some() {
                return Ok(get_error_response("服务器IP已存在"));
            }

            data.push(server_clone)
        }

        Database::update::<Server>(SERVER_DB_NAME, SERVER_NAME, data, "保存服务器失败")
    }

    /// 更新服务器
    fn update(server: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&server, true);
        if let Some(res) = res {
            return Ok(res);
        }

        let mut response = Self::get_list(&server)?;
        info!("update server response: {:#?}", response);
        if response.code != 200 {
            response.error = String::from("更新服务器失败");
            return Ok(response);
        }

        let mut server_clone = server.clone();
        server_clone.update_time = Some(Utils::get_date(None));
        info!("update server params: {:#?}", server_clone);

        let data: Vec<Server> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("更新服务器失败, 该服务器不存在"));
        }

        let serve = data.iter().find(|s| &s.id == &server_clone.id);
        if serve.is_none() {
            return Ok(get_error_response("更新服务器失败, 该服务器不存在"));
        }

        // 判断 IP 是否存在
        let serve = data.iter().find(|s| &s.ip == &server_clone.ip && &s.id != &server_clone.id);
        if serve.is_some() {
            return Ok(get_error_response("更新服务器失败, 该服务器IP已存在"));
        }

        let servers: Vec<Server> = data.iter().map(|s| if &s.id == &server.id { server_clone.clone() } else { s.clone() }).collect();

        Database::update::<Server>(SERVER_DB_NAME, SERVER_NAME, servers, "更新服务器失败")
    }

    /// 删除服务器
    fn delete(server: &Self::B) -> Result<HttpResponse, String> {
        if server.id.is_empty() {
            return Ok(get_error_response("删除服务器失败, `id` 不能为空"));
        }

        let mut response = Self::get_list(&server)?;
        if response.code != 200 {
            response.error = String::from("删除服务器失败");
            return Ok(response);
        }

        let id = &server.id;
        info!("delete server id: {}", &id);
        let data: Vec<Server> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("删除服务器失败, 该服务器不存在"));
        }

        let serve = data.iter().find(|s| s.id.as_str() == id.as_str());
        if serve.is_none() {
            return Ok(get_error_response("删除服务器失败, 该服务器不存在"));
        }

        let servers: Vec<Server> = data.iter().filter_map(|s| if s.id.as_str() == id.as_str() { None } else { Some(s.clone()) }).collect();

        // 删除流水线
        Pipeline::clear(&id)?;

        // 删除流水线日志
        ServerLogger::delete_log_dir(&id);

        // 更新服务器列表
        Database::update::<Server>(SERVER_DB_NAME, SERVER_NAME, servers, "删除服务器失败")
    }

    /// 根据 ID 查找数据
    fn get_by_id(server: &Self::B) -> Result<HttpResponse, String> {
        if server.id.is_empty() {
            return Ok(get_error_response("根据 ID 查找服务器失败, `id` 不能为空"));
        }

        let mut response = Self::get_list(&server)?;
        if response.code != 200 {
            response.error = String::from("根据 ID 查找服务器失败");
            return Ok(response);
        }

        let id = &server.id;
        info!("get server by id: {}", &id);
        let data: Vec<Server> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("根据 ID 查找服务器失败, 服务器不存在"));
        }

        let serve = data.iter().find(|s| &s.id == &server.id);

        if let Some(data) = serve {
            let data = serde_json::to_value(data).map_err(|err| Error::Error(err.to_string()).to_string())?;
            return Ok(get_success_response(Some(data)));
        }

        return Ok(get_error_response("根据 ID 查找服务器失败, 该服务器不存在"));
    }
}

impl Server {
    /// 数据检查
    fn validate(server: &Server, is_update: bool) -> Option<HttpResponse> {
        if is_update {
            if server.id.is_empty() || server.ip.is_empty() {
                return Some(get_error_response("更新服务器失败, `id` 或 `ip` 不能为空"));
            }
        } else {
            if server.ip.is_empty() {
                return Some(get_error_response("更新服务器失败, `ip` 不能为空"));
            }
        }

        if server.account.is_empty() {
            return Some(get_error_response("更新服务器失败, `账号` 不能为空"));
        }

        if server.pwd.is_empty() {
            return Some(get_error_response("更新服务器失败, `密码` 不能为空"));
        }

        return None;
    }
}
