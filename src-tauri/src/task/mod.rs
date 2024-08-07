//! task 任务

use crate::prepare::HttpResponse;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;

pub struct Task;

impl Task {
    /// 异步任务
    pub(crate) async fn task<F>(func: F) -> Result<HttpResponse, String>
    where
        F: FnOnce() -> Result<HttpResponse, String> + Send + 'static,
    {
        let result = async_std::task::spawn(async move { func() });

        return result.await;
    }

    /// 异步任务
    pub(crate) async fn task_param<T, F>(body: T, func: F) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
        F: FnOnce(&T) -> Result<HttpResponse, String> + Send + 'static,
    {
        let body_cloned = Arc::new(body.clone());
        let result = async_std::task::spawn(async move { func(&*body_cloned) });

        return result.await;
    }

    /// 异步任务
    pub(crate) async fn task_param_future<T, F, Fut>(body: T, func: F) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
        F: FnOnce(Arc<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<HttpResponse, String>> + Send + 'static,
    {
        let body_cloned = Arc::new(body.clone());
        let result = async_std::task::spawn(async move { func(body_cloned).await });
        return result.await;
    }

    /// 批量异步任务
    pub(crate) async fn task_batch_param<T, F>(body: T, func: F) -> Result<HttpResponse, String>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
        F: FnOnce(&T) -> Result<HttpResponse, String> + Send + 'static,
    {
        let body_cloned = Arc::new(body.clone());
        let result = async_std::task::spawn(async move { func(&*body_cloned) });
        return result.await;
    }
}
