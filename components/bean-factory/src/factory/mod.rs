/**!
    Bean 工厂, 用于 Bean 的 `创建` 和 `管理`
*/
use crate::actor::{BeanQueryFactory, BeanQueryFactoryResult, ContainerData, Factory};
use crate::bean::BeanInstance;
use crate::core::BeanFactoryCore;
use actix::prelude::*;
use colored::Colorize;
use std::sync::Arc;

/// 初始化工厂
#[derive(Clone)]
pub struct BeanFactory {
    pub core: Addr<BeanFactoryCore>,
}

impl BeanFactory {
    /// 初始化
    pub fn new() -> Self {
        std::env::set_var("RUST_LOG", &"info".to_owned());
        env_logger::builder().format_timestamp_micros().init();

        Self { core: BeanFactoryCore::start_default() }
    }

    /// 通过 `core` 进行初始化
    pub fn new_by_core(core: Addr<BeanFactoryCore>) -> Self {
        Self { core }
    }

    /// 初始化工厂, 创建 `Bean` 实例, 返回数据
    pub async fn init(&self) -> ContainerData {
        match self.core.send(Factory).await {
            Ok(res) => res.unwrap(),
            Err(err) => {
                panic!("Bean Factory init error: {:#?}", err);
            }
        }
    }

    /// 初始化工厂, 创建 `Bean` 实例, 不返回数据
    pub fn register_without_result(&self) {
        self.core.do_send(Factory);
    }

    /// 注册 `Bean`, 不创建 `Bean` 实例, 使用 `do_send`
    pub fn register(&self, bean: BeanInstance) {
        self.core.do_send(bean);
    }

    /// 查询所有 `Bean` `Names`
    pub async fn query_bean_names(&self) -> Vec<String> {
        match self.core.send(BeanQueryFactory::QueryNames).await {
            Ok(res) => res.map_or(Vec::new(), |result| match result {
                BeanQueryFactoryResult::Names(r) => r,
                _ => Vec::new(),
            }),
            Err(err) => {
                log::warn!("{}: {:#?}", "Query Bean Names Error".cyan().bold(), err);
                Vec::new()
            }
        }
    }

    /// 根据 `Name` 查询 `Bean`
    pub async fn query_bean_by_name<T: 'static + Send + Sync>(&self, name: &str) -> Option<Arc<T>> {
        match self.core.send(BeanQueryFactory::QueryName(name.to_owned())).await {
            Ok(res) => res.map_or(None, |result| match result {
                BeanQueryFactoryResult::Bean(r) => match r {
                    None => None,
                    Some(r) => r.downcast::<T>().ok(),
                },
                _ => None,
            }),
            Err(err) => {
                log::warn!("{}: {:#?}", "Query Bean Name Error".cyan().bold(), err);
                None
            }
        }
    }
}

pub struct Bean;
