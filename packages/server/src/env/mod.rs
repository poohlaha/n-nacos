//! 环境配置

use crate::Args;
use clap::Parser;
use colored::*;
use ini::Ini;
use std::io;
use std::path::{Path, PathBuf};
use utils::file::FileHandler;
use utils::utils::UtilsHandler;

#[derive(Default, Debug)]
pub struct Configs {
    pub(crate) rust_log: String,
    pub(crate) http_port: u32,
    pub(crate) grpc_port: u32,
    pub(crate) http_workers: u32,
    pub(crate) database_dir: String,
    pub(crate) database_size: u64,
    pub(crate) http_address: String,
    pub(crate) grpc_address: String,
}

const ENV_VARIABLE: [&str; 6] = ["RUST_LOG", "NACOS_HTTP_PORT", "NACOS_GRPC_PORT", "NAOS_HTTP_WORKERS", "NACOS_DATABASE_DIR", "NACOS_DATABASE_SIZE"];

pub struct Env;

impl Env {
    /// 读取 .ini 文件
    fn read_config(file_path: &str) -> Option<Configs> {
        let path = Path::new(file_path);
        if !path.exists() {
            log::warn!("path: {} not exists!", file_path.magenta().bold());
            return None;
        }

        let contents = FileHandler::read_file(file_path);
        if contents.is_err() {
            log::error!("{:#?}", contents.err());
            return None;
        }

        let contents = contents.unwrap_or(String::new());
        if contents.is_empty() {
            let err_msg = format!("no contents in path: {}", file_path);
            log::error!("{}", err_msg.red().bold());
            return None;
        }

        let conf = match Ini::load_from_str(&contents) {
            Ok(conf) => Some(conf),
            Err(err) => {
                log::error!("analysis `env.ini` error: {:#?}", err);
                None
            }
        };

        if conf.is_none() {
            return None;
        }

        let conf = conf.unwrap();
        let rust_log = conf.get_from::<&str>(None, ENV_VARIABLE[0]).unwrap_or("info");
        let http_port = conf.get_from::<&str>(None, ENV_VARIABLE[1]).unwrap_or_default();
        let grpc_port = conf.get_from::<&str>(None, ENV_VARIABLE[2]).unwrap_or_default();
        let http_workers = conf.get_from::<&str>(None, ENV_VARIABLE[3]).unwrap_or_default();
        let database_dir = conf.get_from::<&str>(None, ENV_VARIABLE[4]).unwrap_or_default();
        let database_size = conf.get_from::<&str>(None, ENV_VARIABLE[5]).unwrap_or_default();

        return Some(Configs {
            rust_log: rust_log.to_string(),
            http_port: http_port.parse::<u32>().unwrap_or(0),
            grpc_port: grpc_port.parse::<u32>().unwrap_or(0),
            http_workers: http_workers.parse::<u32>().unwrap_or(0),
            database_dir: database_dir.to_string(),
            database_size: database_size.parse::<u64>().unwrap_or(0),
            http_address: String::new(),
            grpc_address: String::new(),
        });
    }

    /// 初始化, 读取配置, 设置日志级别等
    pub(crate) fn init() -> Option<Configs> {
        // 先设置为 info 级别, 为了打印设置环境变量日志
        std::env::set_var("RUST_LOG", &"info".to_owned());
        env_logger::builder().format_timestamp_micros().init();

        // 初始化配置文件
        let configs = Self::init_env_config();

        // 检查环境变量中的值
        let configs = Self::validate_env_items(configs);
        if configs.is_none() {
            return None;
        }

        return configs;
    }

    /// 初始化环境配置文件
    fn init_env_config() -> Option<Configs> {
        let args = Args::parse();
        let env_file_path = args.env_file;

        // 设置默认配置
        let path = FileHandler::find(Some(&std::env::current_dir().unwrap()), Path::new("env.ini"));
        let configs = Self::set_env_items(path);

        if env_file_path.is_empty() {
            log::warn!("{}", "env file path is empty, use default `.env` file !".yellow().bold());
            return configs;
        }

        // 判断路径是否存在
        if !Path::new(&env_file_path).exists() {
            log::warn!("env file path not exists, use default `.env` file !");
            return configs;
        }

        // 初始化配置文件, 用新的文件值覆盖默认值
        return Self::set_env_items(FileHandler::find(None, Path::new(&env_file_path)));
    }

    /// 获取根目录下的 `env.ini` 配置文件项列表
    fn set_env_items(path: Result<PathBuf, io::Error>) -> Option<Configs> {
        if path.is_err() {
            log::error!("{:#?}", path.err());
            return None;
        }

        let path = path.unwrap();
        let configs = Self::read_config(path.to_str().unwrap_or_default());
        if configs.is_none() {
            return None;
        }

        return Some(configs.unwrap_or_default());
    }

    /// 校验环境变量中的值
    fn validate_env_items(configs: Option<Configs>) -> Option<Configs> {
        if configs.is_none() {
            return None;
        }

        let mut configs = configs.unwrap_or_default();
        if configs.http_port == 0 {
            log::error!("{} is empty in env !", ENV_VARIABLE[0].red().bold());
            return None;
        }

        if configs.grpc_port == 0 {
            log::error!("{} is empty in env !", ENV_VARIABLE[1].red().bold());
            return None;
        }

        if configs.http_workers == 0 {
            configs.http_workers = UtilsHandler::get_cpu_count(Some(configs.http_workers));
        }

        if configs.database_dir.is_empty() {
            configs.database_dir = "nacos-db".to_string();
        }

        if configs.database_size == 0 {
            configs.database_size = 10 * 1024 * 1024;
        }

        configs.http_address = format!("0.0.0.0:{}", configs.http_port);
        configs.grpc_address = format!("0.0.0.0:{}", configs.grpc_port);

        // 设置日志级别
        std::env::set_var("RUST_LOG", &configs.rust_log);

        // 打印环境变量
        log::info!("{}: {}", ENV_VARIABLE[0].cyan().bold(), configs.rust_log.magenta().bold());
        log::info!("{}: {}", ENV_VARIABLE[1].cyan().bold(), configs.http_port.to_string().magenta().bold());
        log::info!("{}: {}", ENV_VARIABLE[2].cyan().bold(), configs.grpc_port.to_string().magenta().bold());
        log::info!("{}: {}", ENV_VARIABLE[3].cyan().bold(), configs.http_workers.to_string().magenta().bold());
        log::info!("{}: {}", ENV_VARIABLE[4].cyan().bold(), configs.database_dir.magenta().bold());
        log::info!("{}: {}", ENV_VARIABLE[5].cyan().bold(), configs.database_size.to_string().magenta().bold());
        log::info!("{}: {}", "http server address".cyan().bold(), configs.http_address.magenta().bold());
        log::info!("{}: {}", "grpc server address".cyan().bold(), configs.grpc_address.magenta().bold());
        return Some(configs);
    }
}
