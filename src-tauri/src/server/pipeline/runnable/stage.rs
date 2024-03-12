//! 流水线阶段

use crate::database::interface::Treat;
use crate::error::Error;
use crate::event::EventEmitter;
use crate::helper::git::pull::GitConfig;
use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use crate::logger::pipeline::PipelineLogger;
use crate::prepare::{convert_res, get_error_response, HttpResponse};
use crate::server::index::Server;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::languages::h5::{H5FileHandler, H5_INSTALLED_CMDS};
use crate::server::pipeline::props::{PipelineCommandStatus, PipelineRunProps, PipelineStage, PipelineStatus, PipelineStep, PipelineTag};
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

    pub(crate) fn exec(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, steps: Vec<PipelineStep>, order: u32) {
        let installed_commands = H5FileHandler::get_installed_commands();
        info!("installed_commands: {:#?}", installed_commands);
        info!("props: {:#?}", props);

        let mut pipe = pipeline.clone();
        for step in steps.iter() {
            info!("exec step: {:#?}", step);
            let result = Self::exec_step(app, &pipe, props, step, installed_commands.clone(), order);

            match result {
                Ok((success, pipeline)) => {
                    if !success || pipeline.is_none() {
                        info!("exec step failed !");
                        break;
                    }

                    info!("exec step success !");
                    if let Some(pipeline) = pipeline {
                        pipe = pipeline;
                    }
                }
                Err(err) => {
                    info!("exec step error: {}", &err);
                    break;
                }
            }
        }

        // 插入日志
        let update_result: Result<HttpResponse, String> = PipelineRunnable::update_current_pipeline(&pipe, props, false, None, None, None, None, None, None, true);

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
    fn exec_step(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, installed_commands: Vec<String>, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let status = step.status.clone();
        // let commands = step.commands.clone();

        return match status {
            PipelineCommandStatus::None => Ok((false, None)),
            PipelineCommandStatus::GitPull => Self::exec_step_git_pull(app, pipeline, props, step, order),
            PipelineCommandStatus::H5Install => Self::exec_step_h5_install(app, pipeline, props, step, order),
            PipelineCommandStatus::Pack => Self::exec_step_pack(app, pipeline, props, installed_commands.clone(), step, order),
            PipelineCommandStatus::Compress => Self::exec_step_compress(app, pipeline, props, step, order),
            PipelineCommandStatus::Deploy => Self::exec_step_deploy(app, pipeline, props, step, order),
            PipelineCommandStatus::Notice => Self::exec_step_notice(app, pipeline, props, step, order),
        };
    }

    /// 代码拉取
    fn exec_step_git_pull(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = format!("【{}】", step.label);
        let dir = Self::get_project_path(app, &pipeline.server_id, &pipeline.id, order);
        if let Some(dir) = dir {
            let basic = &pipeline.basic;
            let config = GitConfig {
                url: basic.path.clone(),
                branch: props.branch.clone(),
                dir: dir.to_string_lossy().to_string(),
            };

            let server_id_cloned = Arc::new(pipeline.server_id.clone());
            let id_cloned = Arc::new(pipeline.id.clone());

            let result = GitHandler::pull(app, &config, move |msg| {
                PipelineLogger::save_log(msg, &*server_id_cloned, &*id_cloned, order);
            });
            return match result {
                Ok(success) => {
                    let pipe = Self::exec_end_log(app, success, &pack_name, pipeline, props, order);
                    if pipe.is_none() {
                        return Ok((false, None));
                    }

                    if success {
                        return Ok((true, pipe));
                    }

                    return Ok((false, None));
                }
                Err(err) => {
                    Self::save_log(app, &format!("{} error: {}", &pack_name, &err), &pipeline.server_id, &pipeline.id, order);
                    Ok((false, None))
                }
            };
        }

        Self::save_log(app, &format!("{} failed, can not get config dir !", &pack_name), &pipeline.server_id, &pipeline.id, order);
        Ok((false, None))
    }

    /// H5 依赖安装
    fn exec_step_h5_install(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = format!("【H5 {}】", step.label);
        let dir = Self::get_project_path(app, &pipeline.server_id, &pipeline.id, order);
        if let Some(dir) = dir {
            let basic = &pipeline.basic;
            let url = &basic.path;
            let project_name = GitHandler::get_project_name_by_git(&url);
            let project_dir = dir.join(&project_name);
            if !project_dir.exists() {
                Self::save_log(app, &format!("{} failed, project dir: {:#?} not exists !", &pack_name, project_dir), &pipeline.server_id, &pipeline.id, order);
                return Ok((false, None));
            }

            return Self::install_h5_project(app, pipeline, props, project_dir, &project_name, order, &pack_name);
        }

        Self::save_log(app, &format!("{} failed, can not get config dir !", &pack_name), &pipeline.server_id, &pipeline.id, order);
        Ok((false, None))
    }

    /// 项目打包
    fn exec_step_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, installed_commands: Vec<String>, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = format!("【H5 {}】", step.label);
        let tag = props.tag.clone();
        let basic = &pipeline.basic;
        let mut dir = basic.path.clone();
        let project_name = GitHandler::get_project_name_by_git(&basic.path);

        // 获取远程地址
        if GitHandler::is_remote_url(&basic.path) {
            let config_dir = Self::get_project_path(app, &pipeline.server_id, &pipeline.id, order);
            if let Some(config_dir) = config_dir {
                let config_dir = config_dir.join(&project_name);
                if !config_dir.exists() {
                    Self::save_log(app, &format!("exec step {} failed, project path {:#?} not exists !", pack_name, config_dir), &pipeline.server_id, &pipeline.id, order);
                    return Ok((false, None));
                } else {
                    dir = config_dir.to_string_lossy().to_string();
                }
            } else {
                Self::save_log(app, &format!("exec step {} failed, can not get project dir !", pack_name), &pipeline.server_id, &pipeline.id, order);
                return Ok((false, None));
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
            PipelineTag::H5 => return Self::exec_step_h5_pack(app, pipeline, props, installed_commands.clone(), &dir, step, order),
        }

        Ok((true, None))
    }

    /// 文件压缩
    fn exec_step_compress(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【H5 {}】", &step.label);
        Self::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let pipe = Self::exec_end_log(app, true, pack_name, pipeline, props, order);
        return Ok((true, pipe));
    }

    /// 项目部署
    fn exec_step_deploy(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【{}】", &step.label);
        Self::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

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

        let pipe = Self::exec_end_log(app, true, pack_name, pipeline, props, order);
        return Ok((true, pipe));
    }

    /// 发送通知
    fn exec_step_notice(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【{}】", &step.label);
        let pipe = Self::exec_end_log(app, true, pack_name, pipeline, props, order);
        return Ok((true, pipe));
    }

    /// 获取目录
    fn get_project_path(app: &AppHandle, server_id: &str, id: &str, order: u32) -> Option<PathBuf> {
        let dir = Helper::get_project_config_dir(vec![server_id.to_string(), String::from(DIR_NAME)]);
        return match dir {
            Ok(dir) => {
                if let Some(dir) = dir {
                    return Some(dir);
                }

                None
            }
            Err(err) => {
                Self::save_log(app, &err, server_id, id, order);
                None
            }
        };
    }

    fn exec_step_h5_pack(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, installed_commands: Vec<String>, dir: &str, step: &PipelineStep, order: u32) -> Result<(bool, Option<Pipeline>), String> {
        let pack_name = &format!("【H5 {}】", &step.label);
        Self::save_log(app, &format!("exec step {} ...", pack_name), &pipeline.server_id, &pipeline.id, order);

        let make = props.make.clone();
        let command = props.command.clone();
        let script = props.script.clone();

        // make
        if let Some(make) = make {
            if !make.is_empty() {
                Self::save_log(app, "use `make` command", &pipeline.server_id, &pipeline.id, order);

                // 检查 make 命令是否存在
                let found = installed_commands.iter().find(|str| str.as_str() == "make");
                if found.is_none() {
                    Self::save_log(app, "os not install `make` command !", &pipeline.server_id, &pipeline.id, order);
                    Self::exec_end_log(app, false, pack_name, pipeline, props, order);
                    return Ok((false, None));
                }

                // 执行 make 命令
                let server_id_cloned = Arc::new(pipeline.server_id.clone());
                let id_cloned = Arc::new(pipeline.id.clone());
                let success = Helper::exec_command(&app, &make, &dir, move |msg| {
                    PipelineLogger::save_log(msg, &*server_id_cloned, &*id_cloned, order);
                });
                let pipe = Self::exec_end_log(app, success, pack_name, pipeline, props, order);
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
            Self::save_log(app, "`command` or `script` is empty !", &pipeline.server_id, &pipeline.id, order);
            Self::exec_end_log(app, false, pack_name, pipeline, props, order);
            return Ok((false, None));
        }

        // command
        let command = command.unwrap();
        let script = script.unwrap();

        if command.is_empty() || script.is_empty() {
            Self::save_log(app, "`command` or `script` is empty !", &pipeline.server_id, &pipeline.id, order);
            Self::exec_end_log(app, false, pack_name, pipeline, props, order);
            return Ok((false, None));
        }

        let mut run_command = String::new();
        run_command.push_str(command.as_str());
        run_command.push_str(" run ");
        run_command.push_str(script.as_str());
        Self::save_log(app, &format!("run command: {}", &run_command), &pipeline.server_id, &pipeline.id, order);

        // 检查 make 命令是否存在
        let found = installed_commands.iter().find(|str| str.as_str() == command.as_str());
        if found.is_none() {
            Self::save_log(app, &format!("os not install `{}` command !", command), &pipeline.server_id, &pipeline.id, order);
            Self::exec_end_log(app, false, pack_name, pipeline, props, order);
            return Ok((false, None));
        }

        // 执行 run command 命令
        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());
        let success = Helper::exec_command(&app, &run_command, &dir, move |msg| {
            PipelineLogger::save_log(msg, &*server_id_cloned, &*id_cloned, order);
        });

        let pipe = Self::exec_end_log(app, success, pack_name, pipeline, props, order);
        if pipe.is_none() {
            return Ok((false, None));
        }

        if success {
            return Ok((true, pipe));
        }

        return Ok((false, None));
    }

    fn exec_end_log(app: &AppHandle, success: bool, msg: &str, pipeline: &Pipeline, props: &PipelineRunProps, order: u32) -> Option<Pipeline> {
        let mut step: u32 = 0;
        let mut stages: Vec<PipelineStage> = Vec::new();
        // 更新 step
        let run = pipeline.run.clone();
        if let Some(run) = run {
            let current = run.current;
            // step = current.step; // TODO
            stages = current.stages;
        }

        let mut status: Option<PipelineStatus> = None;

        let err = if success { "成功" } else { "失败" };

        let msg = format!("{} {} !", msg, err);
        Self::save_log(app, &msg, &pipeline.server_id, &pipeline.id, order);

        // 成功, 更新 step, 更新状态
        if success {
            // 完成
            if step == stages.len() as u32 {
                status = Some(PipelineStatus::Success)
            }

            step = step + 1;
        } else {
            status = Some(PipelineStatus::Failed)
        }

        // 更新流水线
        let result: Result<HttpResponse, String> = PipelineRunnable::update_current_pipeline(pipeline, props, false, status, None, None, None, Some(step), None, false);

        return match result {
            Ok(res) => {
                EventEmitter::log_step_res(app, Some(res.clone()));
                convert_res::<Pipeline>(res)
            }
            Err(err) => {
                Self::save_log(app, &err, &pipeline.server_id, &pipeline.id, order);
                EventEmitter::log_step_res(app, Some(get_error_response(&err)));
                None
            }
        };
    }

    /// H5 项目安装依赖
    fn install_h5_project(app: &AppHandle, pipeline: &Pipeline, props: &PipelineRunProps, project_path: PathBuf, project_name: &str, order: u32, pack_name: &str) -> Result<(bool, Option<Pipeline>), String> {
        if !project_path.exists() {
            let msg = format!("install h5 project dependencies failed, project dir: {:#?} not exists !", project_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let (cmds, _) = Self::get_h5_install_cmd(project_path.clone(), project_name)?;
        if cmds.is_empty() {
            let msg = "can not found any commands !";
            error!("{}", msg);
            return Err(Error::convert_string(msg));
        }

        let server_id_cloned = Arc::new(pipeline.server_id.clone());
        let id_cloned = Arc::new(pipeline.id.clone());

        let command = cmds.join(" && ");
        let success = Helper::exec_command(&app, &command, &project_path.to_string_lossy().to_string(), move |msg| {
            PipelineLogger::save_log(msg, &*server_id_cloned, &*id_cloned, order);
        });

        let pipe = Self::exec_end_log(app, success, pack_name, pipeline, props, order);
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

    /// 保存日志, 发送消息到前端
    fn save_log(app: &AppHandle, msg: &str, server_id: &str, id: &str, order: u32) {
        EventEmitter::log_event(app, msg);
        PipelineLogger::save_log(msg, server_id, id, order);
    }
}
