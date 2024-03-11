//! 流水线运行

pub(crate) mod steps;

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::props::{PipelineHistoryRun, PipelineRunProps, PipelineStatus};
use crate::server::pipeline::runnable::steps::PipelineRunnableStep;
use crate::MAX_THREAD_COUNT;
use handlers::utils::Utils;
use log::info;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

pub struct PipelineRunnable;

impl PipelineRunnable {
    pub(crate) fn exec(props: &PipelineRunProps, status: Option<PipelineStatus>) -> Result<HttpResponse, String> {
        if props.id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `id` 不能为空"));
        }

        if props.server_id.is_empty() {
            return Ok(get_error_response("运行流水线失败, `server_id` 不能为空"));
        }

        let mut pipeline = Pipeline::default();
        pipeline.id = props.id.clone();
        pipeline.server_id = props.server_id.clone();

        let date = Utils::get_date(None);
        Self::update_current_pipeline(&pipeline, props, true, status, Some(date.clone()), Some(props.clone()), None, Some(props.step), Some(props.branch.clone()), false)
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
        step: Option<u32>,
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

                        // status
                        if let Some(status) = status {
                            current.status = status;
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

                        // step
                        if let Some(step) = step {
                            current.step = step;
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
                            let order = current.order;
                            let steps = current.steps;
                            if steps.is_empty() {
                                Self::emit_error_response(app, "exec steps failed, `steps` props is empty !");
                                return;
                            }

                            // 开始执行步骤
                            return PipelineRunnableStep::exec(app, &pipeline, props, steps, order);
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
}
