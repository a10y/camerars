use std::future::Future;

use lazy_static::lazy_static;
use tokio::runtime::Runtime;

lazy_static! {
    static ref RUNTIME: Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
}

/// Perform an async operation in a blocking fashion on the current thread.
/// Use this method to call async-only code from a sync context.
/// If you actually don't want to halt the current thread, you can instead.
pub(crate) fn do_sync<T, F: Future<Output = T>>(fut: F) -> T {
    RUNTIME.block_on(async move { fut.await })
}
