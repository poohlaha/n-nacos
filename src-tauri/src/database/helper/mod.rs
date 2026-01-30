//! 数据库助手

use crate::error::Error;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use crate::DATABASE_POOLS;
use log::error;
use serde::Serialize;
use serde_json::Value;
use sqlx::mysql::{MySqlArguments, MySqlRow};
use sqlx::query::{Query, QueryAs};
use sqlx::{FromRow, MySql};

pub struct DBHelper;

impl DBHelper {
    /// 执行 update, insert, delete
    pub(crate) async fn execute_update<'a>(query: Query<'a, MySql, MySqlArguments>) -> Result<HttpResponse, String> {
        let pool = Self::get_pools();

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

    /// 执行 sql, 返回 mySqlRow
    pub(crate) async fn execute_rows<'a>(query: Query<'a, MySql, MySqlArguments>) -> Result<Vec<MySqlRow>, String> {
        let pool = Self::get_pools();

        return query.fetch_all(&pool).await.map_err(|err| {
            let msg = format!("execute rows error: {:#?}", err);
            error!("{}", msg);
            Error::Error(msg).to_string()
        });
    }

    pub(crate) fn get_pools() -> sqlx::Pool<MySql> {
        return {
            let pools = DATABASE_POOLS.lock().unwrap();
            pools.clone().unwrap()
        };
    }

    /// 执行 query
    pub(crate) async fn execute_query_one<'a, O>(query: QueryAs<'a, MySql, O, MySqlArguments>) -> Result<Option<O>, String>
    where
        O: Send + Unpin + for<'r> FromRow<'r, MySqlRow> + Serialize + 'static,
    {
        let pool = Self::get_pools();
        let result = query.fetch_optional(&pool).await.map_err(|err| {
            let msg = format!("query list error: {:#?}", err);
            error!("{}", msg);
            Error::Error(msg).to_string()
        })?;
        return Ok(result);
    }

    /// 执行 query
    pub(crate) async fn execute_query<'a, O>(query: QueryAs<'a, MySql, O, MySqlArguments>) -> Result<HttpResponse, String>
    where
        O: Send + Unpin + for<'r> FromRow<'r, MySqlRow> + Serialize + 'static,
    {
        let pool = Self::get_pools();

        let results: Result<Vec<O>, String> = query.fetch_all(&pool).await.map_err(|err| {
            let msg = format!("query list error: {:#?}", err);
            error!("{}", msg);
            Error::Error(msg).to_string()
        });

        return match results {
            Ok(servers) => {
                let data: Option<Value>;
                if !servers.is_empty() {
                    data = Some(serde_json::to_value(servers).map_err(|err| Error::Error(err.to_string()).to_string())?);
                } else {
                    data = Some(Value::Array(Vec::new()))
                }

                Ok(get_success_response(data))
            }
            Err(err) => Ok(get_error_response(&err)),
        };
    }

    /// 使用事务批量提交
    pub(crate) async fn batch_commit<'a>(query_list: Vec<Query<'a, MySql, MySqlArguments>>) -> Result<HttpResponse, String> {
        let pool = Self::get_pools();

        // 开始事务
        let mut tx: sqlx::Transaction<'_, MySql> = pool.begin().await.map_err(|err| {
            let msg = format!("begin transaction error: {:?}", err);
            error!("{}", &msg);
            Error::Error(msg).to_string()
        })?;

        for query in query_list {
            query.execute(&mut *tx).await.map_err(|err| {
                let msg = format!("query error: {:?}", err);
                error!("{}", &msg);
                Error::Error(msg).to_string()
            })?;
        }

        // 提交事务
        tx.commit().await.map_err(|err| {
            let msg = format!("commit transaction error: {:?}", err);
            error!("{}", &msg);
            Error::Error(msg).to_string()
        })?;

        Ok(get_success_response(Some(Value::Bool(true))))
    }
}
