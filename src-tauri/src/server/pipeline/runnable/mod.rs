//! 流水线运行

pub(crate) mod stage;

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::pool::Pool;
use crate::server::pipeline::props::{PipelineCurrentRunStage, PipelineRunProps, PipelineStageTask, PipelineStatus};
use crate::POOLS;
use lazy_static::lazy_static;
use log::{error, info};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

// 共享 pipeline 数据
lazy_static! {
    static ref PIPELINE: Arc<Mutex<Option<Pipeline>>> = Arc::new(Mutex::new(None));
}
pub struct PipelineRunnable;

impl PipelineRunnable {
    /// 添加到线程池中,需要过滤重复数据,以最后一条为主
    pub(crate) fn exec(props: &PipelineRunProps) -> Result<HttpResponse, String> {
        if props.id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `id` 不能为空"));
        }

        if props.server_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `server_id` 不能为空"));
        }

        let mut pipeline = Pipeline::default();
        pipeline.id = props.id.clone();
        pipeline.server_id = props.server_id.clone();

        // 插入到线程池
        Self::insert_into_pool(props, &pipeline)?;

        // 更新线程池数据库
        let pools = POOLS.lock().unwrap();
        Pool::update(pools.clone())?;

        // 更改流水线状态为 `排队中`
        Self::update_current_pipeline(&pipeline, props, false, Some(PipelineStatus::Queue), None, Some(props.clone()), None, Some(props.stage.clone()), Some(props.branch.clone()), false)
    }

    /// 放入线程池
    fn insert_into_pool(props: &PipelineRunProps, pipeline: &Pipeline) -> Result<(), String> {
        info!("insert into pool: {:#?}", props);

        let res = Pipeline::get_by_id(&pipeline)?;
        let pipeline: Pipeline = serde_json::from_value(res.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;

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

        let task = PipelineStageTask {
            id: pipeline.id.clone(),
            server_id: pipeline.server_id.clone(),
            tag: pipeline.basic.tag.clone(),
            stages: stages.clone(),
            props: props.clone(),
            order: current.order,
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
        props: &PipelineRunProps,
        update_order: bool,
        status: Option<PipelineStatus>,
        start_time: Option<String>,
        runnable: Option<PipelineRunProps>,
        duration: Option<u32>,
        stage: Option<PipelineCurrentRunStage>,
        branch: Option<String>,
        insert_current_into_history: bool,
    ) -> Result<HttpResponse, String> {
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
    }
}

/// MARK: 并行任务
impl PipelineRunnable {
    /// 批量执行
    pub(crate) fn batch_exec(list: Vec<PipelineRunProps>) -> Result<HttpResponse, String> {
        if list.is_empty() {
            error!("batch exec pipeline list failed, `list` is empty !");
            return Ok(get_error_response("batch exec pipeline list failed, `list` is empty !"));
        }

        let mut result_errors: Vec<HttpResponse> = Vec::new(); // 错误
        let mut result: Vec<PipelineRunProps> = Vec::new();

        // 插入到线程池
        list.iter().for_each(|props| {
            match Self::exec(props) {
                Ok(_) => result.push(props.clone()),
                Err(err) => {
                    error!("exec pipeline id: {} error: {}", &props.id, &err);
                    result_errors.push(get_error_response(&err))
                }
            };
        });

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
    pub(crate) fn exec_end_log(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage: Option<PipelineCurrentRunStage>, success: bool, msg: &str, order: u32, status: Option<PipelineStatus>) -> Option<Pipeline> {
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
    pub(crate) fn update_stage(pipeline: &Pipeline, props: &PipelineRunProps, stage: Option<PipelineCurrentRunStage>, status: Option<PipelineStatus>) -> Result<Pipeline, String> {
        if stage.is_none() && status.is_none() {
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
