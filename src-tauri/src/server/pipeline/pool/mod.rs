//! 线程池

use crate::database::Database;
use crate::error::Error;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::H5FileHandler;
use crate::server::pipeline::props::{PipelineRuntimeStage, PipelineStageTask, PipelineStatus};
use crate::server::pipeline::runnable::PipelineRunnable;
use crate::{LOOP_SEC, MAX_THREAD_COUNT, POOLS};
use handlers::utils::Utils;
use log::{error, info};
use rayon::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;
use crate::server::pipeline::runnable::stage::PipelineRunnableStage;

/// 存储流水线线程名称
const PIPELINE_POOLS_NAME: &str = "pipeline_pools";

/// 存储服务器名称
const POOLS_NAME: &str = "pools";

pub struct Pool;

impl Pool {
    /// 启动线程池
    pub(crate) fn start(app: &AppHandle) {
        Self::exec_pool_tasks(app);
        thread::sleep(Duration::from_secs(LOOP_SEC));
    }

    /// 从数据库里读取 pools
    pub(crate) fn get_pools() {
        info!("get pools list from database ...");
        let list = match Self::get_list() {
            Ok(list) => list,
            Err(err) => {
                error!("get pools list error: {}", err);
                return;
            }
        };

        info!("get pools list: {:#?}", list);
        info!("get pools list count: {:#?}", list.len());

        let mut pools = POOLS.lock().unwrap();
        *pools = list
    }

    /// 运行任务
    pub(crate) fn exec_pool_tasks(app: &AppHandle) {
        info!("exec pipeline pools tasks ...");
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

        // 同步数据库
        match Self::delete(tasks.clone()) {
            Ok(_) => {}
            Err(err) => {
                error!("update pipeline pools database error: {}", err);
                return;
            }
        }

        let installed_commands = H5FileHandler::get_installed_commands();
        let app_cloned = Arc::new(app.clone());
        let installed_commands_cloned = Arc::new(installed_commands.clone());
        tasks.par_iter().for_each(|step| {
            Self::exec_task_step(&*app_cloned, &*installed_commands_cloned, step);
        });

        info!("exec pipeline pools task end !");
    }

    /// 执行步骤
    pub(crate) fn exec_task_step(app: &AppHandle, installed_commands: &Vec<String>, stage: &PipelineStageTask) {
        info!("exec task step: {:#?}", stage);
        let mut pipeline = Pipeline::default();
        pipeline.id = stage.id.clone();
        pipeline.server_id = stage.server_id.clone();

        // 更改状态为 `执行中` 、运行开始时间、序号
        let status = PipelineStatus::Process;
        let props = &stage.props;
        let mut props_stage = props.stage.clone();
        // props_stage.status = Some(status.clone());

        let start_time = Utils::get_date(None);
        let res = PipelineRunnable::update_current_pipeline(&pipeline, props, true, Some(status.clone()), Some(start_time), None, None, props_stage, None, false);

        let mut error_stage = PipelineRuntimeStage::default();
        error_stage.stage_index = 1;
        // error_stage.status = Some(PipelineStatus::Failed);

        if let Some(res) = res.clone().ok() {
            let pipe: Result<Pipeline, String> = serde_json::from_value(res.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string());
            if pipe.is_err() {
                PipelineRunnable::exec_end_log(app, &pipeline, &props, error_stage.clone(), false, "exec stages failed, `pipeline` prop is empty !", stage.order, Some(PipelineStatus::Failed));
                return;
            }

            // 执行 stage
            let pipe = pipe.unwrap();
            PipelineRunnableStage::exec(app, stage, &pipe, installed_commands);
            return;
        }

        let msg = format!("update pipeline error: {:#?}, exec task step ", res.err());
        PipelineRunnable::exec_end_log(app, &pipeline, &props, error_stage.clone(), false, &msg, stage.order, Some(PipelineStatus::Failed));
    }

    /// 从 database 中读取任务列表
    pub(crate) fn get_list() -> Result<Vec<PipelineStageTask>, String> {
        let response = Database::get_list::<PipelineStageTask>(PIPELINE_POOLS_NAME, POOLS_NAME)?;
        if response.code != 200 {
            return Err(Error::convert_string("get pipeline pools from database failed !"));
        }

        let data: Vec<PipelineStageTask> = serde_json::from_value(response.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(data)
    }

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
}
