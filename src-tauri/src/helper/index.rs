//! Helper handle

use crate::PROJECT_NAME;
use handlers::file::FileHandler;
use log::{error, info};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::{io, thread};

pub struct Helper;

impl Helper {
    /// 获取配置目录
    pub(crate) fn get_project_config_dir(names: Vec<String>) -> Result<Option<PathBuf>, String> {
        let mut path: Option<PathBuf> = dirs::data_dir();
        if path.is_none() {
            path = dirs::config_dir();
        }

        if let Some(data_dir) = path {
            info!("config root dir: {:#?}", data_dir);
            let mut project_config_dir = data_dir.join(PROJECT_NAME);
            if names.len() > 0 {
                names.iter().for_each(|name| {
                    project_config_dir = project_config_dir.join(name);
                })
            }

            info!("config dir: {:#?}", project_config_dir);
            // 创建目录
            let path = FileHandler::create_dirs(project_config_dir.to_str().unwrap())?;
            return Ok(Some(path));
        }

        error!("get config dir error !");
        return Ok(None);
    }

    /// 获取版本
    pub(crate) fn get_cmd_version(name: &str) -> String {
        let output = Command::new(&name).arg("--version").output();
        return match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    // 去除换行
                    let stdout = stdout.replace("\n", "").trim().to_string();
                    return stdout;
                }

                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    info!("get `{}` version error: {:#?}", name, stderr);
                    return String::new();
                }

                info!("get `{}` version failed, error status: {:#?}", name, output.status);
                String::new()
            }
            Err(err) => {
                info!("get `{}` version error: {:#?}", name, err);
                String::new()
            }
        };
    }

    /// 判断本机有没有安装某个命令
    pub(crate) fn check_installed_command(name: &str) -> bool {
        let mut command = "which";
        #[cfg(target_os = "windows")]
        {
            command = "where"
        }

        match Command::new(command).arg(name).output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// 执行命令
    pub(crate) fn exec_command<F>(command: &str, current_dir: &str, func: F) -> bool
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        if command.is_empty() {
            let msg = "command is empty !";
            func(&msg);
            return false;
        }

        let _command = command.replace("\n", " && ");

        // windows 通过 cmd /C 执行多条命令: cd c:\\usr\\local\\nginx\\sbin/ && nginx
        #[cfg(target_os = "windows")]
        {
            let msg = &format!("exec command: {}", _command);
            func(&msg);
            let child = Command::new("cmd").args(&["/C", &_command]).current_dir(current_dir).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
            return Self::get_exec_command_real_time_output_by_spawn(child, move |msg| {
                func(&msg);
            });
        }

        // linux|macos 通过 shell -c 执行多条命令: cd /usr/local/nginx/sbin/\n./nginx
        #[cfg(target_os = "macos")]
        {
            let msg = &format!("exec command: {}", _command);
            func(&msg);
            let child = Command::new("sh").arg("-c").arg(command).current_dir(current_dir).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
            return Self::get_exec_command_real_time_output_by_spawn(child, move |msg| {
                func(&msg);
            });
        }

        #[cfg(target_os = "linux")]
        {
            let msg = &format!("exec command: {}", _command);
            func(&msg);
            output = Command::new("sh").arg("-c").arg(command).current_dir(current_dir).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
            return Self::get_exec_command_real_time_output_by_spawn(child, move |msg| {
                func(&msg);
            });
        }
    }

    /// 实时输出日志
    pub(crate) fn run_command_output_real_time<F>(command: &str, args: &[&str], current_dir: &str, func: F) -> bool
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let msg = format!("current dir: {}", current_dir);
        func(&msg);

        // 判断是不是有命令
        let command_installed = Self::check_installed_command(&command);
        if !command_installed {
            let msg = format!("os not install command: {}", command);
            func(&msg);
            return false;
        }

        let child = Command::new(command).args(args.iter()).current_dir(current_dir).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        return Self::get_exec_command_real_time_output_by_spawn(child, move |msg| func(msg));
    }

    /// 通过 output 实时输出日志
    pub(crate) fn get_exec_command_real_time_output_by_spawn<F>(mut spawn: io::Result<Child>, func: F) -> bool
    where
        F: Fn(&str) + Send + 'static,
    {
        let child = match spawn.as_mut() {
            Ok(child) => Some(child),
            Err(err) => {
                let msg = format!("failed to get spawn, error: {:#?}", err);
                func(&msg);
                None
            }
        };

        if child.is_none() {
            return false;
        }

        let mut child = spawn.unwrap();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        if stdout.is_none() {
            let msg = "failed to open stdout !";
            func(&msg);
            return false;
        }

        if stderr.is_none() {
            let msg = "failed to open stderr !";
            func(&msg);
            return false;
        }

        let stdout = stdout.unwrap();
        let stderr = stderr.unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);
        // let has_error = Arc::new(Mutex::new(false));
        // let has_error_clone = has_error.clone();

        let func_cloned = Arc::new(Mutex::new(func));
        let func_clone = func_cloned.clone();
        let func_new_clone = func_cloned.clone();

        // 启动两个线程来实时输出 stdout 和 stderr
        let stdout_thread = thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    let func = func_cloned.lock().unwrap();
                    (*func)(&line);
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    // 标准错误输出通常用于报告警告、信息或调试信息，而不仅仅是错误
                    /*
                    let mut error = has_error_clone.lock().unwrap();
                    let is_error = error.clone();
                    if !is_error {
                        *error = true
                    }
                     */
                    let func = func_clone.lock().unwrap();
                    (*func)(&line);
                }
            }
        });

        // 等待子进程完成
        let status = match child.wait() {
            Ok(status) => Some(status),
            Err(err) => {
                let msg = format!("failed to wait spawn finished, error: {:#?}", err);
                let func = func_new_clone.lock().unwrap();
                (*func)(&msg);
                None
            }
        };

        if status.is_none() {
            return false;
        }

        let status = status.unwrap();
        match stdout_thread.join() {
            Ok(_) => {}
            Err(err) => {
                let msg = format!("failed to wait stdout thread finished, error: {:#?}", err);
                let func = func_new_clone.lock().unwrap();
                (*func)(&msg);
            }
        }

        match stderr_thread.join() {
            Ok(_) => {}
            Err(err) => {
                let msg = format!("failed to wait stderr thread finished, error: {:#?}", err);
                let func = func_new_clone.lock().unwrap();
                (*func)(&msg);
            }
        }

        let success = status.success();

        // let has_error = has_error.lock().unwrap();
        // let has_error = has_error.clone();
        return success;
    }
}
