//! `Bean` 装配工厂
pub mod factory;

mod core;

pub mod bean;

mod actor;

/// 使用 `inventory` 的 `submit`, 用于宏
pub use inventory::submit;

