/**!
   测试用例
*/
use bean_factory::bean::BeanInstance;
use bean_factory::factory::BeanFactory;
use std::sync::Arc;

#[actix::test]
async fn test() {
    #[derive(Debug)]
    struct TestStruct {
        name: String,
        description: String,
    }

    struct TestStruct2 {
        name: String,
        url: String,
    }

    struct TestStruct3 {
        name: String,
        version: String,
    }

    let factory = BeanFactory::new();
    let test_value = Arc::new(TestStruct {
        name: "poohlaha".to_string(),
        description: "He is a handsome boy !".to_string(),
    });

    let test_value2 = Arc::new(TestStruct2 {
        name: "poohlaha".to_string(),
        url: "https://github.com/poohlaha".to_string(),
    });

    let test_value3 = Arc::new(TestStruct3 {
        name: "poohlaha".to_string(),
        version: "1.0.0".to_string(),
    });

    factory.register(BeanInstance::init_with_value(test_value));
    factory.register(BeanInstance::init_with_value(test_value2));
    factory.register(BeanInstance::init_with_value(test_value3));

    factory.init().await;

    // 查询 names
    let bean_names = factory.query_bean_names().await;
    log::info!("find beans names: {:#?}", bean_names);

    // 根据 name 查询 bean
    let name = bean_names.get(0).unwrap();
    log::info!("bean name: {}", name);
    let bean = factory.query_bean_by_name::<TestStruct>(name).await;
    log::info!("find bean name: {:#?}", bean);
}
