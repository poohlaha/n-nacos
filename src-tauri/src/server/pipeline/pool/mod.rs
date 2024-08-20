//! 线程池

use crate::database::helper::DBHelper;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::logger::Logger;
use crate::prepare::{get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::H5FileHandler;
use crate::server::pipeline::props::{PipelineRuntime, PipelineStageTask, PipelineStatus};
use crate::server::pipeline::runnable::stage::PipelineRunnableStage;
use crate::server::pipeline::runnable::{PipelineRunnable, PipelineRunnableQueryForm};
use crate::{LOOP_SEC, MAX_THREAD_COUNT, POOLS};
use futures::future::join_all;
use handlers::utils::Utils;
use log::{error, info};
use rayon::prelude::*;
use sqlx::MySql;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub struct Pool;

impl Pool {
    /// 启动线程池
    pub(crate) async fn start(app: &AppHandle) {
        Self::exec_pool_tasks(app).await;
        tokio::time::sleep(Duration::from_secs(LOOP_SEC)).await;
    }

    /// 从数据库里读取 pools
    pub(crate) async fn get_pools() {
        info!("get pools list from database ...");
        let list = match Self::get_list().await {
            Ok(list) => list,
            Err(err) => {
                error!("get pools list error: {}", err);
                return;
            }
        };

        info!("get pools list: {:#?}", list);
        info!("get pools list count: {:#?}", list.len());

        for runtime in list.iter() {
            let mut pipeline = Pipeline::default();
            pipeline.id = runtime.pipeline_id.clone();

            let pipeline_list = Pipeline::get_pipeline_list(&pipeline, None, false).await.unwrap_or_else(|err| {
                error!("get pipeline list error: {}", err);
                Vec::new()
            });

            if pipeline_list.is_empty() {
                continue;
            }

            let pipe = pipeline_list.get(0).unwrap();
            let mut pip = pipe.clone();
            pip.runtime = Some(runtime.clone());
            Self::insert_into_pool(&pip).unwrap_or(());
        }
    }

    /// 放入线程池
    pub(crate) fn insert_into_pool(pipeline: &Pipeline) -> Result<(), String> {
        info!("insert into pool: {:#?}", pipeline);

        let runtime = &pipeline.runtime;
        let mut run = PipelineRuntime::default();
        if let Some(runtime) = runtime {
            run = runtime.clone();
        }

        let task = PipelineStageTask {
            id: run.id.clone().unwrap_or(String::from("")),
            server_id: pipeline.server_id.clone(),
            tag: run.tag.clone(),
            runtime: run.clone(),
            order: run.order.unwrap_or(1),
            pipeline: pipeline.clone(),
        };

        info!("pool task: {:#?}", task);

        // 放入线程池中, 过滤重复数据
        let mut pools = POOLS.lock().unwrap();
        let mut has_found = false;
        for item in pools.iter_mut() {
            if item.id.as_str() == task.id.as_str() && &item.pipeline.id == &task.pipeline.id {
                *item = task.clone(); // 存在则替换
                has_found = true;
            }
        }

        if !has_found {
            info!("task not found in pool, it will be added !");
            pools.push(task)
        } else {
            info!("task has found in pool, it will be replaced !");
        }

        info!("pools task list: {:#?}", pools);
        info!("insert into pools success !");
        Ok(())
    }

    /// 运行任务
    pub(crate) async fn exec_pool_tasks(app: &AppHandle) {
        info!("exec pipeline pools tasks ...");
        let tasks: Vec<PipelineStageTask> = {
            let mut pools = POOLS.lock().unwrap();
            if pools.is_empty() {
                info!("pipeline pools is empty !");
                return;
            }

            // 取出 MAX_THREAD_COUNT 条数据
            let pool_len = pools.len();
            let len = pool_len.min(MAX_THREAD_COUNT as usize); // 取 pools 的长度和 5 的最小值
            let tasks: Vec<PipelineStageTask> = pools.drain(0..len).collect();
            info!("pipeline pools lave count: {}", pool_len - len);
            tasks
        };

        let installed_commands = H5FileHandler::get_installed_commands();
        let app_cloned = Arc::new(app.clone());
        let installed_commands_cloned = Arc::new(installed_commands.clone());
        let futures: Vec<_> = tasks
            .par_iter()
            .map(|task| {
                let app_clone = Arc::clone(&app_cloned);
                let commands_clone = Arc::clone(&installed_commands_cloned);
                async move {
                    Self::exec_task(&*app_clone, &*commands_clone, task).await;
                }
            })
            .collect();

        // 并发地等待所有任务完成
        join_all(futures).await;
        info!("exec pipeline pools task end !");
    }

    /// 执行步骤
    pub(crate) async fn exec_task(app: &AppHandle, installed_commands: &Vec<String>, task: &PipelineStageTask) {
        info!("exec task: {:#?}", task);

        let start_now = Instant::now();

        let mut pipeline = Pipeline::default();
        pipeline.id = task.pipeline.id.clone();
        pipeline.server_id = task.server_id.clone();

        // 获取日志目录
        let log_dir = Logger::get_log_dir(vec![pipeline.server_id.clone(), pipeline.id.clone()]);
        info!("log dir: {:#?}", log_dir);

        // 更改状态为 `执行中` 、运行开始时间、序号
        let status = PipelineStatus::Process;
        let start_time = Utils::get_date(None);

        // 1. 更新 pipeline 中状态为 Process
        // 2. 更新 pipeline_runtime 中的状态为 Process
        // 3. 更新 pipeline_runtime 中的 start_time
        let mut query_list = Vec::new();
        let pipeline_query = sqlx::query::<MySql>(
            r#"
            UPDATE pipeline SET `status` = ?, last_run_time = ? WHERE id = ?
        "#,
        )
        .bind(PipelineStatus::got(status.clone()))
        .bind(&start_time)
        .bind(&pipeline.id);
        query_list.push(pipeline_query);

        let mut runtime_sql = String::from(
            r#"
            UPDATE pipeline_runtime SET `status` = ?, start_time = ?
        "#,
        );

        if let Some(log_dir) = log_dir {
            let log = log_dir.as_path().to_string_lossy().to_string();
            runtime_sql.push_str(&format!(", log = '{}'", log));
        }

        runtime_sql.push_str(&format!("WHERE id = '{}'", task.runtime.clone().id.unwrap_or(String::new())));

        let runtime_query = sqlx::query::<MySql>(&runtime_sql).bind(PipelineStatus::got(status.clone())).bind(&start_time);
        query_list.push(runtime_query);

        let res = DBHelper::batch_commit(query_list).await;
        if res.is_err() {
            error!("{:#?}", res.err());
            return;
        }

        // 执行 stages
        let pipe = PipelineRunnableStage::exec(app, task, installed_commands).await;
        let mut runtime = pipe.clone().runtime.unwrap_or(PipelineRuntime::default());
        let elapsed_now = format!("{:.2?}", start_now.elapsed());
        runtime.duration = Some(elapsed_now);
        info!("exec task duration: {:?}", runtime.duration);

        // 更新数据库
        match PipelineRunnable::update_stage(&pipe, &runtime).await {
            Ok(_) => {
                let res = get_success_response_by_value(pipe.clone()).unwrap_or(HttpResponse::default());
                EventEmitter::log_step_res(app, Some(res));
            }
            Err(err) => {
                info!("update stage error: {} !", &err);
            }
        }
    }

    /// 从 database 中读取任务列表
    pub(crate) async fn get_list() -> Result<Vec<PipelineRuntime>, String> {
        // 获取排队中的数据
        let response = PipelineRunnable::get_runtime_list(Some(PipelineRunnableQueryForm {
            status_list: vec![PipelineStatus::got(PipelineStatus::Queue), PipelineStatus::got(PipelineStatus::Process)],
            runtime_id: None,
        }))
        .await?;

        if response.code != 200 {
            return Err(Error::convert_string("get pipeline pools from database failed !"));
        }

        let data: Vec<PipelineRuntime> = serde_json::from_value(response.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(data)
    }

    /*
    pub(crate) fn update(tasks: Vec<PipelineStageTask>) -> Result<(), String> {
        info!("update pools task count: {}", tasks.len());
        Database::update::<PipelineStageTask>(PIPELINE_POOLS_NAME, POOLS_NAME, tasks, "更新线程池失败")?;
        info!("update pools list success !");
        Ok(())
    }

    pub(crate) fn delete(tasks: Vec<PipelineStageTask>) -> Result<(), String> {
        if tasks.is_empty() {
            return Ok(());
        }

        let list = Self::get_list()?;
        if list.is_empty() {
            return Ok(());
        }

        let list: Vec<PipelineStageTask> = list
            .into_iter()
            .filter(|list_item| !tasks.iter().any(|task_item| list_item.id == task_item.id && list_item.server_id == task_item.server_id))
            .collect();

        info!("delete pool list count: {:#?}", tasks.len());
        info!("delete pool lave count: {}", list.len());
        info!("delete pool lave list: {:#?}", list);
        Self::update(list)?;

        Ok(())
    }
     */
}
