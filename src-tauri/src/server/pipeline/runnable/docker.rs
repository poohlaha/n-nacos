//! Docker, 可以使用第三方库 `bollard`

use crate::database::interface::Treat;
use crate::error::Error;
use crate::helper::index::Helper;
use crate::server::index::Server;
use crate::server::pipeline::index::Pipeline;
use crate::server::pipeline::props::{PipelineRunnableStageStep, PipelineRuntime, PipelineRuntimeSnapshot, PipelineStepComponent};
use crate::server::pipeline::runnable::stage::{PipelineRunnableResult, PipelineRunnableStage};
use crate::server::pipeline::runnable::PipelineRunnable;
use handlers::command::CommandHandler;
use handlers::file::FileHandler;
use handlers::utils::Utils;
use log::{error, info};
use regex::Regex;
use sftp::sftp::SftpHandler;
use std::io::{Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use ssh2::Session;
use tauri::AppHandle;

pub struct DockerHandler;

#[derive(Default, Debug, Clone)]
pub struct DockerConfig {
    dockerfile: String,
    address: String,
    image: String,
    version: String,
    namespace: String,
    user: String,
    password: String,
    need_push: String,
    nginx_path: String,
    nginx_content: String,
    platform: String,
}

impl DockerConfig {
    pub fn is_empty(config: &DockerConfig) -> bool {
        if config.need_push == "Yes" {
            return config.dockerfile.is_empty() || config.image.is_empty() || config.address.is_empty() || config.namespace.is_empty() || config.user.is_empty() || config.password.is_empty();
        }

        return config.dockerfile.is_empty() || config.image.is_empty();
    }
}

impl DockerHandler {
    pub(crate) async fn exec(app: &AppHandle, pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep) -> Result<PipelineRunnableResult, String> {
        // let step = stage_step.step.clone();
        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let snapshot = &runtime.snapshot;

        let basic = &runtime.basic;
        let result = PipelineRunnableResult {
            success: true,
            msg: "".to_string(),
            pipeline: Some(pipeline.clone()),
        };

        if basic.is_none() {
            return Err(Error::convert_string("run pipeline failed, `runtime basic filed` is empty!"));
        }

        let order = runtime.order.unwrap_or(1);
        let basic = basic.clone().unwrap();
        let docker_config = Self::exec_docker_config(pipeline, stage_step, snapshot);
        PipelineRunnable::save_log(app, &format!("docker config: {:#?}", docker_config), &pipeline.server_id, &pipeline.id, order);

        if DockerConfig::is_empty(&docker_config) {
            return Err(Error::convert_string("run pipeline failed, `docker config some field` is empty!"));
        }

        // 判断本机有没有安装docker
        let success = Helper::check_installed_command("docker");
        if !success {
            return Err(Error::convert_string("no `docker` installed in os !"));
        }

        // 判断 docker 是否已启动
        let str = CommandHandler::exec_command_result("docker info");
        if str.is_empty() {
            return Err(Error::convert_string("`docker` is not running !"));
        }

        // 获取 docker pull | docker push 命令
        let mut commands: Vec<String> = Vec::new();
        commands.push(format!("cd {}", basic.path));

        let time = Utils::get_date(Some("%Y%m%d%H%M%S".to_string()));
        let deploy_dir = PipelineRunnableStage::get_deploy_dir(&stage_step, &snapshot);

        // 创建 nginx.conf 文件
        let nginx_file_name = format!("nginx_{}.conf", time); // nginx 文件名
        let nginx_file_path = Path::new(&basic.path).join(&nginx_file_name);
        let nginx_file_path_str = nginx_file_path.to_string_lossy().to_string();
        FileHandler::write_to_file_when_clear(&nginx_file_path_str, &docker_config.nginx_content)?;

        let mut dockerfile_content = docker_config.dockerfile.clone();
        if docker_config.need_push == "Yes" {
            // 添加 nginx
            if !docker_config.nginx_path.is_empty() {
                let mut content: Vec<String> = dockerfile_content.lines().map(String::from).collect();
                content.push(format!("ADD {} {}", nginx_file_name, docker_config.nginx_path));
                content.push(format!("COPY {} ./", deploy_dir));
                dockerfile_content = content.join("\n");
            }
        }

        // 创建 Dockerfile 文件
        let dockerfile_file_name = format!("Dockerfile_{}", time); // dockerfile 文件名
        let dockerfile_file_path = Path::new(&basic.path).join(&dockerfile_file_name);
        let dockerfile_file_path_str = dockerfile_file_path.to_string_lossy().to_string();
        FileHandler::write_to_file_when_clear(&dockerfile_file_path_str, &dockerfile_content)?;

        let image = format!("{}/{}/{}:{}", docker_config.address, docker_config.namespace, docker_config.image, docker_config.version);
        if docker_config.need_push == "Yes" {
            let pull_nginx_command = Self::exec_docker_pull_nginx(&docker_config);
            if pull_nginx_command.is_empty() {
                FileHandler::delete_file(&dockerfile_file_path_str)?; // 删除 Dockerfile 文件
                FileHandler::delete_file(&nginx_file_path_str)?; // 删除 nginx.conf 文件
                return Err(Error::convert_string("can not get pull nginx command !"));
            }

            commands.push(format!("docker login {} --username {} --password {}", docker_config.address, docker_config.user, docker_config.password));
            commands.push(pull_nginx_command);
            commands.push(format!("docker buildx build -f ./{} -t {} --platform {} -o type=docker .", dockerfile_file_name, image, docker_config.platform));
            commands.push(format!("docker push {}", image));
        } else {
            // 不需要推送，直接打本地包
            commands.push(format!(
                "docker buildx build -f ./{} -t {}:{} --platform {} -o type=docker .",
                dockerfile_file_name, docker_config.image, docker_config.version, docker_config.platform
            ));
        }

        info!("docker commands: {:#?}", commands);

        let order = runtime.order.unwrap_or(1);
        for command in commands.iter() {
            let server_id_cloned = Arc::new(pipeline.server_id.clone());
            let id_cloned = Arc::new(pipeline.id.clone());
            let app_cloned = Arc::new(app.clone());

            let success = Helper::exec_command(&command, &basic.path, move |msg| {
                PipelineRunnable::save_log(&*app_cloned, msg, &*server_id_cloned, &*id_cloned, order);
            });

            if !success {
                FileHandler::delete_file(&dockerfile_file_path_str)?; // 删除 Dockerfile 文件
                FileHandler::delete_file(&nginx_file_path_str)?; // 删除 nginx.conf 文件
                return Err(Error::convert_string(&format!("run docker command failed: {}", command)));
            }
        }

        info!("run docker commands success !");
        if docker_config.need_push == "Yes" {
            return Self::update_image(app, pipeline, &docker_config, &image, order).await;
        }

        return Ok(result.clone());
    }

    /// 获取 docker 配置
    fn exec_docker_config(pipeline: &Pipeline, stage_step: &PipelineRunnableStageStep, snapshot: &PipelineRuntimeSnapshot) -> DockerConfig {
        let mut components = stage_step.step.clone().components.clone();
        info!("replace variables");
        Self::replace_variables(pipeline, &mut components);

        let mut config = DockerConfig::default();
        for component in components.iter() {
            let prop = &component.prop;
            if prop == "docker.dockerfile" {
                config.dockerfile = component.value.clone();
            }

            if prop == "docker.address" {
                config.address = component.value.clone();
            }

            if prop == "docker.image" {
                config.image = component.value.clone();
            }

            if prop == "docker.namespace" {
                config.namespace = component.value.clone();
            }

            if prop == "docker.version" {
                config.version = component.value.clone();
            }

            if prop == "docker.user" {
                config.user = component.value.clone();
            }

            if prop == "docker.password" {
                config.password = component.value.clone();
            }

            if prop == "docker.platform" {
                config.platform = component.value.clone();
            }

            if prop == "docker.needPush" {
                config.need_push = component.value.clone();
            }

            if prop == "docker.nginx.path" {
                config.nginx_path = component.value.clone();
            }

            if prop == "docker.nginx.conf" {
                config.nginx_content = component.value.clone();
            }
        }

        // 如果 image 为空从 pipeline_runtime_variable 中查找
        if config.image.is_empty() {
            config.image = PipelineRunnableStage::get_value_from_variables(&snapshot.runnable_variables, "dockerImage");
        }

        if config.version.is_empty() {
            config.version = PipelineRunnableStage::get_value_from_variables(&snapshot.runnable_variables, "dockerVersion");
        }

        if config.version.is_empty() {
            config.version = Utils::get_date(Some("%Y%m%d-%H%M%S".to_string()));
        }

        return config;
    }

    //  拉取 nginx 镜像 docker pull xxx
    fn exec_docker_pull_nginx(docker_config: &DockerConfig) -> String {
        let mut first_line = String::new();
        let lines = docker_config.dockerfile.lines();
        for line in lines.into_iter() {
            if !first_line.is_empty() {
                break;
            }

            first_line = line.to_string();
        }

        if first_line.is_empty() {
            return String::new();
        }

        let first_line = first_line.trim();
        if !first_line.starts_with("FROM ") {
            error!("docker pull nginx error, not start with `FORM` ");
            return String::new();
        }

        let command = first_line.replace("FROM", "docker pull");
        info!("docker pull command: {}", command);
        return command;
    }

    /// 替换 Dockerfile中的变量
    fn replace_variables(pipeline: &Pipeline, docker_config: &mut Vec<PipelineStepComponent>) {
        if docker_config.is_empty() {
            return;
        }

        let runtime = &pipeline.clone().runtime.unwrap_or(PipelineRuntime::default());
        let snapshot = &runtime.snapshot;

        let re = Regex::new(r"\$\w+").unwrap();
        for config in docker_config.iter_mut() {
            let value = config.value.clone();
            let value = re
                .replace_all(&value, |caps: &regex::Captures| {
                    let caps = &caps[0];
                    let caps = caps.replace("$", "");
                    let variable_value: String = PipelineRunnableStage::get_value_from_variables(&snapshot.runnable_variables, &caps);
                    return variable_value;
                })
                .to_string();
            config.value = value;
        }
    }

    /// 连接服务器, 修改 image 地址
    async fn update_image(app: &AppHandle, pipeline: &Pipeline, docker_config: &DockerConfig, image: &str, order: u32) -> Result<PipelineRunnableResult, String> {
        PipelineRunnable::save_log(app, "update `image` in `kubectl` ...", &pipeline.server_id, &pipeline.id, order);

        let server_id = &pipeline.server_id;

        // 查找服务器信息
        let mut server = Server::default();
        server.id = server_id.to_string();
        let response = Server::get_by_id(&server).await?;
        if response.code != 200 {
            return Err(Error::convert_string(&response.error));
        }

        let server: Server = serde_json::from_value(response.body).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let serve = sftp::config::Server {
            host: server.ip.to_string(),
            port: server.port,
            username: server.account.to_string(),
            password: server.pwd.to_string(),
            timeout: Some(5),
        };

        let func = |_: &str| {};
        let log_func = Arc::new(Mutex::new(func));
        let session = SftpHandler::connect(&serve, log_func.clone())?;

        // 登录到 root
        let login_cmd = format!("echo {} | sudo -S -i", serve.password);

        // 获取当前 YAML 配置
        let yaml_cmd = format!("kubectl get deploy {} -n devops -o yaml", docker_config.image);
        let cmd = format!("{} bash -c '{}'", login_cmd, yaml_cmd);
        info!("get yaml config command: {}", cmd);

        let yaml_content = Self::exec_remote_command(&session, &cmd, "get kubectl yaml config error")?;

        /*
        let regex = Regex::new(r#"image:\s*([^\s]+)"#).unwrap();
        let modified_yaml = regex.replace_all(&yaml_content, format!("image: {}", image).as_str());
        */

        PipelineRunnable::save_log(app, &format!("kubectl yaml content:\n{}", yaml_content), &pipeline.server_id, &pipeline.id, order);
        if yaml_content.is_empty() {
            return Err(Error::convert_string("can not get kubectl yaml config !"));
        }

        let image_cmd = format!(
            r#"{{"spec": {{
                "template": {{
                    "spec": {{
                        "containers": [
                            {{
                                "name": "{}",
                                "image": "{}"
                            }}
                        ]
                    }}
                }}
            }}
        }}"#,
            docker_config.image, image
        )
        .replace("\n", "") // 去除所有换行符
        .replace("  ", ""); // 去除多余空格

        let cmd = format!("{} kubectl patch deployment {} -n devops --type=merge --patch '{}'", login_cmd, docker_config.image, image_cmd);
        PipelineRunnable::save_log(app, &format!("kubectl update image command:\n{}", cmd), &pipeline.server_id, &pipeline.id, order);

        let output = Self::exec_remote_command(&session, &cmd, "exec command `kubectl patch` error")?;
        PipelineRunnable::save_log(app, &format!("kubectl patch output info: {}", output), &pipeline.server_id, &pipeline.id, order);

        // 如果没有 change, 需要删除原来的 pod
        if output.contains("no change") {
            let success = Self::delete_pod_name(app, &session, pipeline, docker_config, order)?;
            if !success {
                return Err(Error::convert_string("delete pod error!"));
            }
        }

        PipelineRunnable::save_log(app, "update `image` in `kubectl` success ...", &pipeline.server_id, &pipeline.id, order);
        return Ok(PipelineRunnableResult {
            success: true,
            msg: "".to_string(),
            pipeline: Some(pipeline.clone()),
        });
    }

    fn delete_pod_name(app: &AppHandle, session: &Session, pipeline: &Pipeline, docker_config: &DockerConfig, order: u32) -> Result<bool, String> {
        // 1. 查找 pod 名字 kubectl get pod -n devops | grep xxx
        let cmd = format!("kubectl get pod -n devops | grep {}", docker_config.image);
        let output = Self::exec_remote_command(session, &cmd, "kubectl get pod name error")?;

        if output.is_empty() {
            PipelineRunnable::save_log(app, "no pod name output", &pipeline.server_id, &pipeline.id, order);
            return Ok(false)
        }

        let mut pod_name = String::new();
        if let Some(line) = output.lines().find(|line| line.starts_with(&docker_config.image)) {
            let name = line.split_whitespace().next().unwrap_or("");
            PipelineRunnable::save_log(app, &format!("pod name: {}", name), &pipeline.server_id, &pipeline.id, order);
            pod_name = name.to_string()
        } else {
            PipelineRunnable::save_log(app, "no pad name get !", &pipeline.server_id, &pipeline.id, order);
        }

        if pod_name.is_empty() {
            return Ok(false)
        }

        // 2. delete pod: kubectl delete pod -n devops ${podname}
        let cmd = format!("kubectl delete pod -n devops {}", pod_name);
        let output = Self::exec_remote_command(session, &cmd, "kubectl delete pod error")?;
        PipelineRunnable::save_log(app, &format!("kubectl delete pod output: {}", output), &pipeline.server_id, &pipeline.id, order);
        return Ok(true)
    }

    /// 执行远程命令
    fn exec_remote_command(session: &Session, cmd: &str, error_msg: &str) -> Result<String, String> {
        let mut channel = SftpHandler::create_channel(&session)?;
        channel.exec(&cmd).map_err(|err| {
            let msg = format!("{}: {:#?}", error_msg, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        let mut output = String::new();
        channel.read_to_string(&mut output).map_err(|err| {
            let msg = format!("{}: {:#?}", error_msg, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        SftpHandler::close_channel_in_err(&mut channel);

        return Ok(output)
    }
}
