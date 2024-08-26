//! 服务器

use crate::database::helper::DBHelper;
use crate::database::interface::{Treat, TreatBody};
use crate::error::Error;
use crate::logger::server::ServerLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use async_trait::async_trait;
use handlers::utils::Utils;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{FromRow, MySql};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub(crate) id: String,
    pub(crate) ip: String,
    pub(crate) port: u32,
    pub(crate) account: String,
    pub(crate) pwd: String,
    pub(crate) name: String,
    #[serde(rename = "desc")]
    pub(crate) description: String,
    pub(crate) create_time: Option<String>,
    pub(crate) update_time: Option<String>,
}

impl TreatBody for Server {}

#[async_trait]
impl Treat<HttpResponse> for Server {
    type B = Server;

    /// 存储 服务器列表
    async fn insert(server: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&server, false);
        if let Some(res) = res {
            return Ok(res);
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

        // 判断 IP 是否存在
        let query = sqlx::query("select ip from server where ip = ?").bind(&server.ip);
        let data = DBHelper::execute_rows(query).await?;
        if !data.is_empty() {
            return Ok(get_error_response("保存服务器失败, 该服务器IP已存在"));
        }

        info!("ip not exists, insert server ...");

        let query = sqlx::query::<MySql>("INSERT INTO server (id, ip, port, account, pwd, name, description, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&server_clone.id)
            .bind(&server_clone.ip)
            .bind(&server_clone.port)
            .bind(&server_clone.account)
            .bind(&server_clone.pwd)
            .bind(&server_clone.name)
            .bind(&server_clone.description)
            .bind(&server_clone.create_time)
            .bind(&server_clone.update_time);
        return DBHelper::execute_update(query).await;
    }

    /// 获取 服务器列表
    async fn get_list(_: &Self::B) -> Result<HttpResponse, String> {
        let query = sqlx::query_as::<_, Server>(
            // "SELECT id, ip, CAST(port AS UNSIGNED) AS port, account, pwd, name, description, create_time, update_time FROM server ORDER BY CASE WHEN update_time IS NULL THEN 0 ELSE 1 END DESC, update_time DESC, create_time DESC",
            "SELECT id, ip, CAST(port AS UNSIGNED) AS port, account, pwd, name, description, create_time, update_time FROM server ORDER BY create_time DESC",
        );
        return DBHelper::execute_query(query).await;
    }

    /// 更新服务器
    async fn update(server: &Self::B) -> Result<HttpResponse, String> {
        let res = Self::validate(&server, true);
        if let Some(res) = res {
            return Ok(res);
        }

        let mut response: HttpResponse = Self::get_by_id(&server).await?;
        info!("get server by id: {} response: {:#?}", server.id, response);
        if response.code != 200 {
            response.error = String::from("更新服务器失败, 该服务器不存在");
            return Ok(response);
        }

        let mut server_clone = server.clone();
        server_clone.update_time = Some(Utils::get_date(None));

        info!("update server params: {:#?}", server_clone);

        let serve: Server = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;

        // 判断 IP 是否存在
        let query = sqlx::query_as::<_, Server>("select id, ip, CAST(port AS UNSIGNED) AS port, account, pwd, name, description, create_time, update_time from server where ip = ? and id != ?")
            .bind(&server.ip)
            .bind(&serve.id);

        let mut result = DBHelper::execute_query(query).await?;
        if result.code != 200 {
            result.error = String::from("更新服务器失败");
            return Ok(result);
        }

        let data: Vec<Server> = serde_json::from_value(result.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if !data.is_empty() {
            return Ok(get_error_response("更新服务器失败, 该服务器IP已存在"));
        }

        let query = sqlx::query::<MySql>("UPDATE server set ip = ?, port = ?, account = ?, pwd = ?, name = ?, description = ?, update_time = ? where id = ?")
            .bind(&server_clone.ip)
            .bind(&server_clone.port)
            .bind(&server_clone.account)
            .bind(&server_clone.pwd)
            .bind(&server_clone.name)
            .bind(&server_clone.description)
            .bind(&server_clone.update_time)
            .bind(&serve.id);

        return DBHelper::execute_update(query).await;
    }

    /// 删除服务器
    async fn delete(server: &Self::B) -> Result<HttpResponse, String> {
        if server.id.is_empty() {
            return Ok(get_error_response("删除服务器失败, `id` 不能为空"));
        }

        let mut response: HttpResponse = Self::get_by_id(&server).await?;
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

        let serve = data.get(0).unwrap();

        let pipeline_list = Pipeline::get_pipeline_list_by_server_id(&serve.id).await?;

        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();

        // 删除 server
        let server_query = sqlx::query::<MySql>("DELETE FROM `server` WHERE id = ?").bind(&serve.id);
        query_list.push(server_query);

        // 删除 pipeline
        for pipe in pipeline_list.iter() {
            query_list.push(sqlx::query::<MySql>("DELETE FROM pipeline WHERE id = ?").bind(&pipe.id));
            Pipeline::delete_by_pipeline(&pipe.id, &mut query_list, true);
        }

        let response = DBHelper::batch_commit(query_list).await?;
        if response.code != 200 {
            return Ok(response);
        }

        // 删除流水线日志
        ServerLogger::delete_log_dir(&id);
        Ok(get_success_response(Some(Value::Bool(true))))
    }

    /// 根据 ID 查找数据
    async fn get_by_id(server: &Self::B) -> Result<HttpResponse, String> {
        if server.id.is_empty() {
            return Ok(get_error_response("根据 ID 查找服务器失败, `id` 不能为空"));
        }

        info!("get server by id: {}", server.id);
        let query = sqlx::query_as::<_, Server>("select id, ip, CAST(port AS UNSIGNED) AS port, account, pwd, `name`, description, create_time, update_time from `server` where id = ?").bind(&server.id);
        let serve = DBHelper::execute_query_one(query).await?;
        if let Some(serve) = serve {
            get_success_response_by_value(serve)
        } else {
            Ok(get_error_response(&format!("get server by id {} error !", server.id)))
        }
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
