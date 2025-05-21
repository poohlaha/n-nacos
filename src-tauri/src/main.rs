// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod article;
mod database;
mod error;
mod event;
mod exports;
mod helper;
mod logger;
mod look;
mod prepare;
mod server;
mod system;
mod task;

mod utils;
// mod db;

use lazy_static::lazy_static;
use rayon::ThreadPoolBuilder;

use crate::database::Database;
use crate::exports::monitor::{start_monitor, stop_monitor};
use crate::look::cache::CACHE_TTL_SECONDS;
use crate::look::home::Look;
use crate::server::pipeline::pool::Pool;
use crate::server::pipeline::props::PipelineStageTask;
use crate::system::tray::Tray;
use exports::article::{delete_article, get_archive_article_list, get_article_detail, get_article_list, get_article_tag_classify, get_article_tag_list, get_tag_article_list, save_or_update_article};
use exports::look::{get_desktop_list, get_document_list, get_download_list, get_pictures_list, get_recent_used};
use exports::pipeline::{clear_run_history, delete_pipeline, get_pipeline_detail, get_pipeline_list, get_runtime_history, insert_pipeline, pipeline_batch_run, pipeline_run, query_os_commands, update_pipeline};
use exports::server::{delete_server, get_server_detail, get_server_list, insert_server, update_server};
use log::info;
use sqlx::MySql;
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};
// use tauri_plugin_autostart::MacosLauncher;
// use tauri_plugin_autostart::ManagerExt;

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
    ThreadPoolBuilder::new().num_threads(MAX_THREAD_COUNT as usize).build_global().expect("Failed to build global thread pool");

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

// 启动定时器来读取目录(downloads)
fn start_cache_download_dir_timer() {
    tauri::async_runtime::spawn(async move {
        loop {
            info!("loop cache download dir ...");
            Look::refresh("Downloads").await;
            tokio::time::sleep(Duration::from_secs(CACHE_TTL_SECONDS as u64)).await;
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
    let mut builder = tauri::Builder::default()
        // .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, cwd| {
            let window = app.get_webview_window("main");
            if let Some(window) = window {
                window.show().unwrap();
                window.set_focus().unwrap();
            }
        }))
        // .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--flag1", "--flag2"])))
        .setup(move |app| {
            let app_handle = app.handle();

            // 创建系统托盘
            Tray::builder(&app_handle);

            /*
            // 开机启动
            // 获取自动启动管理器
            let autostart_manager = app.autolaunch();
            // 启用 autostart
            let _ = autostart_manager.enable();
            // 检查 enable 状态
            println!("registered for autostart? {}", autostart_manager.is_enabled().unwrap());
            // 禁用 autostart
            // let _ = autostart_manager.disable();
             */

            // 初始化
            tauri::async_runtime::spawn(async move {
                init().await;
            });

            start_task(&app_handle);
            start_cache_download_dir_timer();

            Ok(())
        })
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                info!("focused false...");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close(); // 阻止关闭
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide(); // 最小化到托盘
                }

                // 隐藏 Dock 图标
                // NSApplicationActivationPolicy::Prohibited: 不会显示在 Dock，无法成为活跃应用，无法接受键盘输入
                #[cfg(target_os = "macos")]
                {
                    use cocoa::appkit::NSApplication;
                    unsafe {
                        let ns_app = cocoa::appkit::NSApp();
                        ns_app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyProhibited);
                    }
                }
            }
        });

    let mut app = builder
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
            get_archive_article_list,
            get_recent_used,
            get_desktop_list,
            get_document_list,
            get_pictures_list,
            get_download_list
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(move |app, event| match &event {
        tauri::RunEvent::Reopen { has_visible_windows, .. } => {
            info!("reopen window");
            if !has_visible_windows {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                }
            }
        }
        _ => (),
    });
}
