//! 托盘

use tauri::tray::TrayIconBuilder;
use tauri::App;

pub struct Tray;

const TRAY_ICON_ID: &str = "__TRAY__";

impl Tray {
    /// 创建系统托盘
    pub fn builder(app: &mut App) {
        let _ = TrayIconBuilder::with_id(TRAY_ICON_ID)
            .icon(app.default_window_icon().unwrap().clone())
            .icon_as_template(true)
            // .on_tray_icon_event(Self::on_tray_icon_event)
            .build(app);
    }

    /*
    /// 托盘图标事件, 引入 unstable features 后导致 不能用 ctrl + c/v
    fn on_tray_icon_event(tray: &TrayIcon<Wry>, event: TrayIconEvent) {
        if event.click_type == ClickType::Left {
            let app = tray.app_handle();
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
    */
}
