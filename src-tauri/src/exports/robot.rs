use crate::prepare::HttpResponse;
use crate::robot::Robot;
use crate::task::Task;

// 保存
#[tauri::command]
pub async fn save_robot_config(robot: Robot) -> Result<HttpResponse, String> {
    Task::task_param(robot, |settings| Robot::save(&*settings)).await
}

/// 获取
#[tauri::command]
pub async fn get_robot_config() -> Result<HttpResponse, String> {
    Task::task(|| Robot::get()).await
}
