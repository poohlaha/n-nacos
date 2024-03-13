//! 流水线运行

pub(crate) mod stage;

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::prepare::{get_error_response, get_success_response, get_success_response_by_value, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::props::{PipelineCurrentRunStage, PipelineHistoryRun, PipelineRunnableStageStep, PipelineRunProps, PipelineStage, PipelineStageTask, PipelineStatus};
use crate::server::pipeline::runnable::stage::PipelineRunnableStage;
use crate::{MAX_THREAD_COUNT, POOLS};
use handlers::utils::Utils;
use log::{error, info};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use tauri::AppHandle;
use crate::logger::pipeline::PipelineLogger;
use crate::server::pipeline::languages::h5::H5FileHandler;

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

        // 更改流水线状态为 `排队中`
        Self::update_current_pipeline(
            &pipeline,
            props,
            false,
            Some(PipelineStatus::Queue),
            None,
            Some(props.clone()),
            None,
            Some(props.stage.clone()),
            Some(props.branch.clone()),
            false
        )
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
            order: current.order
        };

        // 放入线程池中, 覆盖重复数据
        let mut pools = POOLS.lock().unwrap();

        let mut list: Vec<PipelineStageTask> = Vec::new();
        for item in pools.iter() {
            if &item.id == &task.id && &item.server_id == &task.server_id {
                list.push(task.clone())
            } else {
                list.push(item.clone())
            }
        }

        *pools = list;
        info!("insert into success !");
        Ok(())
    }

    /// 保存当前流水线
    fn update_current_pipeline(
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
                    let mut pipeline = pipe.clone();

                    if let Some(start_time) = start_time.clone() {
                        pipeline.last_run_time = Some(start_time); // 最后运行时间
                    }

                    // status
                    if let Some(status) = status.clone() {
                        pipeline.status = status;
                    }

                    let run = pipeline.run.clone();
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
                            run.history_list.push(PipelineHistoryRun {
                                id: pipeline.id.clone(),
                                server_id: pipeline.server_id.clone(),
                                current: current.clone(),
                                extra: pipeline.extra.clone(),
                            });
                        }

                        run.current = current;
                        pipeline.run = Some(run);
                    }

                    // 更新流水线
                    let res = Pipeline::update_pipeline(data, &pipeline)?;
                    if res.code != 200 {
                        return Ok(res.clone());
                    }

                    let success: bool = serde_json::from_value(res.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    if !success {
                        return Ok(get_error_response("运行流水线失败 !"));
                    }

                    // 成功后直接返回流水线数据
                    let data = serde_json::to_value(&pipeline).map_err(|err| Error::Error(err.to_string()).to_string())?;
                    return Ok(get_success_response(Some(data)));
                }

                Ok(get_error_response("根据 ID 查找流水线失败, 该流水线不存在"))
            }
            Err(_) => Ok(get_error_response("运行流水线失败, 该流水线不存在")),
        };
    }
}

impl PipelineRunnable {

    /// 执行步骤
    pub(crate) fn exec_pool_task(app: &AppHandle) {
        let mut pools = POOLS.lock().unwrap();
        if pools.is_empty() {
            return
        }

        let installed_commands = H5FileHandler::get_installed_commands();

        let app_cloned = Arc::new(app.clone());
        let installed_commands_cloned = Arc::new(installed_commands.clone());
        pools.par_iter().with_max_len(MAX_THREAD_COUNT as usize).for_each(|step| {
            Self::exec_task_step(&*app_cloned, &*installed_commands_cloned, step);
        });

        let _ = pools.split_off(MAX_THREAD_COUNT as usize);
    }

    /// 执行步骤
    pub(crate) fn exec_task_step(app: &AppHandle, installed_commands: &Vec<String>, stage: &PipelineStageTask) {
        let mut pipeline = Pipeline::default();
        pipeline.id = stage.id.clone();
        pipeline.server_id = stage.server_id.clone();

        // 更改状态为 `执行中` 、运行开始时间、序号
        let status = PipelineStatus::Process;
        let props = &stage.props;
        let mut props_stage = props.stage.clone();
        props_stage.status = Some(status.clone());

        let start_time = Utils::get_date(None);
        let res = Self::update_current_pipeline(
            &pipeline,
            props,
            true,
            Some(status.clone()),
            Some(start_time),
            None,
            None,
            Some(props_stage),
            None,
            false
        );

        let mut run_stage = PipelineCurrentRunStage::default();
        run_stage.index = 1;
        run_stage.status = Some(PipelineStatus::Failed);

        if let Some(res) = res.clone().ok() {
            let pipeline: Pipeline = serde_json::from_value(res.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string())?;
            if pipeline.is_empty() {
                Self::exec_end_log(app, &pipeline, &props, Some(run_stage.clone()), false, "exec stages failed, `pipeline` prop is empty !", stage.order);
                return ;
            }

            // 执行 stage
            PipelineRunnableStage::exec(app, stage, &pipeline, installed_commands);
            return
        }

        let msg = format!("exec stages failed, update pipeline error: {:#?} !", res.err());
        Self::exec_end_log(app, &pipeline, &props, Some(run_stage.clone()), false, &msg, stage.order);
    }

    /// 执行步骤
    pub(crate) fn exec_steps(app: &AppHandle, props: &PipelineRunProps, status: Option<PipelineStatus>, shared_data: Arc<Mutex<HttpResponse>>) {
        info!("begin to exec steps ...");
        let mut pipeline = Pipeline::default();
        pipeline.id = props.id.clone();
        pipeline.server_id = props.server_id.clone();

        let mut guard = shared_data.lock().unwrap();
        let data = Pipeline::get_by_id(&pipeline);

        // 更改状态
        if let Some(status) = status {
            let result: Result<HttpResponse, String> = PipelineRunnable::update_current_pipeline(&pipeline, props, false, Some(status), None, None, None, None, None, false);

            let flag = match result {
                Ok(_) => true,
                Err(err) => {
                    info!("update pipeline status error: {}", &err);
                    false
                }
            };

            if !flag {
                return;
            }
        }

        match data {
            Ok(res) => {
                *guard = res;
                let res_clone = guard.clone();
                let data: Result<Pipeline, String> = serde_json::from_value(res_clone.body.clone()).map_err(|err| Error::Error(err.to_string()).to_string());
                match data {
                    Ok(pipeline) => {
                        let run = pipeline.run.clone();
                        if let Some(run) = run {
                            let current = run.current.clone();
                            let stages = current.stages;
                            if stages.is_empty() {
                                Self::emit_error_response(app, "exec stages failed, `stages` props is empty !");
                                return;
                            }

                            // 开始执行步骤
                            return PipelineRunnableStage::exec(props, stages);
                        }

                        Self::emit_error_response(app, "exec steps failed, `run` props is empty !")
                    }
                    Err(err) => Self::emit_error_response(app, &format!("exec steps error: {}", &err)),
                }
            }
            Err(err) => Self::emit_error_response(app, &format!("exec steps error: {}", &err)),
        }

        info!("exec steps end ...");
    }

    fn emit_error_response(app: &AppHandle, err: &str) {
        EventEmitter::log_res(app, Some(get_error_response(&err)));
    }
}

/// MARK: 并行任务
impl PipelineRunnable {

    /// 批量执行
    pub(crate) fn batch_exec(app: &AppHandle, list: Vec<PipelineRunProps>) -> Result<Vec<HttpResponse>, String> {
        if list.is_empty() {
            return Err(Error::convert_string("batch exec pipeline list failed, `list` is empty !"));
        }

        let mut result_errors: Vec<HttpResponse> = Vec::new(); // 错误
        let mut result: Vec<PipelineRunProps> = Vec::new();

        list.iter().for_each(|props| {
            match Self::exec(props, Some(PipelineStatus::Queue)) {
                Ok(_) => result.push(props.clone()),
                Err(err) => {
                    info!("exec pipeline id: {} error: {}", &props.id, &err);
                    result_errors.push(get_error_response(&err))
                }
            };
        });

        if result.is_empty() {
            info!("exec pipeline list failed, no data need to batch run !");
            return Ok(Vec::new());
        }

        info!("exec batch pipeline list ...");
        let shared_data: Arc<Mutex<HttpResponse>> = Arc::new(Mutex::new(HttpResponse::default()));
        let app_cloned = Arc::new(app.clone());

        result.par_iter().with_max_len(MAX_THREAD_COUNT as usize).for_each(|props| {
            Self::exec_steps(&*app_cloned, props, Some(PipelineStatus::Process), Arc::clone(&shared_data));
        });

        info!("exec batch pipeline list success !");
        return Ok(Vec::new());
    }

    /// 结束
    pub(crate) fn exec_end_log(
        app: &AppHandle,
        pipeline: &Pipeline,
        props: &PipelineRunProps,
        stage: Option<PipelineCurrentRunStage>,
        success: bool,
        msg: &str,
        order: u32
    ) -> Option<Pipeline> {
        let mut status: Option<PipelineStatus> = None;

        let err = if success { "成功" } else { "失败" };

        let msg = format!("{} {} !", msg, err);
        Self::save_log(app, &msg, &pipeline.server_id, &pipeline.id, order);

        // 成功, 更新 step, 更新状态
        if success {
            status = Some(PipelineStatus::Success)
        } else {
            status = Some(PipelineStatus::Failed)
        }

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
            return Ok(pipeline.clone())
        }

        let res = PipelineRunnable::update_current_pipeline(
            &pipeline,
            props,
            false,
            status,
            None,
            None,
            None,
            stage,
            None,
            false
        )?;

        let pipe: Pipeline = serde_json::from_value(res.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(pipe)
    }

    /// 保存日志, 发送消息到前端
    pub(crate) fn save_log(app: &AppHandle, msg: &str, server_id: &str, id: &str, order: u32) {
        EventEmitter::log_event(app, msg);
        PipelineLogger::save_log(msg, server_id, id, order);
    }

}
