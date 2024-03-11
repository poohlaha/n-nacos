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

use rayon::ThreadPoolBuilder;

use exports::server::{delete_server, get_server_detail, get_server_list, insert_server, update_server};

use crate::exports::monitor::{start_monitor, stop_monitor};
use crate::system::tray::Tray;
use exports::pipeline::{delete_pipeline, exec_steps, get_pipeline_detail, get_pipeline_list, insert_pipeline, pipeline_batch_run, pipeline_run, update_pipeline};

const PROJECT_NAME: &str = "n-nacos";

pub(crate) const MAX_THREAD_COUNT: u32 = 2;

// 日志目录: /Users/xxx/Library/Logs/n-nacos-reporter
// 程序配置目录: /Users/xxx/Library/Application Support/n-nacos

/// 初始化一些属性
fn init() {
    ThreadPoolBuilder::new().num_threads(MAX_THREAD_COUNT as usize).build_global().unwrap();
}
fn main() {
    // init
    init();

    // tauri
    tauri::Builder::default()
        // .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(move |app| {
            // 创建系统托盘
            Tray::builder(app);

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
            exec_steps,
            pipeline_batch_run,
            start_monitor,
            stop_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
