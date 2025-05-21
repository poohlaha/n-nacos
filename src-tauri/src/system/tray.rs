//! 托盘

use log::{error, info};
use tauri::menu::{IsMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

pub struct Tray;

const TRAY_ICON_ID: &str = "__TRAY__";

impl Tray {
    // 创建系统托盘
    pub fn builder(app: &AppHandle) {
        let menus = Self::create_menus(app);
        let mut tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
            .icon(app.default_window_icon().unwrap().clone())
            .icon_as_template(true)
           // .on_tray_icon_event(Self::on_tray_icon_event)
            ;

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

        tray.build(app).unwrap();
    }

    fn create_menu(app: &AppHandle, id: &str, text: &str) -> Option<MenuItem<Wry>> {
        let quit_menu = MenuItem::with_id(app, id, text, true, None::<&str>);
        match quit_menu {
            Ok(quit_menu) => Some(quit_menu),
            Err(err) => {
                error!("create quite menu error: {:#?}", err);
                None
            }
        }
    }

    // 创建菜单
    fn create_menus(app: &AppHandle) -> Option<Menu<Wry>> {
        let quit_menu = Self::create_menu(app, "quit", "退出程序");
        let show_menu = Self::create_menu(app, "show", "显示主界面");

        let mut boxed_items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
        if let Some(quit_menu) = quit_menu {
            boxed_items.push(Box::new(quit_menu));
        }

        if let Some(show_menu) = show_menu {
            boxed_items.push(Box::new(show_menu));
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
