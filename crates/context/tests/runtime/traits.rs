use super::support::Probe;
use radicula_context::{Carry, Context};
use static_assertions::{assert_impl_all, assert_not_impl_any};
use std::future::{Pending, Ready};
use std::rc::Rc;

assert_impl_all!(Context: Clone, Default, Send, Sync, Unpin);
assert_impl_all!(Carry<Ready<u8>>: Send, Sync, Unpin);
assert_impl_all!(Carry<Probe>: Send);
assert_not_impl_any!(Carry<Probe>: Sync, Unpin);
assert_impl_all!(Carry<Pending<Rc<()>>>: Send, Sync);
assert_not_impl_any!(Carry<Ready<Rc<()>>>: Send, Sync);

#[test]
fn shared() {
    let held = Context::root();
    std::thread::scope(|scope| {
        scope.spawn(|| held.run(|| assert!(Context::current().get::<u8>().is_none())));
    });
}
