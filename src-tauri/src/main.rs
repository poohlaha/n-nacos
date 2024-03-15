// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod error;
mod event;
mod exports;
mod helper;
mod logger;
mod prepare;
mod server;
mod system;
mod task;

use lazy_static::lazy_static;
use rayon::ThreadPoolBuilder;

use crate::exports::monitor::{start_monitor, stop_monitor};
use crate::server::pipeline::props::PipelineStageTask;
use crate::system::tray::Tray;
use exports::pipeline::{delete_pipeline, get_pipeline_detail, get_pipeline_list, insert_pipeline, pipeline_batch_run, pipeline_run, query_os_commands, update_pipeline};
use exports::server::{delete_server, get_server_detail, get_server_list, insert_server, update_server};
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::AppHandle;
use crate::server::pipeline::pool::Pool;

const PROJECT_NAME: &str = "n-nacos";

pub(crate) const MAX_THREAD_COUNT: u32 = 2;
pub(crate) const LOOP_SEC: u64 = 50;

// 定义全局 线程池
lazy_static! {
    static ref POOLS: Arc<Mutex<Vec<PipelineStageTask>>> = Arc::new(Mutex::new(Vec::new()));
}

/// 初始化一些属性
fn init(app: &AppHandle) {
    // 设置并行任务最大数
    ThreadPoolBuilder::new().num_threads(MAX_THREAD_COUNT as usize).build_global().unwrap();

    // 从数据库读取任务
    Pool::get_pools();

    // 启动线程来执行线程池中任务
    let app_cloned = Arc::new(app.clone());
    thread::spawn(move || loop {
        info!("loop pipeline pools ...");
        Pool::start(&*app_cloned);
    });
}

// 日志目录: /Users/xxx/Library/Logs/n-nacos
// 程序配置目录: /Users/xxx/Library/Application Support/n-nacos
fn main() {
    // tauri
    tauri::Builder::default()
        // .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(move |app| {
            // 创建系统托盘
            Tray::builder(app);

            let app_handle = app.handle();

            // 初始化
            init(app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_server_list,
            insert_server,
            update_server,
            delete_server,
            get_server_detail,
            get_pipeline_list,
            insert_pipeline,
            update_pipeline,
            delete_pipeline,
            get_pipeline_detail,
            pipeline_run,
            query_os_commands,
            pipeline_batch_run,
            start_monitor,
            stop_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
