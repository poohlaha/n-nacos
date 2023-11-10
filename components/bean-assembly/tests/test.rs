/**!
  单元测试
*/

use actix::prelude::*;
use bean_assembly::Component;

#[derive(Message)]
#[rtype(result = "usize")]
struct Ping(usize);

#[derive(Default, Component)]
#[name("test")]
struct TestComponent {
    count: usize
}

impl Actor for TestComponent {
    type Context = Context<Self>;
}

impl Handler<Ping> for TestComponent {
    type Result = usize;

    fn handle(&mut self, msg: Ping, _: &mut Self::Context) -> Self::Result {
        self.count += msg.0;
        self.count
    }
}

#[test]
fn test_component() {


}