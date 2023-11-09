//! 监控

/*!
 let mut mon = Monitor::new();
 let system_info = mon.get_system_info();
 println!("system info: {:#?}", system_info);

 let system_disk_info = mon.get_all_disk_info();
 println!("system disk info: {:#?}", system_disk_info);

 let system_network_info = mon.get_all_network_info();
 println!("system network info: {:#?}", system_network_info);

  let system_process_info = mon.get_all_process_info();
  println!("system process info: {:#?}", system_process_info);
*/
mod monitor;

pub use monitor::{Monitor, Os, OsDisk, OsNetwork, OsProcess, OutputKillProcess, OutputOsProcess};
