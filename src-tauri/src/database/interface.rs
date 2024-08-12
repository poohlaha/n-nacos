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
}
