//! 读取服务器远程监控信息

use crate::error::Error;
use crate::event::EventEmitter;
use crate::prepare::{get_error_response, get_success_response, HttpResponse};
use crate::server::index::Server;
use crypto_hash::{hex_digest, Algorithm};
use handlers::file::FileHandler;
use lazy_static::lazy_static;
use log::{error, info};
use serde_json::Value;
use sftp::runnable::SftpRunnableHandler;
use sftp::sftp::SftpHandler;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const TOOLS_DIR: &str = "resources";
const REMOTE_TOOLS_DIR: &str = "__MONITOR__";
const TOOL_NAME: &str = "n-nacos-tools";

pub struct Monitor;

// 定义全局 session
lazy_static! {
    static ref SESSION: Arc<Mutex<Option<Arc<Mutex<ssh2::Session>>>>> = Arc::new(Mutex::new(None));
}

impl Monitor {
    pub fn exec(app: &AppHandle, server: &Server) -> Result<HttpResponse, String> {
        if server.id.is_empty() {
            return Err(Error::convert_string("server id is empty !"));
        }

        // 根据 id 查找服务器
        /*
        let response = Server::get_by_id(server)?;
        if response.code != 200 {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        // 判断程序是否启动
        let mut is_listening = false;
        if let Some(_) = *SESSION.lock().unwrap() {
            is_listening = true
        }

        // 正在监听，直接返回
        if is_listening {
            return Ok(get_success_response(Some(Value::Bool(true))));
        }

        let serve = convert_res::<Server>(response);
        if serve.is_none() {
            return Err(Error::convert_string(&format!("find server by id: {} failed !", &server.id)));
        }

        let serve = serve.unwrap();
         */
        let serve = Server {
            id: "".to_string(),
            ip: "".to_string(),
            port: 0,
            account: "".to_string(),
            pwd: "".to_string(),
            name: "".to_string(),
            description: "".to_string(),
            create_time: None,
            update_time: None,
        };

        let path = Self::get_local_monitor_file(app);
        if path.is_none() {
            return Err(Error::convert_string("can not get local monitor file !"));
        }

        let path = path.unwrap();
        let file_path = path.as_path().to_string_lossy().to_string();

        // 获取 hash
        let hash = Self::get_local_monitor_file_hash(&file_path)?;
        Self::judge_monitor_file(app, &serve, &hash, &file_path)?;

        Ok(get_success_response(Some(serde_json::Value::Bool(true))))
    }

    /// 获取本地监听程序
    fn get_local_monitor_file(app: &AppHandle) -> Option<PathBuf> {
        let file_path = Path::new(TOOLS_DIR).join(TOOL_NAME);
        let local_file = app.path().resolve(&file_path, tauri::path::BaseDirectory::Resource);
        info!("local monitor file: {:#?}", local_file);
        let file = match local_file {
            Ok(file) => file,
            Err(err) => {
                error!("found `{}` error: {:#?}", TOOLS_DIR, err);
                return None;
            }
        };

        if !file.exists() {
            info!("local monitor file path `{:#?}` not exists !", file);
            return None;
        }

        return Some(file);
    }

    fn get_local_monitor_file_hash(file_path: &str) -> Result<String, String> {
        let buffer = FileHandler::read_file_buffer(&file_path)?;
        return Ok(hex_digest(Algorithm::SHA256, &buffer));
    }

    /// 判断远程监控文件和本地文件是否一致, 如果不一致则重新上传工具, 并启动工具
    pub fn judge_monitor_file(app: &AppHandle, server: &Server, hash: &str, file_path: &str) -> Result<(), String> {
        let serve = sftp::config::Server {
            host: server.ip.to_string(),
            port: server.port,
            username: server.account.to_string(),
            password: server.pwd.to_string(),
            timeout: Some(5),
        };

        let copy = sftp::config::ValidateCopy {
            hash: hash.to_string(),
            file_path: file_path.to_string(),
            dest_dir: REMOTE_TOOLS_DIR.to_string(),
        };

        let file_name = Path::new(file_path).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        let dest_file_path = SftpRunnableHandler::exec(serve.clone(), copy, |_| {})?;

        // 使用异步来启动程序
        let props_cloned = Arc::new(serve.clone());
        let dest_file_path_cloned = Arc::new(dest_file_path.clone());
        let file_name_cloned = Arc::new(file_name.clone());
        let app_cloned = Arc::new(app.clone());
        tauri::async_runtime::spawn(async move {
            match Self::exec_program(&*app_cloned, &*props_cloned, &*dest_file_path_cloned, &*file_name_cloned) {
                Ok(_) => {}
                Err(_) => {}
            }
        });

        Ok(())
    }
    fn exec_program(app: &AppHandle, server: &sftp::config::Server, dest_file_path: &str, file_name: &str) -> Result<(), String> {
        if server.is_empty() {
            let msg = "exec runnable program failed, one of `host`、`port`、`username` and `password` server items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let func = |_: &str| {};
        let log_func = Arc::new(Mutex::new(func));
        // 连接服务器
        let session = SftpHandler::connect(&server, log_func.clone())?;
        let sftp = session.sftp().map_err(|err| {
            let msg = format!("exec runnable program error: {:#?}", err);
            Error::convert_string(&msg);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        // 判断程序是否在运行
        let mut pid = String::new();
        if sftp.stat(Path::new(&dest_file_path)).is_ok() {
            pid = SftpRunnableHandler::judge_program_running(&session, file_name, log_func.clone())?;
        }

        // 如果在运行，则直接结束
        if !pid.is_empty() {
            SftpRunnableHandler::kill_pid(&session, &pid)?;
        }

        info!("start program {} ...", dest_file_path);
        let mut channel = SftpHandler::create_channel(&session)?;

        // let cmd = format!("nohup {} &", file_path); // 不受通道关闭的影响
        // let cmd = format!("{} & disown", file_path);

        // channel.flush().unwrap();

        // 通道一直会开着的, 因为要监听程序的输出, 当通道关闭后, 程序也结束
        channel.exec(dest_file_path).map_err(|err| {
            let msg = format!("start program `{}` error: {:#?}", dest_file_path, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        *SESSION.lock().unwrap() = Some(Arc::new(Mutex::new(session)));
        let mut stdout = channel.stream(0); // 0表示标准输出
        let mut buffer = [0; 4096];
        loop {
            let bytes = match stdout.read(&mut buffer) {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            };

            if bytes.is_none() {
                break;
            }

            let bytes = bytes.unwrap();
            if bytes == 0 {
                break;
            }

            // 处理输出，可以根据需要自定义逻辑
            let output = String::from_utf8_lossy(&buffer[..bytes]);
            info!("{}", output);

            // 发送数据
            Self::send_response(app, &output.to_string());
            thread::sleep(Duration::from_secs(1));
        }

        Ok(())
    }

    /// 停止监听
    pub fn stop() -> Result<HttpResponse, String> {
        info!("stop monitor ...");
        // 直接取出，避免 session n问题
        let session = SESSION.lock().unwrap().take();

        if let Some(session) = session {
            let session = session.lock().unwrap();
            let result = SftpHandler::close_session(session.clone());
            return match result {
                Ok(_) => {
                    *SESSION.lock().unwrap() = None;
                    info!("stop monitor success !");
                    Ok(get_success_response(None))
                }
                Err(err) => {
                    error!("stop monitor error: {}", &err);
                    Ok(get_error_response(&err))
                }
            };
        }

        info!("stop monitor success !");
        return Ok(get_success_response(Some(Value::Bool(true))));
    }

    /// 发送数据
    fn send_response(app: &AppHandle, content: &str) {
        if content.is_empty() {
            return;
        }

        let content = content.replace("system info: ", "");
        if content.is_empty() {
            return;
        }

        let value: Option<HttpResponse> = serde_json::from_str(&content).ok();
        if let Some(value) = value {
            EventEmitter::log_monitor_res(app, Some(value))
        }
    }
}
