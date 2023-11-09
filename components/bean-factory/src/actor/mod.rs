//! 定义 `Actix` 中的 `Actor`

use crate::bean::BeanInstance;
use crate::factory::BeanFactory;
use actix::prelude::*;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// 类型, 静态的, 可以被线程安全的传递和共享
pub type DynAny = dyn Any + 'static + Send + Sync;

/// 所有注入的 Bean Map
pub type InjectBeanMap = HashMap<String, BeanInstance>;
pub type ProviderBeanMap = HashMap<String, Arc<DynAny>>;

/// 定义工厂消息体
#[derive(Message)]
#[rtype(result = "Option<ContainerData>")]
pub struct Factory;

/// 查询
#[derive(Message)]
#[rtype(result = "Option<BeanQueryFactoryResult>")]
pub enum BeanQueryFactory {
    Init,
    QueryName(String),
    QueryNames,
}

/// 获取查询结果
pub enum BeanQueryFactoryResult {
    None,
    Names(Vec<String>),
    Bean(Option<Arc<DynAny>>),
}

/// `Actor` 返回的工厂数据
#[derive(Debug, Clone)]
pub struct ContainerData(pub Arc<ProviderBeanMap>);

/// 定义 `Provider`
#[derive(Clone)]
pub enum Provider {
    Fn(Arc<dyn Fn() -> Option<Arc<DynAny>> + Send + Sync>),
    Value(Arc<DynAny>),
}

/// 定义容器事件
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum ContainerEvent {
    Inject { factory: BeanFactory, data: ContainerData },
    Complete,
}
