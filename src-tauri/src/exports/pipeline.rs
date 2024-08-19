//! 流水线导出列表

use crate::database::interface::Treat;
use crate::prepare::HttpResponse;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::props::PipelineRuntime;
use crate::server::pipeline::runnable::PipelineRunnable;
use crate::task::Task;
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct QueryForm {
    pub(crate) name: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone)]
pub struct Form {
    pub name: String,
    pub status: String,
}

impl Into<QueryForm> for Form {
    fn into(self) -> QueryForm {
        QueryForm { name: self.name, status: self.status }
    }
}

impl QueryForm {
    pub fn is_empty(form: &QueryForm) -> bool {
        return form.name.is_empty() && form.status.is_empty();
    }
}

/// 获取流水线列表
#[tauri::command]
pub async fn get_pipeline_list(server_id: String, form: QueryForm) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.server_id = server_id.to_string();

    let form_cloned = Arc::new(form.clone());
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move {
        let form_cloned = &*form_cloned.clone();
        Pipeline::get_query_list(&*pipe, Some(form_cloned.clone())).await
    })
    .await
}

/// 插入流水线
#[tauri::command]
pub async fn insert_pipeline(pipeline: Pipeline) -> Result<HttpResponse, String> {
    info!("insert_pipeline: {:#?}", pipeline);
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { Pipeline::insert(&*pipe).await }).await
}

/// 更新流水线
#[tauri::command]
pub async fn update_pipeline(pipeline: Pipeline) -> Result<HttpResponse, String> {
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { Pipeline::update(&*pipe).await }).await
}

/// 删除流水线
#[tauri::command]
pub async fn delete_pipeline(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { Pipeline::delete(&*pipe).await }).await
}

/// 获取流水线详情
#[tauri::command]
pub async fn get_pipeline_detail(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();

    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { Pipeline::get_by_id(&*pipe).await }).await
}

/// 运行流水线
#[tauri::command]
pub async fn pipeline_run(props: PipelineRuntime) -> Result<HttpResponse, String> {
    Task::task_param_future::<PipelineRuntime, _, _>(props.clone(), |pipe| async move { PipelineRunnable::exec(&*pipe).await }).await
}

/// 查看流水线运行历史
#[tauri::command]
pub async fn get_runtime_history(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { PipelineRunnable::get_runtime_history(&*pipe).await }).await
}

/// 批量运行流水线
#[tauri::command]
pub async fn pipeline_batch_run(list: Vec<PipelineRuntime>) -> Result<HttpResponse, String> {
    Task::task_batch_param(list, |l| async move { PipelineRunnable::batch_exec(&*l).await }).await
}

/// 查询系统已安装的 commands 列表
#[tauri::command]
pub async fn query_os_commands() -> Result<HttpResponse, String> {
    Task::task(|| Pipeline::query_os_commands()).await
}

/// 清空运行历史记录
#[tauri::command]
pub async fn clear_run_history(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();
    Task::task_param_future::<Pipeline, _, _>(pipeline, |pipe| async move { Pipeline::clear_run_history(&*pipe).await }).await
}
