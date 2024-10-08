//! 数据库接口

use crate::prepare::HttpResponseData;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[allow(dead_code)]
pub trait TreatBody: Serialize + DeserializeOwned + 'static {}

#[async_trait]
pub trait Treat<R>
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
