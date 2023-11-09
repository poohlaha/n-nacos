//! 监控
use chrono::{DateTime, Local, NaiveDateTime, Timelike};
use colored::*;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::process::{Command, Output};
use sysinfo::{DiskExt, DiskKind, NetworkExt, Pid, Process, ProcessExt, ProcessStatus, System, SystemExt};

pub struct Monitor {
    sys: System,
}

/// 系统信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Os {
    os_type: String,        // 名称(平台类型)
    kernel_version: String, // 内核版本
    os_version: String,     // 操作系统版本
    host_name: String,      // 系统名称
    pub cpu_num: usize,     // CPU 总核数
    total_memory: u64,      // 总内存
    used_memory: u64,       // 忆使用内存
    total_swap: u64,        // 总交换分区
    used_swap: u64,         // 已使用的交换分区
}

/// 磁盘信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsDisk {
    type_: String, // 磁盘类型
    pub name: String,
    pub total_space: u64,     // 总数
    pub available_space: u64, // 已使用数
    pub mount_point: String,
    pub is_removable: bool, // 是否被移除
}

/// 网络信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsNetwork {
    interface_name: String, // 网络名称
    received: u64,          // 拒绝
    transmitted: u64,       // 接收
    mac_address: String,    // mac 地址
}

/// 磁盘使用情况(相对于进程)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total_written_bytes: u64, // 总写入数
    pub written_bytes: u64,       // 写入数
    pub total_read_bytes: u64,    // 总读取数
    pub read_bytes: u64,          // 读取数
}

/// 进程
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsProcess {
    name: String,          // 进程名称
    pid: String,           // pid
    parent_pid: String,    // parent pid
    use_id: String,        // use id
    start_time: String,    // 启动时间
    disk_usage: DiskUsage, // 磁盘使用情况
    cpu_usage: f32,        // cpu 使用情况
    status: String,        // 运行状态
    cmd: String,           // 进程 cmd 程序
    exe: String,           // 进程启动方式(路径等)
    memory: u64,           // 进程占用内存
    virtual_memory: u64,   // 进程虚拟内存
    run_time: u64,         // 运行时长(秒)
}

/// 查找进程
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputOsProcess {
    name: String,
    processes: Vec<OsProcess>,
}

/// 结束进程输出结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputKillProcess {
    success: bool, // 是否成功
    error: String, // 错误信息
}

const LOGGER_PREFIX: &str = "[Nacos Monitor]: ";

impl Monitor {
    pub fn new() -> Self {
        Self { sys: System::new_all() }
    }

    /// 获取系统信息
    pub fn get_system_info(&mut self) -> Os {
        self.sys.refresh_all();

        return Os {
            os_type: self.sys.name().unwrap_or(String::new()),
            kernel_version: self.sys.kernel_version().unwrap_or(String::new()),
            os_version: self.sys.os_version().unwrap_or(String::new()),
            host_name: self.sys.host_name().unwrap_or(String::new()),
            cpu_num: self.sys.cpus().len(),
            total_memory: self.sys.total_memory(),
            used_memory: self.sys.used_memory(),
            total_swap: self.sys.total_swap(),
            used_swap: self.sys.used_swap(),
        };
    }

    /// 结束进程
    pub fn kill_process_list(&mut self, process_ids: Vec<String>) -> Vec<OutputKillProcess> {
        if process_ids.is_empty() {
            return Vec::new();
        }

        let get_output = |output: Result<Output, Error>, pid: &str| -> (bool, String) {
            match output {
                Ok(output) => {
                    if output.status.success() {
                        println!("{} Successfully killed process with PID {}", LOGGER_PREFIX.cyan().bold(), pid.to_string().magenta().bold());
                        return (true, String::new());
                    } else {
                        let error_msg = format!("{} Failed to kill process with PID {}: {:#?}", LOGGER_PREFIX.cyan().bold(), pid.to_string().magenta().bold(), output);
                        println!("{}", error_msg);
                        return (false, error_msg);
                    }
                }
                Err(err) => {
                    let error_msg = format!("{} Error killing process with PID {}: {:#?}", LOGGER_PREFIX.cyan().bold(), pid.to_string().magenta().bold(), err);
                    println!("{}", error_msg);
                    return (false, error_msg);
                }
            }
        };

        let mut results: Vec<OutputKillProcess> = Vec::new();

        #[cfg(target_os = "windows")]
        for pid in process_ids {
            let output = Command::new("taskkill").arg("/F").arg("/PID").arg(pid.to_string()).output();
            let (success, error) = get_output(output, &pid);
            if !success {
                results.push(OutputKillProcess { success, error })
            }
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        for pid in process_ids {
            let output = Command::new("kill").arg("-9").arg(pid.to_string()).output();
            let (success, error) = get_output(output, &pid);
            if !success {
                results.push(OutputKillProcess { success, error })
            }
        }

        return results;
    }

    /// 获取所有磁盘信息
    pub fn get_all_disk_info(&mut self) -> Vec<OsDisk> {
        self.sys.refresh_all();
        let mut disks: Vec<OsDisk> = Vec::new();

        for disk in self.sys.disks() {
            let type_ = match disk.kind() {
                DiskKind::HDD => "HDD",
                DiskKind::SSD => "SSD",
                DiskKind::Unknown(_) => "UNKNOWN",
            };
            disks.push(OsDisk {
                type_: String::from(type_),
                name: String::from(disk.name().to_str().unwrap_or("")),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                mount_point: String::from(disk.mount_point().to_owned().to_str().unwrap_or("")),
                is_removable: disk.is_removable(),
            });
        }

        return disks;
    }

    /// 获取所有网络信息
    pub fn get_all_network_info(&mut self) -> Vec<OsNetwork> {
        self.sys.refresh_all();
        let mut networks: Vec<OsNetwork> = Vec::new();

        for (interface_name, data) in self.sys.networks() {
            networks.push(OsNetwork {
                interface_name: String::from(interface_name),
                received: data.received(),
                transmitted: data.transmitted(),
                mac_address: data.mac_address().to_string(),
            })
        }

        return networks;
    }

    /// 获取所有进程
    pub fn get_all_process_info(&mut self) -> Vec<OsProcess> {
        self.sys.refresh_all();

        fn get_start_time(start_time: u64) -> String {
            let date_time = DateTime::<Local>::from_naive_utc_and_offset(NaiveDateTime::from_timestamp_opt(start_time as i64, 0).unwrap(), *Local::now().offset());
            let hour = date_time.hour();
            let minute = date_time.minute();
            let am_pm = if hour < 12 { "AM" } else { "PM" };
            let formatted_hour = if hour <= 12 { hour } else { hour - 12 };
            return format!("{:02}:{:02}{}", formatted_hour, minute, am_pm);
        }

        fn get_process(pid: &Pid, process: &Process) -> OsProcess {
            let parent_pid = match process.parent() {
                None => String::new(),
                Some(parent) => parent.to_string(),
            };

            let use_id = match process.user_id() {
                None => String::new(),
                Some(uid) => uid.to_string(),
            };

            let sys_disk_usage = process.disk_usage();
            let disk_usage = DiskUsage {
                total_written_bytes: sys_disk_usage.total_written_bytes,
                written_bytes: sys_disk_usage.written_bytes,
                total_read_bytes: sys_disk_usage.total_read_bytes,
                read_bytes: sys_disk_usage.read_bytes,
            };

            let status = match process.status() {
                ProcessStatus::Idle => String::from("Idle"),
                ProcessStatus::Run => String::from("Run"),
                ProcessStatus::Sleep => String::from("Sleep"),
                ProcessStatus::Stop => String::from("Stop"),
                ProcessStatus::Zombie => String::from("Zombie"),
                ProcessStatus::Tracing => String::from("Tracing"),
                ProcessStatus::Dead => String::from("Dead"),
                ProcessStatus::Wakekill => String::from("Wakekill"),
                ProcessStatus::Waking => String::from("Waking"),
                ProcessStatus::Parked => String::from("Parked"),
                ProcessStatus::LockBlocked => String::from("LockBlocked"),
                ProcessStatus::UninterruptibleDiskSleep => String::from("UninterruptibleDiskSleep"),
                ProcessStatus::Unknown(_) => String::from("Unknown"),
            };

            let cmd: String = process.cmd().iter().map(|arg| arg.to_string()).collect();

            return OsProcess {
                name: String::from(process.name()),
                pid: pid.to_string(),
                parent_pid,
                use_id,
                start_time: get_start_time(process.start_time()),
                disk_usage,
                cpu_usage: process.cpu_usage(),
                status,
                cmd,
                exe: String::from(process.exe().to_str().unwrap_or("")),
                memory: process.memory(),
                virtual_memory: process.virtual_memory(),
                run_time: process.run_time(),
            };
        }

        let mut processes: Vec<OsProcess> = Vec::new();
        for (pid, process) in self.sys.processes() {
            processes.push(get_process(pid, process));
        }

        return processes;
    }

    /// 根据 `名称` 或者 `pid` 获取进程列表
    pub fn get_process_list(&mut self, process_list: Vec<OsProcess>, process_names: &Vec<String>) -> Vec<OutputOsProcess> {
        if process_names.is_empty() {
            return Vec::new();
        }

        let mut all_processes: Vec<OsProcess> = process_list.clone();
        if all_processes.is_empty() {
            all_processes = self.get_all_process_info();
        }

        let mut processes: Vec<OutputOsProcess> = Vec::new();
        for process_name in process_names.iter() {
            let mut names: Vec<OsProcess> = Vec::new();
            for process in all_processes.iter() {
                let name = &process.name;

                if process_name.to_lowercase() == name.to_lowercase() {
                    names.push(process.clone());
                    continue;
                }
            }

            processes.push(OutputOsProcess {
                name: process_name.to_string(),
                processes: names,
            })
        }

        return processes;
    }

    /// 计算 cpu 使用率
    pub fn get_cpu_usage_rate(&mut self) -> f32 {
        let mut total_usage: f32 = 0.0;
        let processes = self.get_all_process_info();
        for process in processes.iter() {
            println!("{} process name: {}, cpu usage: {}", LOGGER_PREFIX.cyan().bold(), process.name, process.cpu_usage);
            total_usage += process.cpu_usage;
        }

        // 计算平均使用率
        return total_usage / processes.len() as f32;
    }
}
