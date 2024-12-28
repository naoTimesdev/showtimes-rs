use std::{marker::PhantomData, sync::Arc};

use futures_util::FutureExt;
use tokio::{sync::Notify, task::JoinSet};

pub(crate) trait BackgroundTask {
    fn run(&self) -> impl std::future::Future<Output = ()> + std::marker::Send;
}

pub(crate) struct BackgroundTaskHandle<T: BackgroundTask + Send + 'static> {
    handler: tokio::task::AbortHandle,
    task: PhantomData<T>,
}

impl<T: BackgroundTask + Send + 'static> BackgroundTaskHandle<T> {
    pub(crate) fn new(task: T, shutdown_signal: Arc<Notify>, tasks: &mut JoinSet<()>) -> Self {
        let handler = tasks.spawn(async move {
            tokio::select! {
                _ = shutdown_signal.notified() => {
                    tracing::info!("Shutting down background task");
                }
                _ = task.run().fuse() => {
                    tracing::info!("Background task finished");
                }
            }
        });

        Self {
            handler,
            task: PhantomData,
        }
    }
}

impl<T: BackgroundTask + Send + 'static> Drop for BackgroundTaskHandle<T> {
    fn drop(&mut self) {
        self.handler.abort();
    }
}

pub(crate) fn spawn_with<T: BackgroundTask + Send + 'static>(
    task: T,
    shutdown_signal: Arc<Notify>,
    handler: &mut JoinSet<()>,
) -> BackgroundTaskHandle<T> {
    BackgroundTaskHandle::new(task, shutdown_signal, handler)
}

// Now to the actual handler
pub struct RSSTasks {
    state: crate::state::SharedShowtimesState,
}

impl RSSTasks {
    pub fn new(state: crate::state::SharedShowtimesState) -> Self {
        Self { state }
    }
}

impl BackgroundTask for RSSTasks {
    async fn run(&self) {
        loop {
            tracing::info!("Updating RSS feeds");
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            // TODO: Acutal implementation.
        }
    }
}
