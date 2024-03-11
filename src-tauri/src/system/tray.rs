//! 托盘

use tauri::tray::{ClickType, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager, Wry};

pub struct Tray;

const TRAY_ICON_ID: &str = "__TRAY__";

impl Tray {
    /// 创建系统托盘
    pub fn builder(app: &mut App) {
        let _ = TrayIconBuilder::with_id(TRAY_ICON_ID)
            .icon(app.default_window_icon().unwrap().clone())
            .icon_as_template(true)
            .on_tray_icon_event(Self::on_tray_icon_event)
            .build(app);
    }

    /// 托盘图标事件
    fn on_tray_icon_event(tray: &TrayIcon<Wry>, event: TrayIconEvent) {
        if event.click_type == ClickType::Left {
            let app = tray.app_handle();
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}
