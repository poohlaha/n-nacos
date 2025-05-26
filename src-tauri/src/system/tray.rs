//! 托盘

use log::error;
use std::sync::Arc;
use tauri::menu::{IsMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, PhysicalPosition, Wry};

pub struct Tray;

const TRAY_ICON_ID: &str = "__TRAY__";

impl Tray {
    // 创建系统托盘
    pub fn builder(app: &AppHandle) {
        // let menus = Self::create_menus(app);
        let tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
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

    /*
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
     */

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
