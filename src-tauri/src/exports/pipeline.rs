//! 流水线导出列表

use crate::database::interface::Treat;
use crate::prepare::HttpResponse;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::props::{PipelineRunProps, PipelineStatus};
use crate::server::pipeline::runnable::PipelineRunnable;
use crate::task::Task;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct QueryForm {
    pub(crate) name: String,
    pub(crate) status: String,
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
    Task::task_param(pipeline, move |pipeline| Pipeline::get_query_list(pipeline, &*form_cloned)).await
}

/// 插入流水线
#[tauri::command]
pub async fn insert_pipeline(pipeline: Pipeline) -> Result<HttpResponse, String> {
    Task::task_param::<Pipeline, _>(pipeline, |pipeline| Pipeline::insert(&pipeline)).await
}

/// 更新流水线
#[tauri::command]
pub async fn update_pipeline(pipeline: Pipeline) -> Result<HttpResponse, String> {
    Task::task_param::<Pipeline, _>(pipeline, |pipeline| Pipeline::update(&pipeline)).await
}

/// 删除流水线
#[tauri::command]
pub async fn delete_pipeline(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();

    Task::task_param(pipeline, |pipeline| Pipeline::delete(pipeline)).await
}

/// 获取流水线详情
#[tauri::command]
pub async fn get_pipeline_detail(id: String, server_id: String) -> Result<HttpResponse, String> {
    let mut pipeline = Pipeline::default();
    pipeline.id = id.to_string();
    pipeline.server_id = server_id.to_string();

    Task::task_param(pipeline, |pipeline| Pipeline::get_by_id(&pipeline)).await
}

/// 运行流水线
#[tauri::command]
pub async fn pipeline_run(props: PipelineRunProps) -> Result<HttpResponse, String> {
    Task::task_param(props.clone(), |pipeline| PipelineRunnable::exec(pipeline, Some(PipelineStatus::Process))).await
}

/// 启动异步线程运行步骤
#[tauri::command]
pub fn exec_steps(app: AppHandle, props: PipelineRunProps) {
    // 执行异步任务, 运行流水线步骤
    let app_cloned = Arc::new(app.clone());
    let props_cloned = Arc::new(props.clone());
    async_std::task::spawn(async move {
        log::info!("async_std task begin ...");
        PipelineRunnable::exec_steps(&*app_cloned, &*props_cloned, None, Arc::new(Mutex::new(HttpResponse::default())))
    });
}

/// 批量运行流水线
#[tauri::command]
pub async fn pipeline_batch_run(app: AppHandle, list: Vec<PipelineRunProps>) -> Result<Vec<HttpResponse>, String> {
    let app_cloned = Arc::new(app.clone());
    Task::task_batch_param(list, move |list| PipelineRunnable::batch_exec(&*app_cloned, list.clone())).await
}

/// 查询系统已安装的 commands 列表
#[tauri::command]
pub async fn query_os_commands() -> Result<HttpResponse, String> {
    Task::task(|| Pipeline::query_os_commands()).await
}

