//! Bean 实例

use std::any::Any;
use crate::actor::{ContainerEvent, DynAny, Provider};
use actix::dev::ToEnvelope;
use actix::prelude::*;
use std::sync::Arc;

/// 定义 `Bean` 实例
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct BeanInstance {
    pub type_name: String,
    pub provider: Provider,
    pub notify: Option<Arc<dyn Fn(Arc<DynAny>, ContainerEvent) -> () + Send + Sync>>,
}

impl BeanInstance {

    /// 默认初始化
    pub fn init<T: Default + Any + 'static + Send + Sync>() -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            provider: Provider::Fn(Arc::new(move || {
                Some(T::default()).map(|e| Arc::new(e) as Arc<DynAny>)
            })),
            notify: None,
        }
    }

    /// 根据 `Value` 初始化 `Bean`
    pub fn init_with_value<T: 'static + Send + Sync>(value: Arc<T>) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            provider: Provider::Value(value),
            notify: None,
        }
    }

    /// 根据 `Actor` 的 `Address` 初始化 `Bean`
    pub fn init_with_address<T>(address: Addr<T>) -> Self
    where
        T: Actor<Context = Context<T>> + Handler<ContainerEvent>,
        <T as Actor>::Context: ToEnvelope<T, ContainerEvent>,
    {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            provider: Provider::Value(Arc::new(address)),
            notify: Some(Arc::new(|addr, event| {
                addr.downcast::<Addr<T>>().ok().map(|e| e.do_send(event));
            })),
        }
    }
}
