use std::sync::Arc;

use lingxi_domain::AppSettings;
use lingxi_platform::AppPaths;

#[derive(Clone)]
pub struct AppRuntime {
    pub paths: Arc<AppPaths>,
    #[allow(dead_code)]
    pub settings: AppSettings,
    pub tokio_handle: tokio::runtime::Handle,
}

impl AppRuntime {
    pub fn spawn<F>(&self, fut: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tokio_handle.spawn(fut)
    }
}
