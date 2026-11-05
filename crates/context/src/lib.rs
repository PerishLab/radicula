#![forbid(unsafe_code)]

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

struct Node {
    key: TypeId,
    value: Arc<dyn Any + Send + Sync>,
    parent: Option<Arc<Node>>,
}

#[derive(Clone, Default)]
pub struct Context {
    head: Option<Arc<Node>>,
}

thread_local! {
    static CURRENT: RefCell<Context> = RefCell::new(Context::default());
}

impl Context {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn with<T: Send + Sync + 'static>(&self, value: T) -> Self {
        Self {
            head: Some(Arc::new(Node {
                key: TypeId::of::<T>(),
                value: Arc::new(value),
                parent: self.head.clone(),
            })),
        }
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let mut held = &self.head;
        while let Some(node) = held {
            if node.key == TypeId::of::<T>() {
                return node.value.clone().downcast().ok();
            }
            held = &node.parent;
        }
        None
    }

    pub fn current() -> Self {
        CURRENT.with(|held| held.borrow().clone())
    }

    pub fn run<R>(&self, work: impl FnOnce() -> R) -> R {
        let _scope = Scope(CURRENT.with(|held| held.replace(self.clone())));
        work()
    }

    pub fn carry<F: Future>(&self, future: F) -> Carry<F> {
        Carry {
            context: self.clone(),
            future: Some(future),
        }
    }
}

struct Scope(Context);

impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|held| held.replace(std::mem::take(&mut self.0)));
    }
}

pin_project_lite::pin_project! {
    #[must_use = "a carried future must be polled to execute its body"]
    pub struct Carry<F> {
        context: Context,
        #[pin]
        future: Option<F>,
    }

    impl<F> PinnedDrop for Carry<F> {
        fn drop(this: Pin<&mut Self>) {
            let held = this.project();
            let mut future = held.future;
            held.context.run(|| future.set(None));
        }
    }
}

impl<F: Future> Future for Carry<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let held = self.project();
        let mut future = held.future;
        held.context.run(|| {
            let result = future
                .as_mut()
                .as_pin_mut()
                .expect("carried future polled after completion")
                .poll(task);
            if result.is_ready() {
                future.set(None);
            }
            result
        })
    }
}
