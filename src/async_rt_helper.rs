#[cfg(all(feature = "rt-tokio", feature = "rt-async-std"))]
compile_error!("You cannot enable both `rt-tokio` and `rt-async-std` features at the same time.");

#[cfg(all(not(feature = "rt-tokio"), not(feature = "rt-async-std")))]
compile_error!("You must enable one of the `rt-tokio` and `rt-async-std` features.");

#[cfg(feature = "rt-tokio")]
use rt_tokio as rt;

#[cfg(feature = "rt-async-std")]
use rt_async_std as rt;

pub(crate) use rt::{spawn, yield_now};

#[cfg(feature = "rt-tokio")]
mod rt_tokio {
    use std::future::Future;
    use tokio::task::JoinHandle;

    pub(crate) fn spawn<T>(future: T) -> JoinHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(future)
    }

    pub(crate) async fn yield_now() {
        tokio::task::yield_now().await;
    }
}

#[cfg(feature = "rt-async-std")]
mod rt_async_std {
    use async_std::task::JoinHandle;
    use std::future::Future;

    pub(crate) fn spawn<F, T>(future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        async_std::task::spawn(future)
    }

    pub(crate) async fn yield_now() {
        async_std::task::yield_now().await;
    }
}
