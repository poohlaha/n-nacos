//! Git代码拉取

use crate::helper::git::GitHandler;
use crate::helper::index::Helper;
use handlers::file::FileHandler;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct GitConfig {
    pub(crate) url: String,    // Git 地址
    pub(crate) branch: String, // Git 分支
    pub(crate) dir: String,    // 存放地址
}

pub struct GitHelper;

impl GitHelper {
    /// 拉取代码
    pub(crate) fn pull<F>(config: &GitConfig, func: F) -> Result<bool, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        if config.url.is_empty() {
            let error_msg = "Git pull failed, `url` is empty!";
            func(error_msg);
            return Err(crate::error::Error::convert_string(error_msg));
        }

        if config.branch.is_empty() {
            let error_msg = "Git pull failed, `branch` is empty!";
            func(error_msg);
            return Err(crate::error::Error::convert_string(error_msg));
        }

        if config.dir.is_empty() {
            let error_msg = "Git pull failed, `dir` is empty!";
            func(error_msg);
            return Err(crate::error::Error::convert_string(error_msg));
        }

        // 判断目录是否存在
        let path = PathBuf::from(&config.dir);
        if !path.exists() {
            let error_msg = format!("Git pull failed, path: {} is not exists!", &config.dir);
            func(&error_msg);
            return Err(crate::error::Error::convert_string(&error_msg));
        }

        let msg = format!("Git pull params:\n {:#?}", &config);
        func(&msg);

        // 判断是否存在目录, 如果目录存在, 则直接删除
        let project_name = GitHandler::get_project_name_by_git(&config.url);
        let mut project_path = PathBuf::from(&config.dir);
        project_path.push(&project_name);

        // 目录存在, 则删除
        if project_path.exists() {
            let msg = format!("workspace exists project: {}, will be deleted !", &project_name);
            func(&msg);
            FileHandler::delete_dirs(vec![project_path.as_path().to_string_lossy().to_string()])?;
        }

        // 开始拉取代码
        let msg = format!("Starting pull {} code, branch {} ...", &project_name, &config.branch);
        func(&msg);

        let start_time = Instant::now();
        let func_cloned = Arc::new(Mutex::new(func));
        let func_clone = func_cloned.clone();

        let success = Helper::run_command_output_real_time("git", &["clone", "-b", &config.branch, &config.url], &config.dir, move |msg| {
            let func = func_cloned.lock().unwrap();
            (*func)(&msg);
        });

        if !success {
            let msg = format!("pull {} error !", &project_name);
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        } else {
            let msg = format!("pull {} success !", &project_name);
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        let elapsed_time = format!("{:.2?}", start_time.elapsed());
        let msg = format!("Finished pull {} after {}", &project_name, elapsed_time);
        let func = func_clone.lock().unwrap();
        (*func)(&msg);

        Ok(success)
    }
}
