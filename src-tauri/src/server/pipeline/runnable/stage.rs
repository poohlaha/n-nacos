//! 流水线阶段

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::helper::git::pull::GitConfig;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::prepare::{convert_res, get_error_response, get_success_response_by_value};
use crate::server::index::Server;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::{H5FileHandler, H5_INSTALLED_CMDS};
use crate::server::pipeline::props::{PipelineCommandStatus, PipelineRunnableStageStep, PipelineRuntime, PipelineRuntimeSnapshot, PipelineRuntimeVariable, PipelineStageTask, PipelineStatus, PipelineStep, PipelineTag};
use crate::server::pipeline::runnable::PipelineRunnable;
use log::{error, info};
use sftp::config::Upload;
use sftp::upload::SftpUpload;
use std::path::{Path, PathBuf};
use std::sync::Arc;
// use images_compressor::compressor::{Compressor, CompressorArgs};
// use images_compressor::factor::Factor;
use crate::server::pipeline::runnable::docker::DockerHandler;
use minimize::minify::Minimize;
use tauri::AppHandle;

const DIR_NAME: &str = "projects";

pub struct PipelineRunnableStage;

#[derive(Default, Debug, Clone)]
pub struct PipelineRunnableResult {
    pub(crate) success: bool,
    pub(crate) msg: String,
    pub(crate) pipeline: Option<Pipeline>,
}

impl PipelineRunnableStage {
    /// 执行 stage
    pub(crate) async fn exec(app: &AppHandle, task: &PipelineStageTask, installed_commands: &Vec<String>) -> Pipeline {
        let runtime = &task.runtime;
        let pipeline = &task.pipeline;
        let mut stages = runtime.stages.clone();
        stages.sort_by(|stage1, stage2| stage1.order.cmp(&stage2.order));

        // 提取所有的 step
        let mut list: Vec<PipelineRunnableStageStep> = Vec::new();
        for (i, stage) in stages.iter().enumerate() {
            let groups = &stage.groups;

            for (j, group) in groups.iter().enumerate() {
                let steps = &group.steps;

                for (k, step) in steps.iter().enumerate() {
                    let mut step = step.clone();
                    step.status = PipelineStatus::Queue;

                    list.push(PipelineRunnableStageStep {
                        id: pipeline.id.clone(),
                        server_id: pipeline.server_id.clone(),
                        tag: runtime.tag.clone(),
                        stage_index: (i + 1) as u32,
                        group_index: j as u32,
                        step_index: k as u32,
                        step: step.clone(),
                    });
                }
            }
        }

        if list.is_empty() {
            let mut runtime = task.runtime.clone();
            runtime.status = PipelineStatus::Failed;
            let mut pipe = pipeline.clone();
            pipe.runtime = Some(runtime);
            pipe.status = Some(PipelineStatus::Failed);
            PipelineRunnable::exec_end_log(app, &pipeline, false, "exec stages failed, `stages` prop is empty !").await;
            return pipe;
        }

        // 根据 stage_index, group_index, step_index 过滤
        let stage = runtime.stage.clone();
        let mut steps: Vec<PipelineRunnableStageStep> = Vec::new();
        for step in list.iter() {
            if step.stage_index < stage.stage_index {
                continue;
            }

            if step.stage_index == stage.stage_index {
                if step.group_index < stage.group_index {
                    continue;
                }

                if step.group_index == stage.group_index {
                    if step.step_index < stage.step_index {
                        continue;
                    }
                }
            }

            steps.push(step.clone())
        }

        if steps.len() == 0 {
            let mut runtime = task.runtime.clone();
            runtime.status = PipelineStatus::Failed;
            let mut pipe = pipeline.clone();
            pipe.runtime = Some(runtime);
            pipe.status = Some(PipelineStatus::Failed);
            PipelineRunnable::exec_end_log(app, &pipeline, false, "exec stages failed, `stages` prop is empty !").await;
            return pipe;
        }

        info!("exec filter steps list: {:#?}", steps);

        // 执行所有的 step
        return Self::exec_steps(app, &task, steps, installed_commands).await;
    }

    /// 执行所有的 step
    async fn exec_steps(app: &AppHandle, task: &PipelineStageTask, steps: Vec<PipelineRunnableStageStep>, installed_commands: &Vec<String>) -> Pipeline {
        info!("installed_commands: {:#?}", installed_commands);

        let mut pipe = task.pipeline.clone();
        let runtime = pipe.runtime.clone();
        let mut has_error: bool = false;
        let mut error_step: Option<PipelineRunnableStageStep> = None;
        for step in steps.iter() {
            // 设置运行步骤
            let run = runtime.clone();
            if let Some(mut run) = run {
                run.stage.stage_index = step.stage_index;
                run.stage.group_index = step.group_index;
                run.stage.step_index = step.step_index;
                pipe.runtime = Some(run);
            }

            let result = Self::exec_step(app, &pipe, step, installed_commands.clone()).await;
            match result {
                Ok(result) => {
                    if !result.success || result.pipeline.is_none() {
                        has_error = true;
                        error_step = Some(step.clone());
                        error!("exec step failed !");
                        break;
                    }

                    has_error = false;
                    info!("exec step success !");
                    if let Some(pipeline) = result.pipeline {
                        pipe = pipeline;
                    }
                }
                Err(err) => {
                    has_error = true;
                    error_step = Some(step.clone());
                    let msg = format!("exec step error: {}", &err);
                    error!("{}", &msg);
                    PipelineRunnable::exec_end_log(app, &pipe, false, &msg).await;
                    break;
                }
            }
        }

        // 插入日志
        info!("insert result to log ...");
        let last_step = steps.get(steps.len() - 1);

        let mut runtime = pipe.runtime.unwrap_or(PipelineRuntime::default());
        runtime.status = if has_error { PipelineStatus::Failed } else { PipelineStatus::Success };

        info!("error_step: {:#?}", error_step);
        if let Some(error_step) = error_step.clone() {
            runtime.stage.stage_index = error_step.stage_index;
            runtime.stage.group_index = error_step.group_index;
            runtime.stage.step_index = error_step.step_index;
            pipe.status = Some(PipelineStatus::Failed)
            // runtime.stage.finish_group_count = error_step.group_index
        } else {
            if let Some(last_step) = last_step {
                runtime.stage.stage_index = last_step.stage_index;
                runtime.stage.group_index = last_step.group_index;
                runtime.stage.step_index = last_step.step_index;
                runtime.stage.finished = true;
                pipe.status = Some(PipelineStatus::Success)
                // runtime.stage.finish_group_count = last_step.group_index
            }
        }

        // info!("exec steps runtime: {:#?}", runtime);
        pipe.runtime = Some(runtime);

        let success = error_step.clone().is_none();
        let msg = format!("exec task {} !", if success { "success".to_string() } else { "failed".to_string() });
        PipelineRunnable::exec_end_log(app, &pipe, success, &msg).await;
        return pipe.clone();
    }

    /// 执行步骤
    async fn exec_step(app: &AppHandle, pipeline: &Pipeline, stage: &PipelineRunnableStageStep, installed_commands: Vec<String>) -> Result<PipelineRunnableResult, String> {
        let status = stage.step.module.clone();
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());

        // 更新 step, 通知前端
        PipelineRunnable::update_stage(pipeline, &runtime).await?;
        EventEmitter::log_step_res(app, Some(get_success_response_by_value(pipeline.clone()).unwrap()));

        return match status {
            PipelineCommandStatus::None => Ok(PipelineRunnableResult::default()),
            PipelineCommandStatus::GitPull => Self::exec_step_git_pull(app, &pipeline, stage).await,
            PipelineCommandStatus::H5Install => Self::exec_step_h5_install(app, &pipeline, stage).await,
            PipelineCommandStatus::Pack => Self::exec_step_pack(app, &pipeline, installed_commands.clone(), stage).await,
            PipelineCommandStatus::Minimize => Self::exec_step_minimize(app, &pipeline, stage).await,
            PipelineCommandStatus::Compress => Self::exec_step_compress(app, &pipeline, stage).await,
            PipelineCommandStatus::Deploy => Self::exec_step_deploy(app, &pipeline, stage).await,
            PipelineCommandStatus::Docker => Self::exec_step_docker(app, &pipeline, stage),
            PipelineCommandStatus::Notice => Self::exec_step_notice(app, &pipeline, stage).await,
        };
    }

    /// 代码拉取
    async fn exec_step_git_pull(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = format!("【{}】", step.label);
        let basic = &pipeline.basic;
        let runtime = pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());

        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, runtime.order.unwrap_or(1));

        // 非远程项目, 直接成功
        if !GitHandler::is_remote_url(&basic.path) {
            let mut pipe = pipeline.clone();
            let msg = format!("{} not a remote project, pull {} ", &pack_name, &basic.path);
            let mut runtime = runtime.clone();
            runtime.status = PipelineStatus::Success;
            pipe.runtime = Some(runtime.clone());
            let pipe = PipelineRunnable::exec_end_log(app, &pipe, true, &msg).await;
            return Ok(PipelineRunnableResult { success: pipe.is_some(), msg, pipeline: pipe });
        }

        let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;
        let config = GitConfig {
            url: basic.path.clone(),
            branch: String::new(),
            dir: dir.to_string_lossy().to_string(),
        };

        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());

        // 代码拉取
        let app_cloned = Arc::new(app.clone());
        let success = GitHandler::pull(&config, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, runtime.order.unwrap_or(1));
        })?;

        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = if success { PipelineStatus::Success } else { PipelineStatus::Failed };
        pipe.runtime = Some(runtime);

        let msg = format!("{} pull {} ", &pack_name, &basic.path);
        let pipe = PipelineRunnable::exec_end_log(app, &pipe, success, &msg).await;

        let error_result = PipelineRunnableResult { success, msg: String::new(), pipeline: None };

        if pipe.is_none() {
            return Ok(error_result);
        }

        if success {
            return Ok(PipelineRunnableResult { success: true, msg, pipeline: pipe });
        }

        return Ok(error_result);
    }

    /// H5 依赖安装
    async fn exec_step_h5_install(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = format!("【{}】", step.label);
        let basic = &pipeline.basic;
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());

        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, runtime.order.unwrap_or(1));

        let url = &basic.path;
        let project_name = GitHandler::get_project_name_by_git(&url);

        let project_dir;
        if GitHandler::is_remote_url(&basic.path) {
            info!("url is remote !");

            // 远程项目
            let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;

            project_dir = dir.join(&project_name);
            if !project_dir.exists() {
                let msg = format!("project dir: {:#?} not exists, {} ", project_dir, &pack_name);
                let mut pipe = pipeline.clone();
                let mut runtime = runtime.clone();
                runtime.status = PipelineStatus::Failed;
                pipe.runtime = Some(runtime.clone());

                PipelineRunnable::exec_end_log(app, &pipe, false, &msg).await;
                return Ok(PipelineRunnableResult { success: false, msg, pipeline: None });
            }
        } else {
            project_dir = PathBuf::from(url);
        }

        let order = runtime.order.unwrap_or(1);
        return Self::install_h5_project(app, pipeline, &runtime, project_dir, &project_name, order, &pack_name).await;
    }

    /// 项目打包
    async fn exec_step_pack(app: &AppHandle, pipeline: &Pipeline, installed_commands: Vec<String>, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = format!("【H5 {}】", step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, runtime.order.unwrap_or(1));

        let tag = runtime.tag.clone();
        let basic = &pipeline.basic;
        let project_name = GitHandler::get_project_name_by_git(&basic.path);

        // 取 packDir
        let dir = match Self::get_project_pack_dir(stage_step, pipeline, &project_name) {
            Ok(dir) => dir,
            Err(err) => {
                let msg = format!("exec step pack {} failed, {}, {}", project_name, err, pack_name);
                let mut pipe = pipeline.clone();
                let mut runtime = runtime.clone();
                runtime.status = PipelineStatus::Failed;
                pipe.runtime = Some(runtime.clone());

                PipelineRunnable::exec_end_log(app, &pipe, false, &msg).await;
                return Ok(PipelineRunnableResult { success: false, msg, pipeline: None });
            }
        };

        match tag {
            PipelineTag::None => {}
            PipelineTag::Develop => {}
            PipelineTag::Test => {}
            PipelineTag::CAddAdd => {}
            PipelineTag::Rust => {}
            PipelineTag::Java => {}
            PipelineTag::Android => {}
            PipelineTag::Ios => {}
            PipelineTag::H5 => return Self::exec_step_h5_pack(app, &pipeline, installed_commands.clone(), &dir, step).await,
            PipelineTag::DockerH5 => return Self::exec_step_h5_pack(app, &pipeline, installed_commands.clone(), &dir, step).await,
        }

        Ok(PipelineRunnableResult::default())
    }

    /// 文件压缩
    async fn exec_step_minimize(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);

        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = PipelineStatus::Failed;
        pipe.runtime = Some(runtime.clone());

        let mut args = minimize::minify::Args::default();
        let components = &stage_step.step.components;
        let mut needed: bool = true;
        if components.len() > 0 {
            // isNeed
            let component_needed = components.iter().find(|com| com.prop.as_str() == "isNeed");
            if let Some(component_needed) = component_needed {
                if !component_needed.value.is_empty() {
                    let is_needed = component_needed.value.trim().to_string();
                    if is_needed.to_lowercase().as_str() == "yes" {
                        needed = true
                    } else {
                        needed = false
                    }
                }
            }

            // dir
            let component_dir = components.iter().find(|com| com.prop.as_str() == "dir");
            if let Some(component_dir) = component_dir {
                if !component_dir.value.is_empty() {
                    args.dir = component_dir.value.trim().to_string();
                }
            }

            // excludes
            let component_excludes = components.iter().find(|com| com.prop.as_str() == "excludes");
            if let Some(component_excludes) = component_excludes {
                if !component_excludes.value.is_empty() {
                    let component_excludes = component_excludes.value.trim().to_string();
                    let excludes: Vec<String> = component_excludes.split("\n").map(|str| str.to_string()).collect();
                    args.excludes = excludes;
                }
            }

            // validateJs
            let component_validate_js = components.iter().find(|com| com.prop.as_str() == "validateJs");
            if let Some(component_validate_js) = component_validate_js {
                if !component_validate_js.value.is_empty() {
                    let component_validate_js = component_validate_js.value.trim().to_string();
                    if component_validate_js.to_lowercase().as_str() == "yes" {
                        args.validate_js = true
                    } else {
                        args.validate_js = false
                    }
                }
            }

            // optimizationCss
            let component_optimization_css = components.iter().find(|com| com.prop.as_str() == "optimizationCss");
            if let Some(component_optimization_css) = component_optimization_css {
                if !component_optimization_css.value.is_empty() {
                    let component_optimization_css = component_optimization_css.value.trim().to_string();
                    if component_optimization_css.to_lowercase().as_str() == "yes" {
                        args.optimization_css = true
                    } else {
                        args.optimization_css = false
                    }
                }
            }

            if args.dir.is_empty() {
                args.dir = String::from("build")
            }

            args.dir = Self::get_deploy_path(pipeline, &args.dir, stage_step, &pack_name)?;
            PipelineRunnable::save_log(app, &format!("exec minimize step args: {:#?} ...", args), &pipeline.server_id, &pipeline.id, order);

            if needed {
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Minimize::exec(&args, move |msg| {
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                });

                if !success {
                    let error_msg = String::from("minimize failed !");
                    PipelineRunnable::exec_end_log(app, &pipe, false, &error_msg).await;
                    return Ok(PipelineRunnableResult {
                        success: false,
                        msg: error_msg,
                        pipeline: None,
                    });
                }
            }

            PipelineRunnable::save_log(app, "skip minimize step ...", &pipeline.server_id, &pipeline.id, order);
        }

        runtime.status = PipelineStatus::Success;
        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, true, &format!("{}", pack_name)).await;
        return Ok(PipelineRunnableResult {
            success: pipe.is_some(),
            msg: "".to_string(),
            pipeline: pipe,
        });
    }

    /// 图片压缩
    async fn exec_step_compress(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        /*
        let mut args = CompressorArgs {
            factor: None,
            origin: "".to_string(),
            dest: "".to_string(),
            thread_count: None,
            image_size: 0,
        };

        let mut factor = Factor {
            quality: 0.00,
            size_ratio: 0.00
        };

        let components = &stage_step.step.components;
        let mut needed: bool = true;
        if components.len() > 0 {
            // isNeed
            let component_needed = components.iter().find(|com| com.prop.as_str() == "isNeed");
            if let Some(component_needed) = component_needed {
                if !component_needed.value.is_empty() {
                    let is_needed = component_needed.value.trim().to_string();
                    if is_needed.to_lowercase().as_str() == "yes" {
                        needed = true
                    } else {
                        needed = false
                    }
                }
            }

            // origin
            let component_origin = components.iter().find(|com| com.prop.as_str() == "origin");
            if let Some(component_origin) = component_origin {
                if !component_origin.value.is_empty() {
                    let component_origin = component_origin.value.trim().to_string();
                    args.origin = component_origin
                }
            }

            // dest
            let component_dest = components.iter().find(|com| com.prop.as_str() == "dest");
            if let Some(component_dest) = component_dest {
                if !component_dest.value.is_empty() {
                    let component_dest = component_dest.value.trim().to_string();
                    args.dest = component_dest
                }
            }

            // quality
            let component_quality = components.iter().find(|com| com.prop.as_str() == "quality");
            if let Some(component_quality) = component_quality {
                if !component_quality.value.is_empty() {
                    let component_quality = component_quality.value.trim().to_string();
                    if !component_quality.is_empty() {
                        let quality = match component_quality.parse::<f32>() {
                            Ok(quality) => {
                                quality
                            }
                            Err(_) => {
                                0.0
                            }
                        };

                        factor.quality = quality
                    }
                }
            }

            // sizeRatio
            let component_ratio = components.iter().find(|com| com.prop.as_str() == "sizeRatio");
            if let Some(component_ratio) = component_ratio {
                if !component_ratio.value.is_empty() {
                    let component_ratio = component_ratio.value.trim().to_string();
                    if !component_ratio.is_empty() {
                        let ratio = match component_ratio.parse::<f32>() {
                            Ok(ratio) => {
                                ratio
                            }
                            Err(_) => {
                                0.0
                            }
                        };

                        factor.size_ratio = ratio
                    }
                }
            }

            // imageSize
            let component_image_size = components.iter().find(|com| com.prop.as_str() == "imageSize");
            if let Some(component_image_size) = component_image_size {
                if !component_image_size.value.is_empty() {
                    let component_image_size = component_image_size.value.trim().to_string();
                    if !component_image_size.is_empty() {
                        let image_size = match component_image_size.parse::<u64>() {
                            Ok(image_size) => {
                                image_size
                            }
                            Err(_) => {
                               0
                            }
                        };

                        args.image_size = image_size
                    }
                }
            }

            args.factor = Some(factor);

            if args.origin.is_empty() {
                args.origin = "build".to_string();
            }

            args.origin = Self::get_deploy_dir(pipeline, &args.origin, stage_step, &pack_name)?;
            if args.dest.is_empty() {
                args.dest = args.origin.clone();
            }

            PipelineRunnable::save_log(app, &format!("exec compress step args: {:#?}", args), &pipeline.server_id, &pipeline.id, order);

            if needed {
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Compressor::new(args).compress(move |msg|{
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                })?;

                if !success {
                    let mut result_stage = stage.clone();
                    result_stage.status = Some(PipelineStatus::Failed);
                    PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), false, "compress failed !", order, Some(PipelineStatus::Failed));
                    return Ok((false, None));
                }
            }

            PipelineRunnable::save_log(app, "skip compress step ...", &pipeline.server_id, &pipeline.id, order);
        }
         */

        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = PipelineStatus::Success;
        pipe.runtime = Some(runtime.clone());

        let pipe = PipelineRunnable::exec_end_log(app, &pipe, true, &format!("{}", pack_name)).await;
        return Ok(PipelineRunnableResult {
            success: pipe.is_some(),
            msg: "".to_string(),
            pipeline: pipe,
        });
    }

    /// 项目部署
    async fn exec_step_deploy(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);
        let snapshot = &runtime.snapshot;
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = PipelineStatus::Success;
        pipe.runtime = Some(runtime.clone());

        let mut server = Server::default();
        server.id = pipeline.server_id.clone();

        // 取 deployDir,默认为 build 目录
        let deploy_dir = Self::get_deploy_dir(&stage_step, &snapshot);
        let mut server_dir = String::new();

        let components = &stage_step.step.components;
        if components.len() > 0 {
            let component = components.iter().find(|com| com.prop.as_str() == "serverDir");
            if let Some(component) = component {
                server_dir = component.value.clone()
            }
        }

        // 判断 deploy_dir 是不是绝对路径
        let build_dir = Self::get_deploy_path(pipeline, &deploy_dir, stage_step, &pack_name)?;
        let response = Server::get_by_id(&Server {
            id: pipeline.server_id.clone(),
            ..Default::default()
        })
        .await?;

        if response.code != 200 {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        let se = convert_res::<Server>(response);
        if se.is_none() {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        // let need_increment_str: String = Self::get_value_from_variables(&props.variables, "needIncrement");
        let need_increment_str: String = String::from("No");
        let need_increment = if need_increment_str.as_str().to_lowercase() == "yes" { true } else { false };

        let server = se.unwrap();
        let serve = sftp::config::Server {
            host: server.ip.to_string(),
            port: server.port,
            username: server.account.to_string(),
            password: server.pwd.to_string(),
            timeout: Some(5),
        };

        let upload = Upload {
            cmds: vec![],
            dir: build_dir,
            server_dir: server_dir.to_string(),
            server_file_name: None,
            need_increment,
            need_delete_dir: None,
        };

        info!("sftp server config: {:#?}", serve);
        info!("sftp upload config: {:#?}", upload);

        let result = SftpUpload::exec(serve, upload, |str| {
            EventEmitter::log_event(app, &pipeline.id, str);
        });

        return match result {
            Ok(result) => {
                info!("sftp deploy result: {:#?}", result);

                runtime.status = PipelineStatus::Success;
                let msg = format!("{}", pack_name);
                let pipe = PipelineRunnable::exec_end_log(app, &pipe, true, &msg).await;
                Ok(PipelineRunnableResult { success: pipe.is_some(), msg, pipeline: pipe })
            }
            Err(err) => {
                runtime.status = PipelineStatus::Failed;

                let msg = format!("deploy error: {}, {}", err, pack_name);
                PipelineRunnable::exec_end_log(app, &pipe, false, &msg).await;
                Ok(PipelineRunnableResult { success: false, msg, pipeline: None })
            }
        };
    }

    /// docker
    fn exec_step_docker(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);
        return DockerHandler::exec(app, pipeline, stage_step);
    }
    /// 发送通知
    async fn exec_step_notice(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let order = runtime.order.unwrap_or(1);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = PipelineStatus::Success;
        pipe.runtime = Some(runtime.clone());

        let msg = format!("{}", pack_name);
        let pipe = PipelineRunnable::exec_end_log(app, &pipe, true, &msg).await;
        if pipe.is_none() {
            return Ok(PipelineRunnableResult { success: false, msg, pipeline: None });
        }

        // 向前端发送通知, 通知弹框
        EventEmitter::log_step_notice(app, Some(get_success_response_by_value(pipe.clone()).unwrap_or(get_error_response("exec step notice error !"))));
        return Ok(PipelineRunnableResult { success: true, msg, pipeline: pipe });
    }

    /// 获取目录
    fn get_project_path(server_id: &str, id: &str) -> Result<PathBuf, String> {
        let dir = Helper::get_project_config_dir(vec![server_id.to_string(), id.to_string(), String::from(DIR_NAME)])?;
        if let Some(dir) = dir {
            return Ok(dir);
        }

        return Err(Error::convert_string("get project path failed !"));
    }

    /// 执行 H5 打包
    async fn exec_step_h5_pack(app: &AppHandle, pipeline: &Pipeline, installed_commands: Vec<String>, dir: &str, step: &PipelineStep) -> Result<PipelineRunnableResult, String> {
        let pack_name = &format!("【H5 {}】", &step.label);
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());

        let snapshot = &runtime.snapshot;
        let order = runtime.order.unwrap_or(1);
        let make = snapshot.make.clone();
        let command = snapshot.command.clone();
        let script = snapshot.script.clone();

        let mut error_msg = String::new();
        let mut pipe = pipeline.clone();
        let mut runtime = runtime.clone();
        runtime.status = PipelineStatus::Failed;
        pipe.runtime = Some(runtime.clone());

        let mut error_result = PipelineRunnableResult {
            success: false,
            msg: error_msg.to_string(),
            pipeline: None,
        };

        // make
        if let Some(make) = make {
            if !make.is_empty() {
                PipelineRunnable::save_log(app, "use `make` command", &pipeline.server_id, &pipeline.id, order);

                error_msg = format!("os not install `make` command, {}", pack_name);
                // 检查 make 命令是否存在
                let found = installed_commands.iter().find(|str| str.as_str() == "make");
                if found.is_none() {
                    error_result.msg = error_msg.clone();
                    PipelineRunnable::exec_end_log(app, &pipe, false, &error_msg).await;
                    return Ok(error_result);
                }

                // 执行 make 命令
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Helper::exec_command(&make, &dir, move |msg| {
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                });

                runtime.status = if success { PipelineStatus::Success } else { PipelineStatus::Failed };

                error_msg = format!("{}", pack_name);
                let pipe = PipelineRunnable::exec_end_log(app, &pipe, success, &format!("{}", pack_name)).await;
                if pipe.is_none() {
                    return Ok(error_result);
                }

                if success {
                    return Ok(PipelineRunnableResult {
                        success,
                        msg: error_msg.to_string(),
                        pipeline: pipe,
                    });
                }

                return Ok(error_result);
            }
        }

        // command
        if command.is_empty() || script.is_empty() {
            error_msg = format!("`command` or `script` is empty , {}", pack_name);
            error_result.msg = error_msg.clone();
            PipelineRunnable::exec_end_log(app, &pipe, false, &error_msg).await;
            return Ok(error_result);
        }

        if command.is_empty() || script.is_empty() {
            error_msg = format!("`command` or `script` is empty , {}", pack_name);
            error_result.msg = error_msg.clone();
            PipelineRunnable::exec_end_log(app, &pipe, false, &error_msg).await;
            return Ok(error_result);
        }

        let mut run_command = String::new();
        run_command.push_str(command.as_str());
        run_command.push_str(" run ");
        run_command.push_str(script.as_str());
        PipelineRunnable::save_log(app, &format!("run command: {}", &run_command), &pipeline.server_id, &pipeline.id, order);

        // 检查命令是否存在
        let found = installed_commands.iter().find(|str| str.as_str() == command.as_str());
        if found.is_none() {
            error_msg = format!("os not install `{}` command, {}", command, pack_name);
            error_result.msg = error_msg.clone();
            PipelineRunnable::exec_end_log(app, &pipe, false, &error_msg).await;
            return Ok(error_result);
        }

        // 执行 run command 命令
        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());
        let app_cloned = Arc::new(app.clone());
        let success = Helper::exec_command(&run_command, &dir, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        });

        runtime.status = if success { PipelineStatus::Success } else { PipelineStatus::Failed };
        let msg = format!("{}", pack_name);
        let pipe = PipelineRunnable::exec_end_log(app, &pipe, success, &msg).await;
        if pipe.is_none() {
            return Ok(error_result);
        }

        if success {
            return Ok(PipelineRunnableResult { success: true, msg, pipeline: pipe });
        }

        return Ok(error_result);
    }

    /// H5 项目安装依赖
    async fn install_h5_project(app: &AppHandle, pipeline: &Pipeline, runtime: &PipelineRuntime, project_path: PathBuf, project_name: &str, order: u32, pack_name: &str) -> Result<PipelineRunnableResult, String> {
        if !project_path.exists() {
            let msg = format!("install h5 project dependencies failed, project dir: {:#?} not exists !", project_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let (cmds, _) = Self::get_h5_install_cmd(app, project_path.clone(), project_name, &pipeline.server_id, &pipeline.id, order)?;
        if cmds.is_empty() {
            let msg = "can not found any commands in os !";
            error!("{}", msg);
            return Err(Error::convert_string(msg));
        }

        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());

        let command = cmds.join(" && ");
        let app_cloned = Arc::new(app.clone());
        let success = Helper::exec_command(&command, &project_path.to_string_lossy().to_string(), move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        });

        let mut run = runtime.clone();
        run.status = if success { PipelineStatus::Success } else { PipelineStatus::Failed };
        let mut pipe = pipeline.clone();
        pipe.runtime = Some(run);

        let msg = format!("{} install h5 project {:#?} ", &pack_name, project_path);
        let pipe = PipelineRunnable::exec_end_log(app, &pipe, success, &msg).await;

        let error_result = PipelineRunnableResult { success, msg: String::new(), pipeline: None };

        if pipe.is_none() {
            return Ok(error_result);
        }

        if success {
            return Ok(PipelineRunnableResult { success: true, msg, pipeline: pipe });
        }

        return Ok(error_result);
    }

    /// 获取 H5 安装的命令，动态智能判断
    fn get_h5_install_cmd(app: &AppHandle, project_path: PathBuf, project_name: &str, server_id: &str, id: &str, order: u32) -> Result<(Vec<String>, String), String> {
        let installed_commands = H5FileHandler::get_installed_commands();
        if installed_commands.is_empty() {
            let msg = "`yarn`、`pnpm`、`cnpm`、`npm` not found in the os !";
            error!("{}", msg);
            return Err(Error::convert_string(msg));
        }

        let files = vec![
            String::from("pnpm-lock.yaml"),
            String::from("yarn.lock"),
            String::from("package-lock.json"), // cnpm npm 共用 package-lock.json 文件
        ];

        let mut cmds: Vec<String> = Vec::new();

        // 1. 判断是否有 pnpm-lock.yaml, yarn.lock, package-lock.json
        // 1.1 pnpm-lock.yaml
        let mut path = project_path.clone();
        path.set_file_name(files.get(0).unwrap());
        if path.exists() {
            Self::send_log(app, &format!("project {} have `pnpm-lock.yaml`, use `pnpm install`", project_name), server_id, id, order);

            // 判断是否安装了 pnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[2].to_string()) {
                Self::send_log(app, &format!("os not install {}, it will be installed !", H5_INSTALLED_CMDS[2]), server_id, id, order);
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[2]));
            }

            Self::send_log(app, &format!("run `{} install`", H5_INSTALLED_CMDS[2]), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[2]));
            return Ok((cmds, H5_INSTALLED_CMDS[2].to_string()));
        }

        // 1.2 yarn.lock
        let mut path = project_path.clone();
        path.push(files.get(1).unwrap());
        if path.exists() {
            Self::send_log(app, &format!("project {} have `yarn.lock`, use `yarn install`", project_name), server_id, id, order);

            // 判断是否安装了 yarn
            if !installed_commands.contains(&H5_INSTALLED_CMDS[1].to_string()) {
                Self::send_log(app, &format!("os not install {}, it will be installed !", H5_INSTALLED_CMDS[1]), server_id, id, order);
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[1]));
            }

            Self::send_log(app, &format!("run `{} install`", H5_INSTALLED_CMDS[1]), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[1]));
            return Ok((cmds, H5_INSTALLED_CMDS[1].to_string()));
        }

        // 1.3 package-lock.json
        let mut path = project_path.clone();
        path.push(files.get(2).unwrap());
        if path.exists() {
            Self::send_log(app, &format!("project {} have `package-lock.json`, use `cnpm install`", project_name), server_id, id, order);

            // 判断是否安装了 cnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[3].to_string()) {
                Self::send_log(app, &format!("os not install {}, it will be installed !", H5_INSTALLED_CMDS[3]), server_id, id, order);
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[3]));
            }

            Self::send_log(app, &format!("run `{} install`", H5_INSTALLED_CMDS[3]), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[3]));
            return Ok((cmds, H5_INSTALLED_CMDS[3].to_string()));
        }

        // 2. 判断项目中是否包含 `.npmrc` 文件
        let mut path = project_path.clone();
        path.push(".npmrc");
        if path.exists() {
            Self::send_log(app, &format!("project {} have `.npmrc`, use `pnpm install`", project_name), server_id, id, order);

            // 判断是否安装了 pnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[2].to_string()) {
                Self::send_log(app, &format!("os not install {}, it will be installed !", H5_INSTALLED_CMDS[2]), server_id, id, order);
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[2]));
            }

            Self::send_log(app, &format!("run `{} install`", H5_INSTALLED_CMDS[2]), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[2]));
            return Ok((cmds, H5_INSTALLED_CMDS[2].to_string()));
        }

        // 3. 动态智能判断, 判断 cnpm yarn npm
        if installed_commands.contains(&H5_INSTALLED_CMDS[3].to_string()) {
            Self::send_log(app, &format!("project {} dynamic use `cnpm install`", project_name), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[3]));
            return Ok((cmds, H5_INSTALLED_CMDS[3].to_string()));
        }

        if installed_commands.contains(&H5_INSTALLED_CMDS[1].to_string()) {
            Self::send_log(app, &format!("project {} dynamic use `yarn install`", project_name), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[1]));
            return Ok((cmds, H5_INSTALLED_CMDS[1].to_string()));
        }

        if installed_commands.contains(&H5_INSTALLED_CMDS[0].to_string()) {
            Self::send_log(app, &format!("project {} dynamic npm `yarn install`", project_name), server_id, id, order);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[0]));
            return Ok((cmds, H5_INSTALLED_CMDS[0].to_string()));
        }

        Ok((cmds, String::new()))
    }

    /// 从选中的 variables 取值
    pub(crate) fn get_value_from_variables(variables: &Vec<PipelineRuntimeVariable>, prop_name: &str) -> String {
        if variables.is_empty() {
            return String::new();
        }

        let variable = variables.iter().find(|variable| variable.name.as_str() == prop_name);
        if let Some(variable) = variable {
            return variable.value.clone();
        }

        return String::new();
    }

    /// 获取项目打包目录
    fn get_project_pack_dir(stage_step: &PipelineRunnableStageStep, pipeline: &Pipeline, project_name: &str) -> Result<String, String> {
        // 取 packDir
        let mut pack_dir = String::new();
        let components = &stage_step.step.components;
        if components.len() > 0 {
            let component = components.iter().find(|com| com.prop.as_str() == "packDir");
            if let Some(component) = component {
                pack_dir = component.value.clone()
            }
        }

        let basic = &pipeline.basic;
        let mut dir = basic.path.clone();

        // 获取远程地址
        if GitHandler::is_remote_url(&basic.path) {
            let project_dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;
            let project_dir = project_dir.join(&project_name).join(&pack_dir);
            if !project_dir.exists() {
                return Err(Error::convert_string(&format!("project dir: {:#?} not exists !", project_dir)));
            }

            dir = project_dir.to_string_lossy().to_string();
        } else {
            if Path::new(&pack_dir).is_absolute() {
                dir = pack_dir;
            } else {
                dir.push_str(&pack_dir)
            }
        }

        Ok(dir)
    }

    /// 发送日志
    fn send_log(app: &AppHandle, msg: &str, server_id: &str, id: &str, order: u32) {
        info!("{}", &msg);
        PipelineRunnable::save_log(app, &msg, server_id, id, order);
    }

    /// 获取发布目录
    fn get_deploy_path(pipeline: &Pipeline, deploy_dir: &str, stage_step: &PipelineRunnableStageStep, pack_name: &str) -> Result<String, String> {
        info!("exec step deploy, deploy dir: {}", deploy_dir);
        let mut build_dir: String = String::new();
        if Path::new(&deploy_dir).is_absolute() {
            info!("exec step deploy, deploy is absolute path !");
            build_dir = deploy_dir.to_string();
        } else {
            let basic = &pipeline.basic;
            let project_name = GitHandler::get_project_name_by_git(&basic.path);
            let project_dir = match Self::get_project_pack_dir(stage_step, pipeline, &project_name) {
                Ok(dir) => dir,
                Err(err) => {
                    return Err(Error::convert_string(&format!("exec step deploy {} failed, {}, {}", project_name, err, pack_name)));
                }
            };

            info!("exec step deploy, get project dir: {}", project_dir);
            if !project_dir.is_empty() {
                let path = Path::new(&project_dir).join(deploy_dir);
                build_dir = path.as_path().to_string_lossy().to_string();
                info!("exec step deploy, get build dir: {}", build_dir);
            }
        }

        return Ok(build_dir);
    }

    pub(crate) fn get_deploy_dir(stage_step: &PipelineRunnableStageStep, snapshot: &PipelineRuntimeSnapshot) -> String {
        let mut deploy_dir = String::from("");
        let components = &stage_step.step.components;
        if components.len() > 0 {
            let component = components.iter().find(|com| com.prop.as_str() == "deployDir");
            if let Some(component) = component {
                if !component.value.is_empty() {
                    deploy_dir = component.value.clone();
                    info!("exec step deploy found deploy_dir in components: {}", deploy_dir);
                }
            }

            // 没有找到去 selected_variables 中查找
            if deploy_dir.is_empty() {
                let dir = Self::get_value_from_variables(&snapshot.runnable_variables, "deployDir");
                if !dir.is_empty() {
                    info!("exec step deploy found deploy_dir in runnable variables: {}", deploy_dir);
                    deploy_dir = dir;
                }
            }

            // 都未找到, 直接设置默认值
            if deploy_dir.is_empty() {
                deploy_dir = String::from("build");
            }
        }

        return deploy_dir;
    }
}
