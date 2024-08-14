//! 流水线阶段

use crate::database::interface::{Treat, Treat2};
use crate::error::Error;
use crate::event::EventEmitter;
use crate::helper::git::pull::GitConfig;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::prepare::{convert_res, get_error_response, get_success_response_by_value, HttpResponse};
use crate::server::index::Server;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::{H5FileHandler, H5_INSTALLED_CMDS};
use crate::server::pipeline::props::{PipelineCommandStatus, PipelineRuntimeStage, PipelineRunnableStageStep, PipelineRuntime, PipelineStageTask, PipelineStatus, PipelineStep, PipelineTag, PipelineRuntimeVariable};
use crate::server::pipeline::runnable::PipelineRunnable;
use log::{error, info};
use sftp::config::Upload;
use sftp::upload::SftpUpload;
use std::path::{Path, PathBuf};
use std::sync::Arc;
// use images_compressor::compressor::{Compressor, CompressorArgs};
// use images_compressor::factor::Factor;
use minimize::minify::Minimize;
use tauri::AppHandle;

const DIR_NAME: &str = "projects";

pub struct PipelineRunnableStage;

impl PipelineRunnableStage {
    /// 执行 stage
    pub(crate) fn exec(app: &AppHandle, task: &PipelineStageTask, pipeline: &Pipeline, installed_commands: &Vec<String>) {
        let stages = &task.stages;

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
                        tag: pipeline.basic.tag.clone(),
                        stage_index: (i + 1) as u32,
                        group_index: j as u32,
                        step_index: k as u32,
                        step: step.clone(),
                    });
                }
            }
        }

        if list.is_empty() {
            let mut error_stage = PipelineRuntimeStage::default();
            // error_stage.status = Some(PipelineStatus::Failed);
            error_stage.stage_index = 1;
            PipelineRunnable::exec_end_log(
                app,
                &pipeline,
                &task.props,
                error_stage.clone(),
                false,
                "exec stages failed, `pipeline` prop is empty !",
                task.order,
                Some(PipelineStatus::Failed),
            );
            return;
        }

        info!("exec list: {:#?}", list);

        // 执行所有的 step
        Self::exec_steps(app, pipeline, &task, list, installed_commands);
    }

    /// 执行所有的 step
    fn exec_steps(app: &AppHandle, pipeline: &Pipeline, task: &PipelineStageTask, steps: Vec<PipelineRunnableStageStep>, installed_commands: &Vec<String>) {
        let mut error_stage = PipelineRuntimeStage::default();
        // error_stage.status = Some(PipelineStatus::Failed);
        error_stage.stage_index = 1;

        if steps.is_empty() {
            PipelineRunnable::exec_end_log(app, &pipeline, &task.props, error_stage.clone(), false, "no steps need to exec !", task.order, Some(PipelineStatus::Failed));
            return;
        }

        /*
        let run = &pipeline.run;
        if run.is_none() {
            PipelineRunnable::exec_end_log(app, &pipeline, &task.props, error_stage.clone(), false, "run steps failed, `run` prop is empty !", task.order, Some(PipelineStatus::Failed));
            return;
        }

        let run = run.clone().unwrap();
        let order = run.current.order;
         */

        info!("installed_commands: {:#?}", installed_commands);

        let mut pipe = pipeline.clone();
        let mut has_error: bool = false;
        let mut error_step: Option<PipelineRunnableStageStep> = None;
        for step in steps.iter() {
            // let result = Self::exec_step(app, &pipe, &task.props, step, installed_commands.clone(), order);
            let result = Self::exec_step(app, &pipe, &task.props, step, installed_commands.clone(), 1);

            match result {
                Ok((success, pipeline)) => {
                    if !success || pipeline.is_none() {
                        has_error = true;
                        error_step = Some(step.clone());
                        error!("exec step failed !");
                        break;
                    }

                    has_error = false;
                    info!("exec step success !");
                    if let Some(pipeline) = pipeline {
                        pipe = pipeline;
                    }
                }
                Err(err) => {
                    has_error = true;
                    error_step = Some(step.clone());
                    let msg = format!("exec step error: {}", &err);
                    error!("{}", &msg);
                    // PipelineRunnable::exec_end_log(app, &pipeline, &task.props, error_stage.clone(), false, &msg, order, Some(PipelineStatus::Failed));
                    break;
                }
            }
        }

        // 插入日志
        info!("insert result to log ...");
        let last_step = steps.get(steps.len() - 1);
        let mut success_stage = PipelineRuntimeStage::default();
        // success_stage.status = if has_error { Some(PipelineStatus::Failed) } else { Some(PipelineStatus::Success) };

        info!("error_step: {:#?}", error_step);
        if let Some(error_step) = error_step {
            success_stage.stage_index = error_step.stage_index;
            success_stage.group_index = error_step.group_index;
            success_stage.step_index = error_step.step_index;
            // success_stage.finish_group_count = error_step.group_index;
        } else {
            if let Some(last_step) = last_step {
                success_stage.stage_index = last_step.stage_index;
                success_stage.group_index = last_step.group_index;
                success_stage.step_index = last_step.step_index;
                // success_stage.finish_group_count = last_step.group_index;
                success_stage.finished = true;
            }
        }

        info!("success_stage: {:#?}", success_stage);
        let update_result: Result<HttpResponse, String> = PipelineRunnable::update_current_pipeline(&pipe, &task.props, false, None, None, None, None, success_stage, None, true);
        match update_result {
            Ok(res) => {
                info!("insert in to history list success !");
                EventEmitter::log_step_res(app, Some(res.clone()));
            }
            Err(err) => {
                info!("insert in to history list error: {} !", &err);
            }
        }
    }

    /// 执行步骤
    fn exec_step(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage: &PipelineRunnableStageStep, installed_commands: Vec<String>, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let status = stage.step.module.clone();

        let mut run_stage = PipelineRuntimeStage::default();
        run_stage.stage_index = stage.stage_index;
        run_stage.group_index = stage.group_index;
        run_stage.step_index = stage.step_index;

        // 更新 step, 通知前端
        let pipeline = PipelineRunnable::update_stage(pipeline, props, run_stage.clone(), None)?;
        EventEmitter::log_step_res(app, Some(get_success_response_by_value(pipeline.clone()).unwrap()));

        return match status {
            PipelineCommandStatus::None => Ok((false, None)),
            PipelineCommandStatus::GitPull => Self::exec_step_git_pull(app, &pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::H5Install => Self::exec_step_h5_install(app, &pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Pack => Self::exec_step_pack(app, &pipeline, props, installed_commands.clone(), stage, &run_stage, order),
            PipelineCommandStatus::Minimize => Self::exec_step_minimize(app, &pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Compress => Self::exec_step_compress(app, &pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Deploy => Self::exec_step_deploy(app, &pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Notice => Self::exec_step_notice(app, &pipeline, props, stage, &run_stage, order),
        };
    }

    /// 代码拉取
    fn exec_step_git_pull(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = format!("【{}】", step.label);
        let basic = &pipeline.basic;

        // 非远程项目, 直接成功
        if !GitHandler::is_remote_url(&basic.path) {
            let mut result_stage = stage.clone();
            // result_stage.status = Some(PipelineStatus::Success);
            PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), true, &format!("{} not a remote project, pull {} ", &pack_name, &basic.path), order, None);
            return Ok((true, Some(pipeline.clone())));
        }

        let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;

        let config = GitConfig {
            url: basic.path.clone(),
            // branch: props.branch.clone(),
            branch: String::new(),
            dir: dir.to_string_lossy().to_string(),
        };

        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());

        // 代码拉取
        let app_cloned = Arc::new(app.clone());
        let success = GitHandler::pull(&config, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        })?;

        // result stage
        let mut result_stage = stage.clone();
        // result_stage.status = if success { Some(PipelineStatus::Success) } else { Some(PipelineStatus::Failed) };

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), success, &format!("{} pull {} ", &pack_name, &basic.path), order, None);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
    }

    /// H5 依赖安装
    fn exec_step_h5_install(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = format!("【{}】", step.label);
        let basic = &pipeline.basic;

        let url = &basic.path;
        let project_name = GitHandler::get_project_name_by_git(&url);

        let project_dir;
        if GitHandler::is_remote_url(&basic.path) {
            // 远程项目
            let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;

            // error stage
            let mut error_stage = stage.clone();
            // error_stage.status = Some(PipelineStatus::Failed);

            project_dir = dir.join(&project_name);
            if !project_dir.exists() {
                PipelineRunnable::exec_end_log(
                    app,
                    &pipeline,
                    &props,
                    error_stage.clone(),
                    false,
                    &format!("project dir: {:#?} not exists, {} ", project_dir, &pack_name),
                    order,
                    Some(PipelineStatus::Failed),
                );
                return Ok((false, None));
            }
        } else {
            project_dir = PathBuf::from(url);
        }

        return Self::install_h5_project(app, pipeline, props, project_dir, &project_name, order, &pack_name, stage);
    }

    /// 项目打包
    fn exec_step_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, installed_commands: Vec<String>, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;

        let pack_name = format!("【H5 {}】", step.label);
        let tag = props.tag.clone();
        let basic = &pipeline.basic;
        let project_name = GitHandler::get_project_name_by_git(&basic.path);

        // error stage
        let mut error_stage = stage.clone();
        // error_stage.status = Some(PipelineStatus::Failed);

        // 取 packDir
        let dir = match Self::get_project_pack_dir(stage_step, pipeline, &project_name) {
            Ok(dir) => dir,
            Err(err) => {
                PipelineRunnable::exec_end_log(
                    app,
                    &pipeline,
                    &props,
                    error_stage.clone(),
                    false,
                    &format!("exec step pack {} failed, {}, {}", project_name, err, pack_name),
                    order,
                    Some(PipelineStatus::Failed),
                );
                return Ok((false, None));
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
            PipelineTag::H5 => return Self::exec_step_h5_pack(app, &pipeline, props, installed_commands.clone(), &dir, step, stage, order),
        }

        Ok((true, None))
    }

    /// 文件压缩
    fn exec_step_minimize(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

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

            args.dir = Self::get_deploy_dir(pipeline, &args.dir, stage_step, &pack_name)?;
            PipelineRunnable::save_log(app, &format!("exec minimize step args: {:#?} ...", args), &pipeline.server_id, &pipeline.id, order);

            if needed {
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Minimize::exec(&args, move |msg| {
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                });

                if !success {
                    let mut result_stage = stage.clone();
                    // result_stage.status = Some(PipelineStatus::Failed);
                    PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), false, "minimize failed !", order, Some(PipelineStatus::Failed));
                    return Ok((false, None));
                }
            }

            PipelineRunnable::save_log(app, "skip minimize step ...", &pipeline.server_id, &pipeline.id, order);
        }

        // result stage
        let mut result_stage = stage.clone();
        // result_stage.status = Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), true, &format!("{}", pack_name), order, None);
        return Ok((true, pipe));
    }

    /// 图片压缩
    fn exec_step_compress(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
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

        // result stage
        let mut result_stage = stage.clone();
        // result_stage.status = Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), true, &format!("{}", pack_name), order, None);
        return Ok((true, pipe));
    }

    /// 项目部署
    fn exec_step_deploy(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        // error stage
        let mut error_stage = stage.clone();
        // error_stage.status = Some(PipelineStatus::Failed);

        let mut server = Server::default();
        server.id = pipeline.server_id.clone();

        // 取 deployDir,默认为 build 目录
        let mut deploy_dir = String::from("");
        let mut server_dir = String::new();
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
            /*
            if deploy_dir.is_empty() {
                let dir = Self::get_value_from_variables(&props.variables, "deployDir");
                if !dir.is_empty() {
                    info!("exec step deploy found deploy_dir in selected variables: {}", deploy_dir);
                    deploy_dir = dir;
                }
            }
             */

            // 都未找到, 直接设置默认值
            if deploy_dir.is_empty() {
                deploy_dir = String::from("build");
            }

            let component = components.iter().find(|com| com.prop.as_str() == "serverDir");
            if let Some(component) = component {
                server_dir = component.value.clone()
            }
        }

        // 判断 deploy_dir 是不是绝对路径
        let build_dir = Self::get_deploy_dir(pipeline, &deploy_dir, stage_step, &pack_name)?;
        /*
        let response = Server::get_by_id(&Server {
            id: pipeline.server_id.clone(),
            ..Default::default()
        })?;

        if response.code != 200 {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }


        let se = convert_res::<Server>(response);
         */

        let se = Some(Server {
            id: "".to_string(),
            ip: "".to_string(),
            port: 0,
            account: "".to_string(),
            pwd: "".to_string(),
            name: "".to_string(),
            description: "".to_string(),
            create_time: None,
            update_time: None,
        });

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

                // result stage
                let mut result_stage = stage.clone();
                // result_stage.status = Some(PipelineStatus::Success);

                let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), true, &format!("{}", pack_name), order, None);
                Ok((true, pipe))
            }
            Err(err) => {
                // result stage
                let mut result_stage = stage.clone();
                // result_stage.status = Some(PipelineStatus::Failed);

                PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), false, &format!("deploy error: {}, {}", err, pack_name), order, Some(PipelineStatus::Failed));
                Ok((false, None))
            }
        };
    }

    /// 发送通知
    fn exec_step_notice(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, stage_step: &PipelineRunnableStageStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        // 更新 Stage
        let pipeline = PipelineRunnable::update_stage(pipeline, props, stage.clone(), None)?;

        // result stage
        let mut result_stage = stage.clone();
        // result_stage.status = Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), true, &format!("{}", pack_name), order, None);

        // 向前端发送通知, 通知弹框
        EventEmitter::log_step_notice(app, Some(get_success_response_by_value(pipe.clone()).unwrap_or(get_error_response("exec step notice error !"))));
        return Ok((true, pipe));
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
    fn exec_step_h5_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, installed_commands: Vec<String>, dir: &str, step: &PipelineStep, stage: &PipelineRuntimeStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【H5 {}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        /*
        let make = props.make.clone();
        let command = props.command.clone();
        let script = props.script.clone();
         */

        // error stage
        let mut error_stage = stage.clone();
        // error_stage.status = Some(PipelineStatus::Failed);

        let get_result_stage = |success: bool| {
            // result stage
            let mut result_stage = stage.clone();
            // result_stage.status = if success { Some(PipelineStatus::Success) } else { Some(PipelineStatus::Failed) };

            return result_stage;
        };

        // make
        /*
        if let Some(make) = make {
            if !make.is_empty() {
                PipelineRunnable::save_log(app, "use `make` command", &pipeline.server_id, &pipeline.id, order);

                // 检查 make 命令是否存在
                let found = installed_commands.iter().find(|str| str.as_str() == "make");
                if found.is_none() {
                    PipelineRunnable::exec_end_log(
                        app,
                        &pipeline,
                        &props,
                        error_stage.clone(),
                        false,
                        &format!("os not install `make` command, {}", pack_name),
                        order,
                        Some(PipelineStatus::Failed),
                    );
                    return Ok((false, None));
                }

                // 执行 make 命令
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Helper::exec_command(&make, &dir, move |msg| {
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                });

                let mut status: Option<PipelineStatus> = None;
                if !success {
                    status = Some(PipelineStatus::Failed)
                }

                let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(get_result_stage(success)), success, &format!("{}", pack_name), order, status);
                if pipe.is_none() {
                    return Ok((false, None));
                }

                if success {
                    return Ok((true, pipe));
                }

                return Ok((false, None));
            }
        }

         */

        // command
        /*
        if command.is_none() || script.is_none() {
            PipelineRunnable::exec_end_log(
                app,
                &pipeline,
                &props,
                error_stage.clone(),
                false,
                &format!("`command` or `script` is empty , {}", pack_name),
                order,
                Some(PipelineStatus::Failed),
            );
            return Ok((false, None));
        }

         */

        // command
        /*
        let command = command.unwrap();
        let script = script.unwrap();

        if command.is_empty() || script.is_empty() {
            PipelineRunnable::exec_end_log(
                app,
                &pipeline,
                &props,
                error_stage.clone(),
                false,
                &format!("`command` or `script` is empty , {}", pack_name),
                order,
                Some(PipelineStatus::Failed),
            );
            return Ok((false, None));
        }

        let mut run_command = String::new();
        run_command.push_str(command.as_str());
        run_command.push_str(" run ");
        run_command.push_str(script.as_str());
        PipelineRunnable::save_log(app, &format!("run command: {}", &run_command), &pipeline.server_id, &pipeline.id, order);

        // 检查命令是否存在
        let found = installed_commands.iter().find(|str| str.as_str() == command.as_str());
        if found.is_none() {
            PipelineRunnable::exec_end_log(
                app,
                &pipeline,
                &props,
                error_stage.clone(),
                false,
                &format!("os not install `{}` command, {}", command, pack_name),
                order,
                Some(PipelineStatus::Failed),
            );
            return Ok((false, None));
        }

        // 执行 run command 命令
        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());
        let app_cloned = Arc::new(app.clone());
        let success = Helper::exec_command(&run_command, &dir, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        });

        let mut status: Option<PipelineStatus> = None;
        if !success {
            status = Some(PipelineStatus::Failed)
        }

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(get_result_stage(success)), success, &format!("{}", pack_name), order, status);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }
         */

        return Ok((false, None));
    }

    /// H5 项目安装依赖
    fn install_h5_project(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRuntime, project_path: PathBuf, project_name: &str, order: u32, pack_name: &str, stage: &PipelineRuntimeStage) -> Result<(bool, Option<Pipeline>), String> {
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

        // result stage
        let mut result_stage = stage.clone();
        // result_stage.status = if success { Some(PipelineStatus::Success) } else { Some(PipelineStatus::Failed) };

        let mut status: Option<PipelineStatus> = None;
        if !success {
            status = Some(PipelineStatus::Failed)
        }

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, result_stage.clone(), success, &format!("{} install h5 project {:#?} ", &pack_name, project_path), order, status);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
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
    fn get_value_from_variables(variables: &Vec<PipelineRuntimeVariable>, prop_name: &str) -> String {
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
    fn get_deploy_dir(pipeline: &Pipeline, deploy_dir: &str, stage_step: &PipelineRunnableStageStep, pack_name: &str) -> Result<String, String> {
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
}
