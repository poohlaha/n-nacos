// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod article;
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

// mod db;

use lazy_static::lazy_static;
use rayon::ThreadPoolBuilder;

use crate::database::Database;
use crate::exports::monitor::{start_monitor, stop_monitor};
use crate::server::pipeline::pool::Pool;
use crate::server::pipeline::props::PipelineStageTask;
use crate::system::tray::Tray;
use exports::article::{delete_article, get_article_detail, get_article_list, get_article_tag_classify, get_article_tag_list, save_or_update_article, get_tag_article_list, get_archive_article_list};
use exports::pipeline::{clear_run_history, delete_pipeline, get_pipeline_detail, get_pipeline_list, get_runtime_history, insert_pipeline, pipeline_batch_run, pipeline_run, query_os_commands, update_pipeline};
use exports::server::{delete_server, get_server_detail, get_server_list, insert_server, update_server};
use log::info;
use sqlx::MySql;
use std::env;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

const PROJECT_NAME: &str = "n-nacos";

pub(crate) const MAX_THREAD_COUNT: u32 = 4;

pub(crate) const MAX_DATABASE_COUNT: u32 = 5;
pub(crate) const LOOP_SEC: u64 = 10;

const DATABASE_URL: &str = "mysql://root:123456@localhost/nacos";

// 定义全局 线程池
lazy_static! {
    static ref POOLS: Arc<Mutex<Vec<PipelineStageTask>>> = Arc::new(Mutex::new(Vec::new()));
}

// 定义全局 数据库连接池
lazy_static! {
    static ref DATABASE_POOLS: Arc<Mutex<Option<sqlx::Pool<MySql>>>> = Arc::new(Mutex::new(None));
}

/// 初始化一些属性
async fn init() {
    // 设置并行任务最大数
    ThreadPoolBuilder::new().num_threads(MAX_THREAD_COUNT as usize).build_global().unwrap();

    // 从数据库读取任务
    Pool::get_pools().await;
}

// 启动线程来执行线程池中任务
fn start_task(app: &AppHandle) {
    let app_cloned = Arc::new(app.clone());
    tauri::async_runtime::spawn(async move {
        loop {
            info!("loop pipeline pools ...");
            Pool::start(&*app_cloned).await;
        }
    });
}

// 日志目录: /Users/xxx/Library/Logs/n-nacos
// 程序配置目录: /Users/xxx/Library/Application Support/n-nacos
#[tokio::main]
async fn main() {
    // 创建数据库连接池
    Database::create_db().await.unwrap();

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
            tauri::async_runtime::spawn(async move {
                init().await;
            });

            start_task(&app_handle);

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
            get_runtime_history,
            query_os_commands,
            clear_run_history,
            pipeline_batch_run,
            start_monitor,
            stop_monitor,
            get_article_list,
            save_or_update_article,
            get_article_tag_list,
            get_article_detail,
            delete_article,
            get_article_tag_classify,
            get_tag_article_list,
            get_archive_article_list
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
