//! 公共方法类

use monitor::Monitor;

pub struct UtilsHandler;

impl UtilsHandler {
    /// 获取 `cpu` 的核心数, 如果 `count` 没有值, 则默认为: `cpu 核心数 - 1`
    pub fn get_cpu_count(count: Option<u32>) -> u32 {
        let get_cpu_num = || -> u32 {
            let thread_num: u32;
            let mut monitor = Monitor::new();
            let cpu_num = monitor.get_system_info().cpu_num as u32;
            if cpu_num > 1 {
                let num = cpu_num - 1;
                thread_num = if num > 1 { num } else { 1 };
            } else {
                thread_num = 1
            }

            return thread_num;
        };

        let cpu_num = get_cpu_num();
        if count.is_none() {
            return cpu_num;
        }

        let thread_num = count.unwrap_or(0);
        if thread_num < 1 {
            return cpu_num;
        }

        if thread_num > cpu_num {
            return cpu_num;
        }

        return thread_num;
    }
}
