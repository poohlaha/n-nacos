//! 流水线阶段

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::helper::git::pull::GitConfig;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::prepare::{convert_res, HttpResponse};
use crate::server::index::Server;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::{H5FileHandler, H5_INSTALLED_CMDS};
use crate::server::pipeline::props::{PipelineCommandStatus, PipelineCurrentRunStage, PipelineRunnableStageStep, PipelineRunProps, PipelineStageTask, PipelineStatus, PipelineStep, PipelineTag};
use crate::server::pipeline::runnable::PipelineRunnable;
use log::{error, info};
use sftp::config::Upload;
use sftp::upload::SftpUpload;
use std::path::PathBuf;
use std::sync::Arc;
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
                        stage_index: i as u32,
                        group_index: j as u32,
                        step_index: k as u32,
                        step: step.clone(),
                    });
                }
            }
        }

        if list.is_empty() {
            let mut error_stage = PipelineCurrentRunStage::default();
            error_stage.status = Some(PipelineStatus::Failed);
            error_stage.index = 1;
            PipelineRunnable::exec_end_log(app, &pipeline, &task.props, Some(error_stage.clone()), false, "exec stages failed, `pipeline` prop is empty !", task.order);
            return
        }

        // 执行所有的 step
        Self::exec_steps(app, pipeline, &task, list, installed_commands);
    }

    /// 执行所有的 step
    fn exec_steps(app: &AppHandle, pipeline: &Pipeline, task: &PipelineStageTask, steps: Vec<PipelineRunnableStageStep>, installed_commands: &Vec<String>) {
        let mut error_stage = PipelineCurrentRunStage::default();
        error_stage.status = Some(PipelineStatus::Failed);
        error_stage.index = 1;

        if steps.is_empty() {
            PipelineRunnable::exec_end_log(app, &pipeline, &task.props, Some(error_stage.clone()), false, "no steps need to exec !", task.order);
            return
        }

        let run = &pipeline.run;
        if run.is_none() {
            PipelineRunnable::exec_end_log(app, &pipeline, &task.props, Some(error_stage.clone()), false, "run steps failed, `run` prop is empty !", task.order);
            return
        }

        let run = run.clone().unwrap();
        let order = run.current.order;

        info!("installed_commands: {:#?}", installed_commands);

        let mut pipe = pipeline.clone();
        for step in steps.iter() {
            let result = Self::exec_step(app, &pipe, &task.props, step, installed_commands.clone(), order);

            match result {
                Ok((success, pipeline)) => {
                    if !success || pipeline.is_none() {
                        error!("exec step failed !");
                        break;
                    }

                    info!("exec step success !");
                    if let Some(pipeline) = pipeline {
                        pipe = pipeline;
                    }
                }
                Err(err) => {
                    let msg = format!("exec step error: {}", &err);
                    error!("{}", &msg);
                    PipelineRunnable::exec_end_log(app, &pipeline, &task.props, Some(error_stage.clone()), false, &msg, order);
                    break;
                }
            }
        }

        // 插入日志
        let update_result: Result<HttpResponse, String> = PipelineRunnable::update_current_pipeline(&pipe, &task.props, false, None, None, None, None, None, None, true);
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
    fn exec_step(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage: &PipelineRunnableStageStep, installed_commands: Vec<String>, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let status = stage.step.module.clone();

        let mut run_stage = PipelineCurrentRunStage::default();
        run_stage.index = stage.step_index;
        run_stage.group_index = stage.group_index;
        run_stage.step_index = stage.step_index;

        return match status {
            PipelineCommandStatus::None => Ok((false, None)),
            PipelineCommandStatus::GitPull => Self::exec_step_git_pull(app, pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::H5Install => Self::exec_step_h5_install(app, pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Pack => Self::exec_step_pack(app, pipeline, props, installed_commands.clone(), stage, &run_stage, order),
            PipelineCommandStatus::Compress => Self::exec_step_compress(app, pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Deploy => Self::exec_step_deploy(app, pipeline, props, stage, &run_stage, order),
            PipelineCommandStatus::Notice => Self::exec_step_notice(app, pipeline, props, stage, &run_stage, order),
        };
    }

    /// 代码拉取
    fn exec_step_git_pull(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = format!("【{}】", step.label);
        let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;

        let basic = &pipeline.basic;
        let config = GitConfig {
            url: basic.path.clone(),
            branch: props.branch.clone(),
            dir: dir.to_string_lossy().to_string(),
        };

        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());

        // 更改运行中 Stage
        let mut stage_clone = stage.clone();
        stage_clone.status = Some(PipelineStatus::Process);
        let pipeline = PipelineRunnable::update_stage(pipeline, props, Some(stage_clone), None)?;

        // 代码拉取
        let app_cloned = Arc::new(app.clone());
        let success = GitHandler::pull(&config, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        })?;

        // result stage
        let mut result_stage = stage.clone();
        result_stage.status = if success {
            Some(PipelineStatus::Success)
        } else {
            Some(PipelineStatus::Failed)
        };

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(result_stage.clone()), success, &format!("{} pull {} ", &pack_name, &basic.path), order);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
    }

    /// H5 依赖安装
    fn exec_step_h5_install(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = format!("【H5 {}】", step.label);
        let dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;

        // error stage
        let mut error_stage = stage.clone();
        error_stage.status = Some(PipelineStatus::Failed);

        let basic = &pipeline.basic;
        let url = &basic.path;
        let project_name = GitHandler::get_project_name_by_git(&url);
        let project_dir = dir.join(&project_name);
        if !project_dir.exists() {
            PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("project dir: {:#?} not exists, {} ", project_dir, &pack_name), order);
            return Ok((false, None));
        }

        return Self::install_h5_project(app, pipeline, props, project_dir, &project_name, order, &pack_name, stage);
    }

    /// 项目打包
    fn exec_step_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, installed_commands: Vec<String>, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;

        let pack_name = format!("【H5 {}】", step.label);
        let tag = props.tag.clone();
        let basic = &pipeline.basic;
        let mut dir = basic.path.clone();
        let project_name = GitHandler::get_project_name_by_git(&basic.path);

        // error stage
        let mut error_stage = stage.clone();
        error_stage.status = Some(PipelineStatus::Failed);

        // 获取远程地址
        if GitHandler::is_remote_url(&basic.path) {
            let config_dir = Self::get_project_path(&pipeline.server_id, &pipeline.id)?;
            let config_dir = config_dir.join(&project_name);
            if !config_dir.exists() {
                PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("exec step {} failed, project path {:#?} not exists, {}", project_name, config_dir, pack_name), order);
                return Ok((false, None));
            } else {
                dir = config_dir.to_string_lossy().to_string();
            }
        }

        match tag {
            PipelineTag::None => {}
            PipelineTag::Develop => {}
            PipelineTag::Test => {}
            PipelineTag::CAddAdd => {}
            PipelineTag::Rust => {}
            PipelineTag::Java => {}
            PipelineTag::Android => {}
            PipelineTag::Ios => {}
            PipelineTag::H5 => return Self::exec_step_h5_pack(app, pipeline, props, installed_commands.clone(), &dir, step, stage, order),
        }

        Ok((true, None))
    }

    /// 文件压缩
    fn exec_step_compress(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        // result stage
        let mut result_stage = stage.clone();
        result_stage.status =  Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(result_stage.clone()), true, &format!("{}", pack_name), order);
        return Ok((true, pipe));

    }

    /// 项目部署
    fn exec_step_deploy(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        // error stage
        let mut error_stage = stage.clone();
        error_stage.status = Some(PipelineStatus::Failed);

        let mut server = Server::default();
        server.id = pipeline.server_id.clone();

        let response = Server::get_by_id(&Server {
            id: pipeline.server_id.clone(),
            ..Default::default()
        })?;

        if response.code != 200 {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        let se = convert_res::<Server>(response);
        if se.is_none() {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        let server = se.unwrap();
        let serve = sftp::config::Server {
            host: server.ip.to_string(),
            port: server.port,
            username: server.account.to_string(),
            password: server.pwd.to_string(),
            timeout: Some(5),
        };

        SftpUpload::exec(
            serve,
            Upload {
                cmds: vec![],
                dir: "".to_string(),
                server_dir: "".to_string(),
                server_file_name: None,
                need_increment: false,
            },
            |str| {
                EventEmitter::log_event(app, str);
            },
        )?;

        // result stage
        let mut result_stage = stage.clone();
        result_stage.status =  Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(result_stage.clone()), true, &format!("{}", pack_name), order);
        return Ok((true, pipe));
    }

    /// 发送通知
    fn exec_step_notice(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, stage_step: &PipelineRunnableStageStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let step = &stage_step.step;
        let pack_name = &format!("【{}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        // result stage
        let mut result_stage = stage.clone();
        result_stage.status =  Some(PipelineStatus::Success);

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(result_stage.clone()), true, &format!("{}", pack_name), order);
        return Ok((true, pipe));
    }

    /// 获取目录
    fn get_project_path(server_id: &str, id: &str) -> Result<PathBuf, String>{
        let dir = Helper::get_project_config_dir(vec![server_id.to_string(), id.to_string(), String::from(DIR_NAME)])?;
        if let Some(dir) = dir {
            return Ok(dir);
        }

        return Err(Error::convert_string("get project path failed !"))
    }

    /// 执行 H5 打包
    fn exec_step_h5_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, installed_commands: Vec<String>, dir: &str, step: &PipelineStep, stage: &PipelineCurrentRunStage, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【H5 {}】", &step.label);
        PipelineRunnable::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let make = props.make.clone();
        let command = props.command.clone();
        let script = props.script.clone();

        // error stage
        let mut error_stage = stage.clone();
        error_stage.status = Some(PipelineStatus::Failed);

        let get_result_stage = |success: bool| {
            // result stage
            let mut result_stage = stage.clone();
            result_stage.status = if success {
                Some(PipelineStatus::Success)
            } else {
                Some(PipelineStatus::Failed)
            };

            return result_stage;
        };

        // make
        if let Some(make) = make {
            if !make.is_empty() {
                PipelineRunnable::save_log(app, "use `make` command", &pipeline.server_id, &pipeline.id, order);

                // 检查 make 命令是否存在
                let found = installed_commands.iter().find(|str| str.as_str() == "make");
                if found.is_none() {
                    PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("os not install `make` command, {}", pack_name), order);
                    return Ok((false, None));
                }

                // 执行 make 命令
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let app_cloned = Arc::new(app.clone());
                let success = Helper::exec_command(&make, &dir, move |msg| {
                    PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
                });

                let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(get_result_stage(success)), success, &format!("{}", pack_name), order);
                if pipe.is_none() {
                    return Ok((false, None));
                }

                if success {
                    return Ok((true, pipe));
                }

                return Ok((false, None));
            }
        }

        // command
        if command.is_none() || script.is_none() {
            PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("`command` or `script` is empty , {}", pack_name), order);
            return Ok((false, None));
        }

        // command
        let command = command.unwrap();
        let script = script.unwrap();

        if command.is_empty() || script.is_empty() {
            PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("`command` or `script` is empty , {}", pack_name), order);
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
            PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(error_stage.clone()), false, &format!("os not install `{}` command, {}", command, pack_name), order);
            return Ok((false, None));
        }

        // 执行 run command 命令
        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());
        let app_cloned = Arc::new(app.clone());
        let success = Helper::exec_command(&run_command, &dir, move |msg| {
            PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
        });

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(get_result_stage(success)), success, &format!("{}", pack_name), order);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
    }

    /// H5 项目安装依赖
    fn install_h5_project(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, project_path: PathBuf, project_name: &str, order: u32, pack_name: &str, stage: &PipelineCurrentRunStage) -> Result<(bool, Option<Pipeline>), String> {
        if !project_path.exists() {
            let msg = format!("install h5 project dependencies failed, project dir: {:#?} not exists !", project_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let (cmds, _) = Self::get_h5_install_cmd(project_path.clone(), project_name)?;
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
        result_stage.status = if success {
            Some(PipelineStatus::Success)
        } else {
            Some(PipelineStatus::Failed)
        };

        let pipe = PipelineRunnable::exec_end_log(app, &pipeline, &props, Some(result_stage.clone()), success, &format!("{} install h5 project {:#?} ", &pack_name, project_path), order);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
    }

    /// 获取 H5 安装的命令，动态智能判断
    fn get_h5_install_cmd(project_path: PathBuf, project_name: &str) -> Result<(Vec<String>, String), String> {
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
            info!("project {} have `pnpm-lock.yaml`, use `pnpm install`", project_name);
            // 判断是否安装了 pnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[2].to_string()) {
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[2]));
            }

            cmds.push(format!("{} install", H5_INSTALLED_CMDS[2]));
            return Ok((cmds, H5_INSTALLED_CMDS[2].to_string()));
        }

        // 1.2 yarn.lock
        let mut path = project_path.clone();
        path.push(files.get(1).unwrap());
        if path.exists() {
            info!("project {} have `yarn.lock`, use `yarn install`", project_name);
            // 判断是否安装了 yarn
            if !installed_commands.contains(&H5_INSTALLED_CMDS[1].to_string()) {
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[1]));
            }

            cmds.push(format!("{} install", H5_INSTALLED_CMDS[1]));
            return Ok((cmds, H5_INSTALLED_CMDS[1].to_string()));
        }

        // 1.3 package-lock.json
        let mut path = project_path.clone();
        path.push(files.get(2).unwrap());
        if path.exists() {
            info!("project {} have `package-lock.json`, use `cnpm install`", project_name);
            // 判断是否安装了 cnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[3].to_string()) {
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[3]));
            }

            cmds.push(format!("{} install", H5_INSTALLED_CMDS[3]));
            return Ok((cmds, H5_INSTALLED_CMDS[3].to_string()));
        }

        // 2. 判断项目中是否包含 `.npmrc` 文件
        let mut path = project_path.clone();
        path.push(".npmrc");
        if path.exists() {
            info!("project {} have `.npmrc`, use `pnpm install`", project_name);
            // 判断是否安装了 pnpm
            if !installed_commands.contains(&H5_INSTALLED_CMDS[2].to_string()) {
                cmds.push(format!("npm install -g {}", H5_INSTALLED_CMDS[2]));
            }

            cmds.push(format!("{} install", H5_INSTALLED_CMDS[2]));
            return Ok((cmds, H5_INSTALLED_CMDS[2].to_string()));
        }

        // 3. 动态智能判断, 判断 cnpm yarn npm
        if installed_commands.contains(&H5_INSTALLED_CMDS[3].to_string()) {
            info!("project {} dynamic use `cnpm install`", project_name);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[3]));
            return Ok((cmds, H5_INSTALLED_CMDS[3].to_string()));
        }

        if installed_commands.contains(&H5_INSTALLED_CMDS[1].to_string()) {
            info!("project {} dynamic use `yarn install`", project_name);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[1]));
            return Ok((cmds, H5_INSTALLED_CMDS[1].to_string()));
        }

        if installed_commands.contains(&H5_INSTALLED_CMDS[0].to_string()) {
            info!("project {} dynamic use `npm install`", project_name);
            cmds.push(format!("{} install", H5_INSTALLED_CMDS[0]));
            return Ok((cmds, H5_INSTALLED_CMDS[0].to_string()));
        }

        Ok((cmds, String::new()))
    }


    /// 发送错误消息
    fn send_error_msg(app: &AppHandle, msg: &str) -> Result<(), String> {
        PipelineRunnable::emit_error_response(app, msg);
        return Err(Error::convert_string(msg))
    }

}
