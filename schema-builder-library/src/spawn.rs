/// Platform-specific bounds required to spawn tasks.
///
/// On non-WASM targets, `DataService` implementations must be `Send + Sync + 'static`
/// so they can be moved into `tokio::spawn`-ed tasks. On WASM, JavaScript objects
/// are `!Send`, so only `'static` is required.
#[cfg(not(feature = "wasm"))]
pub(crate) trait DataServiceBounds: crate::DataService<Error: Send> + Send + Sync + 'static {}
#[cfg(not(feature = "wasm"))]
impl<T: crate::DataService + Send + Sync + 'static> DataServiceBounds for T
where
    T::Error: Send,
{}

#[cfg(feature = "wasm")]
pub(crate) trait DataServiceBounds: crate::DataService + 'static {}
#[cfg(feature = "wasm")]
impl<T: crate::DataService + 'static> DataServiceBounds for T {}

/// Runs a collection of `()` futures in parallel.
///
/// On non-WASM targets, each future is spawned as an independent tokio task on the
/// thread pool. On WASM, all futures are driven cooperatively within the current task
/// via `join_all`.
#[cfg(not(feature = "wasm"))]
pub(crate) async fn join_parallel<F>(futures: impl IntoIterator<Item = F>)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let handles: Vec<_> = futures.into_iter().map(tokio::spawn).collect();
    futures::future::join_all(handles).await;
}

#[cfg(feature = "wasm")]
pub(crate) async fn join_parallel<F>(futures: impl IntoIterator<Item = F>)
where
    F: std::future::Future<Output = ()>,
{
    futures::future::join_all(futures).await;
}
