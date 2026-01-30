/*!
  聊天机器人
*/

use crate::prepare::HttpResponse;
use crate::utils::cache::CacheHelper;
use serde::{Deserialize, Serialize};

const ROBOT_FILE: &str = "robot.json";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Robot {
    name: String,
    pub url: String,
}

impl Robot {
    // 保存
    pub fn save(robot: &Robot) -> Result<HttpResponse, String> {
        CacheHelper::save::<Robot>(robot, ROBOT_FILE)
    }

    pub fn get() -> Result<HttpResponse, String> {
        CacheHelper::get::<Robot>(ROBOT_FILE)
    }
}
