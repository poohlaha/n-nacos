use crate::actor::{BeanQueryFactory, BeanQueryFactoryResult, ContainerData, ContainerEvent, Factory, InjectBeanMap, Provider, ProviderBeanMap};
use crate::bean::BeanInstance;
use crate::factory::BeanFactory;
use actix::prelude::*;
use colored::*;
/**!
 Bean 工厂核心, 通过 `Actix` 的 `Actor` 发送和接收消息
*/
use std::sync::Arc;

#[derive(Default)]
pub struct BeanFactoryCore {
    provider_map: ProviderBeanMap, // provider 中的 `fn` 或 `value`
    inject_map: InjectBeanMap,     // 所有注入的 Bean
}

impl BeanFactoryCore {
    /// 初始化
    pub fn init(&mut self) {
        log::info!("{}", "Bean Factory init ...".cyan().bold());

        if self.inject_map.is_empty() {
            log::info!("{}", "No beans need to init !".magenta().red());
            log::info!("{}", "Bean Factory init successfully !".cyan().bold());
            return;
        }

        // 获取 bean 中 provider, 写入到 map
        self.inject_map.iter().for_each(|(name, bean)| match &bean.provider {
            Provider::Fn(f) => {
                if let Some(f) = f() {
                    self.provider_map.insert(bean.type_name.to_owned(), f);
                    log::info!("{}: {}", "Bean Factory init bean with fn".cyan().bold(), name.magenta().bold());
                }
            }
            Provider::Value(v) => {
                self.provider_map.insert(bean.type_name.to_owned(), v.clone());
                log::info!("{}: {}", "Bean Factory init bean with value".cyan().bold(), name.magenta().bold());
            }
        });

        log::info!("{}", "Bean Factory init successfully !".cyan().bold());
    }

    /// 注入 `Bean`
    fn inject(&mut self, ctx: &mut Context<Self>) -> ContainerData {
        let container_data = ContainerData(Arc::new(self.provider_map.clone()));
        let event = ContainerEvent::Inject {
            factory: BeanFactory::new_by_core(ctx.address()),
            data: container_data.clone(),
        };

        self.notify(event);
        // self.notify(ContainerEvent::Complete);
        log::info!("{}", "Bean Factory inject successfully !".cyan().bold());
        return container_data;
    }

    /// 发送通知
    fn notify(&mut self, event: ContainerEvent) {
        log::info!("{}", "Bean Factory send notify ...".cyan().bold());
        if self.inject_map.is_empty() {
            log::info!("{}", "No beans need to send notify !".magenta().red());
            log::info!("{}", "Bean Factory send notify successfully !".cyan().bold());
            return;
        }

        let mut count = 0; // 发送消息数量
        for (name, bean) in self.inject_map.iter() {
            let provider_bean = self.provider_map.get(name);
            if provider_bean.is_none() {
                continue;
            }

            let notify = bean.notify.clone();
            if notify.is_none() {
                continue;
            }

            let provider_bean = provider_bean.unwrap();
            let notify = notify.unwrap();

            // 发送消息
            notify(provider_bean.clone(), event.clone());

            log::info!("{}: {}", "Bean Factory trigger inject, bean".cyan().bold(), name.magenta().bold());
            count += 1;
        }

        log::info!("{}: {}", "Bean Factory trigger inject, bean count".cyan().bold(), count.to_string().magenta().bold());
        log::info!("{}", "Bean Factory send notify successfully !".cyan().bold());
    }
}

/// 实现 `Actor`
impl Actor for BeanFactoryCore {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::info!("{}", "Bean Factory Core started ...".cyan().bold());
    }
}

/// 实现监听 `Factory` 事件
impl Handler<Factory> for BeanFactoryCore {
    type Result = Option<ContainerData>;

    fn handle(&mut self, _: Factory, ctx: &mut Self::Context) -> Self::Result {
        self.init();
        let factory_data = self.inject(ctx);
        Some(factory_data)
    }
}

/// 实现监听 `BeanInstance` 事件
impl Handler<BeanInstance> for BeanFactoryCore {
    type Result = ();

    fn handle(&mut self, msg: BeanInstance, _: &mut Self::Context) -> Self::Result {
        // 插入到实例化列表
        self.inject_map.insert(msg.type_name.to_owned(), msg);
    }
}

/// 实现监听 `BeanQueryFactory` 事件
impl Handler<BeanQueryFactory> for BeanFactoryCore {
    type Result = Option<BeanQueryFactoryResult>;

    fn handle(&mut self, msg: BeanQueryFactory, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            BeanQueryFactory::Init => {
                // 初始化
                self.init();
                self.inject(ctx);
                Some(BeanQueryFactoryResult::None)
            }
            BeanQueryFactory::QueryName(name) => {
                let value = self.provider_map.get(&name).map(|e| e.clone());
                Some(BeanQueryFactoryResult::Bean(value.clone()))
            }
            BeanQueryFactory::QueryNames => {
                let names = self.inject_map.keys().into_iter().cloned().collect();
                Some(BeanQueryFactoryResult::Names(names))
            }
        }
    }
}
