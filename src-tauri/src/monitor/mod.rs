//! 通过 sysinfo 来获取系统信息

use log::info;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use sysinfo::{Pid, Signal, System};

/// 系统信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Os {
    os_type: String,        // 名称(平台类型)
    kernel_version: String, // 内核版本
    os_version: String,     // 操作系统版本
    host_name: String,      // 系统名称
    cpu_num: usize,         // CPU 总核数
    total_memory: u64,      // 总内存
    used_memory: u64,       // 忆使用内存
    total_swap: u64,        // 总交换分区
    used_swap: u64,         // 已使用的交换分区
}

/// 磁盘信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsDisk {
    type_: String, // 磁盘类型
    name: String,
    total_space: u64,     // 总数
    available_space: u64, // 已使用数
    mount_point: String,
    is_removable: bool, // 是否被移除
}

/// 磁盘使用情况(相对于进程)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total_written_bytes: u64, // 总写入数
    pub written_bytes: u64,       // 写入数
    pub total_read_bytes: u64,    // 总读取数
    pub read_bytes: u64,          // 读取数
}

pub struct Monitor {
    sys: System,
}

impl Monitor {
    pub fn new() -> Self {
        Self { sys: System::new_all() }
    }

    // 获取应用程序进程 ID 列表
    pub fn find_app_process_ids(&mut self, app_name: &str, app_path: Option<String>, prefix: Option<String>) -> Vec<u32> {
        self.sys.refresh_all();
        let mut processes = self.sys.processes();
        let process_list: Vec<(Pid, String, String)> = processes
            .iter()
            .filter_map(|(pid, process)| {
                let path = process.exe(); // &Path
                let name = process.name().to_string_lossy().to_string();
                if let Some(path) = path {
                    if let Some(path_str) = path.to_str() {
                        if let Some(prefix) = &prefix {
                            if path_str.starts_with(prefix) {
                                return Some((*pid, path_str.to_string(), name));
                            }
                        } else {
                            return Some((*pid, path_str.to_string(), name));
                        }
                    }
                }

                None
            })
            .collect();

        info!("processes: {:#?}", process_list);
        info!("app path: {:#?}", app_path);
        let processes: Vec<u32> = process_list
            .iter()
            .filter_map(|(pid, path, name)| {
                if let Some(app_path) = &app_path {
                    if app_path == path {
                        return Some(pid.as_u32());
                    } else {
                        // 向上查找
                        if Self::is_same_app(PathBuf::from(path).as_path(), PathBuf::from(app_path).as_path()) {
                            return Some(pid.as_u32());
                        }
                    }
                } else {
                    if name.as_str() != app_name {
                        return Some(pid.as_u32());
                    }
                }

                None
            })
            .collect();

        processes
    }

    // 杀死进程
    pub fn kill_process_list(&mut self, pid: &Vec<u32>) -> Vec<bool> {
        self.sys.refresh_all();

        let results: Vec<bool> = pid
            .iter()
            .map(|pid| {
                if let Some(process) = self.sys.process(Pid::from_u32(pid.clone())) {
                    let result = process.kill_with(Signal::Kill);
                    if let Some(result) = result {
                        return result;
                    }
                }

                false
            })
            .collect();

        results
    }

    fn kill_process(&mut self, pid: u32) -> bool {
        self.sys.refresh_all();

        if let Some(process) = self.sys.process(Pid::from_u32(pid as u32)) {
            let result = process.kill_with(Signal::Kill);
            if let Some(result) = result {
                return result;
            }
        }

        false
    }

    /// 判断实际进程路径是否属于目标 `.app` 应用
    fn is_same_app(process_path: &Path, app_path: &Path) -> bool {
        let mut current = process_path;

        while let Some(parent) = current.parent() {
            if parent.extension().map_or(false, |ext| ext == "app") {
                return parent == app_path;
            }
            current = parent;
        }

        false
    }
}
