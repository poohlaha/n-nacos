//! 导出设置方法

use crate::prepare::{get_success_response, HttpResponse};
use crate::setting::Settings;
use crate::task::Task;

/// 保存
#[tauri::command]
pub async fn save_setting(settings: Settings) -> Result<HttpResponse, String> {
    Task::task_param(settings, |settings| Settings::save(&*settings)).await
}

/// 获取
#[tauri::command]
pub async fn get_setting() -> Result<HttpResponse, String> {
    Task::task(|| Settings::get()).await
}

/// 隐藏 DOCK 栏
#[tauri::command]
pub async fn hide_dock() -> Result<HttpResponse, String> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSApplication;
        unsafe {
            let ns_app = cocoa::appkit::NSApp();
            ns_app.setActivationPolicy_(cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyProhibited);
        }
    }

    Ok(get_success_response(None))
}

/// 显示 DOCK 栏
#[tauri::command]
pub async fn show_dock() -> Result<HttpResponse, String> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
        unsafe {
            let ns_app = NSApp();

            // 临时允许激活窗口
            ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);

            // 再隐藏 Dock 图标
            // ns_app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyProhibited);
        }
    }

    Ok(get_success_response(None))
}
