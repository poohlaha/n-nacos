//! 托盘

use crate::error::Error;
use log::error;
use plist::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{IsMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, PhysicalPosition, Wry};

pub struct Tray;

const TRAY_ICON_ID: &str = "__TRAY__";

struct ApplicationApp {
    icon: Option<String>, // 应用程序图标
    path: PathBuf,        // 应用程序位置
    name: String,         // 应用程序名称
}

impl Tray {
    // 创建系统托盘
    pub fn builder(app: &AppHandle) {
        let menus = Self::create_menus(app);
        let mut tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
            .icon(app.default_window_icon().unwrap().clone())
            .icon_as_template(true)
           // .on_tray_icon_event(Self::on_tray_icon_event)
            ;

        /*
        if let Some(menus) = menus {
            tray = tray.menu(&menus).show_menu_on_left_click(true).on_menu_event(|app, event| match event.id.as_ref() {
                "quit" => {
                    info!("quit menu item was clicked");
                    app.exit(0);
                }
                "show" => {
                    info!("show menu item was clicked");
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        // let _ = win.set_focus();

                        #[cfg(target_os = "macos")]
                        {
                            use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
                            unsafe {
                                let ns_app = NSApp();

                                // 临时允许激活窗口
                                ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);

                                // 显示并保持窗口前置
                                let _ = win.set_focus(); // 这个 set_focus 是 tauri 的 API

                                // 再隐藏 Dock 图标
                                // ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyProhibited);
                            }
                        }
                    }
                }
                _ => {
                    info!("menu item {:?} not handled", event.id);
                }
            });
        }
         */

        let app_clone = Arc::new(app.clone());
        tray.on_tray_icon_event(move |_, event| {
            let app_cloned = Arc::clone(&app_clone);
            // tauri_plugin_positioner::on_tray_event(app.app_handle(), &event);
            match event {
                TrayIconEvent::Click { id: _, rect: _, button, position, .. } => match button {
                    MouseButton::Left {} => {
                        log::info!("left clicked");
                        Self::send_tray_menu_message(&*app_cloned, position)
                    }
                    MouseButton::Right {} => {}
                    _ => {}
                },
                TrayIconEvent::DoubleClick { .. } => {}
                TrayIconEvent::Enter { .. } => {}
                TrayIconEvent::Move { .. } => {}
                TrayIconEvent::Leave { .. } => {}
                _ => {}
            }
        })
        .build(app)
        .unwrap();
    }

    fn send_tray_menu_message(app: &AppHandle, position: PhysicalPosition<f64>) {
        app.emit("tray_contextmenu", position).unwrap();
    }

    fn create_menu(app: &AppHandle, id: &str, text: &str) -> Option<MenuItem<Wry>> {
        let menu = MenuItem::with_id(app, id, text, true, None::<&str>);
        match menu {
            Ok(menu) => Some(menu),
            Err(err) => {
                error!("create `{}` menu error: {:#?}", text, err);
                None
            }
        }
    }

    fn create_menu_with_sub_menu(app: &AppHandle, id: &str, text: &str, submenu: Vec<&dyn IsMenuItem<Wry>>) -> Option<Submenu<Wry>> {
        let menu = Submenu::with_id_and_items(app, id, text, true, &submenu);
        match menu {
            Ok(menu) => Some(menu),
            Err(err) => {
                error!("create `{}` menu with submenu error: {:#?}", text, err);
                None
            }
        }
    }

    // 创建菜单
    fn create_menus(app: &AppHandle) -> Option<Menu<Wry>> {
        let quit_menu = Self::create_menu(app, "quit", "退出程序");
        let show_menu = Self::create_menu(app, "show", "显示主界面");
        let app_menu = Self::create_app_menus(app);

        let mut boxed_items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
        if let Some(quit_menu) = quit_menu {
            boxed_items.push(Box::new(quit_menu));
        }

        if let Some(show_menu) = show_menu {
            boxed_items.push(Box::new(show_menu));
        }

        if let Some(app_menu) = app_menu {
            boxed_items.push(Box::new(app_menu));
        }

        let menus: Vec<&dyn IsMenuItem<Wry>> = boxed_items.iter().map(|i| i.as_ref()).collect();
        let menus = Menu::with_items(app, &menus);
        match menus {
            Ok(menus) => Some(menus),
            Err(err) => {
                error!("create menus error: {:#?}", err);
                None
            }
        }
    }

    // 获取本机上的所有应用程序
    fn get_applications() -> Result<Vec<ApplicationApp>, String> {
        let applications_dir = PathBuf::from("/Applications");
        let entries = fs::read_dir(applications_dir).map_err(|err| Error::Error(err.to_string()).to_string())?.filter_map(Result::ok).collect::<Vec<_>>();

        let apps: Vec<ApplicationApp> = entries
            .iter()
            .filter_map(|entry| {
                let path = entry.path();
                // let file_path = path.to_string_lossy().to_string();
                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        let icon = Self::get_app_icon_path(&path);
                        let mut application = ApplicationApp {
                            icon: None,
                            path: path.clone(),
                            name: name.to_string(),
                        };

                        match icon {
                            Ok(icon) => {
                                application.icon = Some(icon);
                            }
                            Err(_) => {}
                        };

                        return Some(application);
                    }
                }

                None
            })
            .collect();

        Ok(apps)
    }

    // 获取 app 应用图标, 读取 `Info.plist` 中的字段 `CFBundleIconFile` 的值, 然后在 Resources 中查找
    fn get_app_icon_path(app_path: &PathBuf) -> Result<String, String> {
        let plist_path = app_path.join("Contents").join("Info.plist");
        let plist_data = fs::read(plist_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let plist: Value = plist::from_bytes(&plist_data).map_err(|err| Error::Error(err.to_string()).to_string())?;

        let dir = plist.as_dictionary();
        if let Some(dir) = dir {
            let value = dir.get("CFBundleIconFile");
            if let Some(value) = value {
                let icon_file = value.as_string();
                if let Some(icon_file) = icon_file {
                    // 添加 `.icns` 后缀（Info.plist 里通常不带）
                    let mut icon_name = icon_file.to_string();
                    if !icon_name.ends_with(".icns") {
                        icon_name.push_str(".icns");
                    }

                    let icon_path = app_path.join("Contents").join("Resources").join(icon_name);
                    if icon_path.exists() {
                        return Ok(icon_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(String::new())
    }

    fn create_app_submenus(app: &AppHandle, apps: Vec<ApplicationApp>) -> Option<Submenu<Wry>> {
        if apps.is_empty() {
            return None;
        }

        let menus: Vec<Box<dyn IsMenuItem<Wry>>> = apps
            .into_iter()
            .filter_map(|application_app: ApplicationApp| {
                let app_menu = Self::create_menu(app, &application_app.name, &application_app.name);
                if let Some(menu) = app_menu {
                    return Some(Box::new(menu) as Box<dyn IsMenuItem<Wry>>);
                }
                None
            })
            .collect();

        let menus: Vec<&dyn IsMenuItem<Wry>> = menus.iter().map(|i| i.as_ref()).collect();
        Self::create_menu_with_sub_menu(app, "apps", "打开应用程序", menus)
    }

    // 创建 apps 菜单
    fn create_app_menus(app: &AppHandle) -> Option<Submenu<Wry>> {
        let apps = Self::get_applications();
        match apps {
            Ok(apps) => Self::create_app_submenus(app, apps),
            Err(err) => {
                error!("read application app error: {:#?}", err);
                None
            }
        }
    }

    // 托盘图标点击事件
    /*
    fn on_tray_icon_event(tray: &TrayIcon<Wry>, event: TrayIconEvent) {
        info!("on_tray_icon_event event: {:#?}", event);
        match event {
            TrayIconEvent::Click { .. } => {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {
                info!("unhandled event {event:?}");
            }
        }
    }
     */
}
