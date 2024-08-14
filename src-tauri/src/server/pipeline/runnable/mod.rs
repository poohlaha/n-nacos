//! 流水线运行

pub(crate) mod stage;

use crate::database::helper::DBHelper;
use crate::database::interface::{Treat, Treat2};
use crate::error::Error;
use crate::event::EventEmitter;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::pool::Pool;
use crate::server::pipeline::props::{PipelineRuntime, PipelineRuntimeSnapshot, PipelineRuntimeStage, PipelineStageTask, PipelineStatus};
use crate::POOLS;
use handlers::utils::Utils;
use lazy_static::lazy_static;
use log::{error, info};
use serde_json::Value;
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{MySql, Row};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use uuid::Uuid;

// 共享 pipeline 数据
lazy_static! {
    static ref PIPELINE: Arc<Mutex<Option<Pipeline>>> = Arc::new(Mutex::new(None));
}
pub struct PipelineRunnable;

impl PipelineRunnable {
    /// 添加到线程池中, 以最后一条为主
    pub(crate) async fn exec(props: &PipelineRuntime) -> Result<HttpResponse, String> {
        if props.pipeline_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `pipelineId` 不能为空"));
        }

        if props.server_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `serverId` 不能为空"));
        }

        let mut pipeline = Pipeline::default();
        pipeline.id = props.pipeline_id.clone();
        pipeline.server_id = props.server_id.clone();

        // 查询流水线是否存在
        info!("get pipeline by id: {}, server_id: {}", &pipeline.id, &pipeline.server_id);
        let response = Pipeline::get_query_list(&pipeline, None).await?;
        if response.code != 200 {
            return Ok(response);
        }

        let data: Vec<Pipeline> = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        if data.is_empty() {
            return Ok(get_error_response("运行流水线失败, 该流水线不存在"));
        }

        if data.len() > 1 {
            return Ok(get_error_response("运行流水线失败, 该存在多条相同的流水线"));
        }

        let pipe = data.get(0).unwrap();

        // 查询 pipeline_runtime order
        let runtime_order_query = sqlx::query(
            r#"
            select MAX(CAST(`order` AS UNSIGNED)) AS max_order FROM pipeline_runtime WHERE pipeline_id = ?
          "#,
        )
        .bind(&pipe.id);

        let mut order: u32 = 1;
        let rows = DBHelper::execute_rows(runtime_order_query).await?;
        if !rows.is_empty() {
            let row = rows.get(0);
            if let Some(row) = row {
                order = row.try_get("max_order").unwrap_or(0);
            }
        }

        // 插入到数据库
        let mut query_list: Vec<Query<MySql, MySqlArguments>> = Vec::new();
        let stage = &props.stage;
        let create_time = Utils::get_date(None);

        // 插入到 pipeline_runtime 表
        let basic_str = serde_json::to_string(&pipe.basic).unwrap_or(String::from(""));
        let stages_str = serde_json::to_string(&pipe.process_config.stages).unwrap_or(String::from(""));
        let process_query = sqlx::query::<MySql>(
            r#"
            INSERT INTO pipeline_runtime (
                id, pipeline_id, order, basic, stages, `status`, stage_index, group_index, step_index, create_time, update_time
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&pipe.id)
        .bind(format!("{}", order))
        .bind(basic_str) // basic
        .bind(stages_str) // stages
        .bind(PipelineStatus::got(props.status.clone()))
        .bind(format!("{}", stage.stage_index.clone()))
        .bind(format!("{}", stage.group_index.clone()))
        .bind(format!("{}", stage.step_index.clone()))
        .bind(&create_time)
        .bind("");
        query_list.push(process_query);

        // 插入到线程池
        Self::insert_into_pool(props, &pipe).await?;

        // 更新线程池数据库
        let pools = POOLS.lock().unwrap();
        Pool::update(pools.clone())?;

        // 更改流水线状态为 `排队中`
        // Self::update_current_pipeline(&pipeline, props, false, Some(PipelineStatus::Queue), None, Some(props.clone()), None, Some(props.stage.clone()), Some(props.branch.clone()), false)
        Ok(get_error_response(""))
    }

    /// 放入线程池
    async fn insert_into_pool(props: &PipelineRuntime, pipeline: &Pipeline) -> Result<(), String> {
        info!("insert into pool: {:#?}", props);

        /*
        let run = pipeline.run;
        if run.is_none() {
            let msg = "insert into pool failed, `run` prop is empty !";
            error!("{}", msg);
            return Err(Error::convert_string(msg));
        }

        let run = run.unwrap();
        let current = run.current;
        let stages = current.stages.clone();
        if stages.is_empty() {
            let msg = "insert into pool failed, `stages` prop is empty !";
            error!("{}", msg);
            return Err(Error::convert_string(msg));
        }
         */

        let task = PipelineStageTask {
            id: pipeline.id.clone(),
            server_id: pipeline.server_id.clone(),
            tag: pipeline.basic.tag.clone(),
            // stages: stages.clone(),
            stages: Vec::new(),
            props: props.clone(),
            // order: current.order,
            order: 1,
        };
        info!("pool task: {:#?}", task);

        // 放入线程池中, 覆盖重复数据
        let mut pools = POOLS.lock().unwrap();
        let mut has_found = false;
        for item in pools.iter_mut() {
            if &item.id == &task.id && &item.server_id == &task.server_id {
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
        info!("insert into success !");
        Ok(())
    }

    /// 保存当前流水线
    pub(crate) fn update_current_pipeline(
        pipeline: &Pipeline,
        props: &PipelineRuntime,
        update_order: bool,
        status: Option<PipelineStatus>,
        start_time: Option<String>,
        runnable: Option<PipelineRuntimeSnapshot>,
        duration: Option<u32>,
        stage: PipelineRuntimeStage,
        branch: Option<String>,
        insert_current_into_history: bool,
    ) -> Result<HttpResponse, String> {
        Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"))
        /*
        let data = Pipeline::get_pipeline_list(&pipeline);
        return match data {
            Ok(data) => {
                if data.is_empty() {
                    return Ok(get_error_response("运行流水线失败, 该流水线不存在"));
                }

                let pipe = data.iter().find(|s| &s.id == &props.id);
                if let Some(pipe) = pipe {
                    let mut pipe = pipe.clone();

                    if let Some(start_time) = start_time.clone() {
                        pipe.last_run_time = Some(start_time); // 最后运行时间
                    }

                    // status
                    if let Some(status) = status.clone() {
                        pipe.status = status;
                    }

                    let run = pipe.run.clone();
                    if let Some(mut run) = run {
                        let mut current = run.current.clone();

                        // order
                        if update_order {
                            current.order = current.order + 1;
                        }

                        // start_time
                        if let Some(start_time) = start_time {
                            current.start_time = start_time;
                        }

                        // runnable
                        if let Some(runnable) = runnable {
                            current.runnable = runnable;
                        }

                        // duration
                        if let Some(duration) = duration {
                            current.duration = duration;
                        }

                        // stage
                        if let Some(stage) = stage {
                            current.stage = stage;
                        }

                        // status
                        if let Some(status) = status {
                            current.stage.status = Some(status.clone());
                        }

                        // branch
                        if let Some(branch) = branch {
                            run.branch = branch;
                        }

                        // history
                        if insert_current_into_history {
                            let history = pipe.clone();
                            if let Some(mut run) = pipeline.run.clone() {
                                run.history_list = Vec::new()
                            }
                            run.history_list.push(history);
                        }

                        run.current = current;
                        pipe.run = Some(run);
                    }

                    // 更新流水线
                    let res = Pipeline::update_pipeline(data, &pipe)?;
                    if res.code != 200 {
                        return Ok(res.clone());
                    }

                    let success: bool = serde_json::from_value(res.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    if !success {
                        return Ok(get_error_response("运行流水线失败 !"));
                    }

                    // 成功后直接返回流水线数据
                    let data = serde_json::to_value(&pipe).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    return Ok(get_success_response(Some(data)));
                }

                Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"))
            }
            Err(_) => Ok(get_error_response("运行流水线失败, 该流水线不存在")),
        };
         */
    }
}

/// MARK: 并行任务
impl PipelineRunnable {
    /// 批量执行
    pub(crate) async fn batch_exec(list: &Vec<PipelineRuntime>) -> Result<HttpResponse, String> {
        if list.is_empty() {
            error!("batch exec pipeline list failed, `list` is empty !");
            return Ok(get_error_response("batch exec pipeline list failed, `list` is empty !"));
        }

        let mut result_errors: Vec<HttpResponse> = Vec::new(); // 错误
        let mut result: Vec<PipelineRuntime> = Vec::new();

        // 插入到线程池
        /*
        list.iter().for_each(|props| async move {
            match Self::exec(props).await {
                Ok(_) => result.push(props.clone()),
                Err(err) => {
                    error!("exec pipeline id: {} error: {}", &props.id, &err);
                    result_errors.push(get_error_response(&err))
                }
            };
        });
         */

        if result.is_empty() {
            error!("exec pipeline list failed, no data need to batch run !");
            return Ok(get_error_response("exec pipeline list failed, no data need to batch run !"));
        }

        if !result_errors.is_empty() {
            return Ok(get_success_response(Some(serde_json::Value::String(String::from("some pipeline into pools error !")))));
        }

        // 更新线程池数据库
        let pools = POOLS.lock().unwrap();
        Pool::update(pools.clone())?;

        info!("insert into pools success !");
        return Ok(get_success_response(None));
    }

    /// 结束
    pub(crate) fn exec_end_log(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage: PipelineRuntimeStage, success: bool, msg: &str, order: u32, status: Option<PipelineStatus>) -> Option<Pipeline> {
        let err = if success { "成功" } else { "失败" };

        let msg = format!("{} {} !", msg, err);
        Self::save_log(app, &msg, &pipeline.server_id, &pipeline.id, order);

        // 更新流水线
        let result = Self::update_stage(pipeline, props, stage, status);
        return match result {
            Ok(res) => {
                EventEmitter::log_step_res(app, Some(get_success_response_by_value(res.clone()).unwrap()));
                Some(res.clone())
            }
            Err(err) => {
                Self::save_log(app, &err, &pipeline.server_id, &pipeline.id, order);
                EventEmitter::log_step_res(app, Some(get_error_response(&err)));
                None
            }
        };
    }

    /// 更新 stage 状态
    pub(crate) fn update_stage(pipeline: &Pipeline, props: &PipelineRuntime, stage: PipelineRuntimeStage, status: Option<PipelineStatus>) -> Result<Pipeline, String> {
        if status.is_none() {
            return Ok(pipeline.clone());
        }

        let res = PipelineRunnable::update_current_pipeline(&pipeline, props, false, status, None, None, None, stage, None, false)?;

        let pipe: Pipeline = serde_json::from_value(res.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(pipe)
    }

    /// 保存日志, 发送消息到前端
    pub(crate) fn save_log(app: &AppHandle, msg: &str, server_id: &str, id: &str, order: u32) {
        EventEmitter::log_event(app, id, msg);
        PipelineLogger::save_log(msg, server_id, id, order);
    }
}
