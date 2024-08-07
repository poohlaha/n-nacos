//! 数据库接口

use crate::error::Error;
use crate::prepare::{get_error_response, get_success_response, HttpResponse, HttpResponseData};
use crate::DATABASE_POOLS;
use async_trait::async_trait;
use log::error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sqlx::mysql::{MySqlArguments, MySqlRow};
use sqlx::query::{Query, QueryAs};
use sqlx::{FromRow, MySql};

pub trait TreatBody: Serialize + DeserializeOwned + 'static {}

pub trait Treat<R>
where
    R: HttpResponseData,
{
    type B: TreatBody;

    /// 获取列表
    fn get_list(body: &Self::B) -> Result<R, String>;

    /// 插入
    fn insert(body: &Self::B) -> Result<R, String>;

    /// 修改
    fn update(body: &Self::B) -> Result<R, String>;

    /// 删除
    fn delete(body: &Self::B) -> Result<R, String>;

    /// 根本 ID 查找数据
    fn get_by_id(body: &Self::B) -> Result<R, String>;
}

#[async_trait]
pub trait Treat2<R>
where
    R: HttpResponseData,
{
    type B: TreatBody;

    /// 获取列表
    async fn get_list(body: &Self::B) -> Result<R, String>;

    /// 插入
    async fn insert(body: &Self::B) -> Result<R, String>;

    /// 修改
    async fn update(body: &Self::B) -> Result<R, String>;

    /// 删除
    async fn delete(body: &Self::B) -> Result<R, String>;

    /// 根本 ID 查找数据
    async fn get_by_id(body: &Self::B) -> Result<R, String>;

    /// 执行 update, insert, delete
    async fn execute_update<'a>(query: Query<'a, MySql, MySqlArguments>) -> Result<HttpResponse, String> {
        let pool = {
            let pools = DATABASE_POOLS.lock().unwrap();
            pools.clone().unwrap()
        };

        let result = query.execute(&pool).await.map_err(|err| {
            let msg = format!("execute update error: {:#?}", err);
            error!("{}", msg);
            Error::Error(msg).to_string()
        });

        return match result {
            Ok(_) => Ok(get_success_response(Some(Value::Bool(true)))),
            Err(err) => Ok(get_error_response(&err)),
        };
    }

    fn get_pools() -> sqlx::Pool<MySql> {
        return {
            let pools = DATABASE_POOLS.lock().unwrap();
            pools.clone().unwrap()
        };
    }

    /// 执行 query
    async fn execute_query<'a, O>(query: QueryAs<'a, MySql, O, MySqlArguments>) -> Result<HttpResponse, String>
    where
        O: Send + Unpin + for<'r> FromRow<'r, MySqlRow> + Serialize + 'static,
    {
        let pool = {
            let pools = DATABASE_POOLS.lock().unwrap();
            pools.clone().unwrap()
        };

        let results: Result<Vec<O>, String> = query.fetch_all(&pool).await.map_err(|err| {
            let msg = format!("query server list error: {:#?}", err);
            error!("{}", msg);
            Error::Error(msg).to_string()
        });

        return match results {
            Ok(servers) => {
                let data = serde_json::to_value(servers).map_err(|err| Error::Error(err.to_string()).to_string())?;
                Ok(get_success_response(Some(data)))
            }
            Err(err) => Ok(get_error_response(&err)),
        };
    }
}
