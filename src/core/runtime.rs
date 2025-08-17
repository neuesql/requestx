use pyo3::prelude::*;
use pyo3_asyncio::tokio::{future_into_py, get_current_loop};
use std::future::Future;
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Handle, Runtime};

/// Global shared runtime for optimal performance and resource management
static GLOBAL_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

/// Get the global tokio runtime instance
fn get_global_runtime() -> &'static Arc<Runtime> {
    GLOBAL_RUNTIME.get_or_init(|| {
        let worker_threads = std::thread::available_parallelism()
            .map(|n| (n.get() * 2).min(16).max(4))
            .unwrap_or(8);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .max_blocking_threads(512)
            .thread_name("requestx-worker")
            .thread_stack_size(1024 * 1024)
            .enable_all()
            .build()
            .expect("Failed to create optimized global tokio runtime");

        Arc::new(runtime)
    })
}

/// Enhanced runtime manager for handling both sync and async execution contexts
pub struct RuntimeManager {
    custom_runtime: Option<Arc<Runtime>>,
}

impl RuntimeManager {
    /// Create a new RuntimeManager using the global runtime
    pub fn new() -> Self {
        RuntimeManager {
            custom_runtime: None,
        }
    }

    /// Create a RuntimeManager with a custom runtime
    #[allow(dead_code)]
    pub fn with_runtime(runtime: Runtime) -> Self {
        RuntimeManager {
            custom_runtime: Some(Arc::new(runtime)),
        }
    }

    /// Get the runtime to use (custom or global)
    pub fn get_runtime(&self) -> &Arc<Runtime> {
        self.custom_runtime
            .as_ref()
            .unwrap_or_else(|| get_global_runtime())
    }

    /// Enhanced async context detection using pyo3-asyncio
    pub fn is_async_context(py: Python) -> PyResult<bool> {
        // Method 1: Try to get the current asyncio event loop using pyo3-asyncio
        match get_current_loop(py) {
            Ok(_) => return Ok(true),
            Err(_) => {}
        }

        // Method 2: Fallback to checking asyncio directly
        let asyncio = py
            .import("asyncio")
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Failed to import asyncio"))?;

        match asyncio.call_method0("get_running_loop") {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Detect the current event loop and return its handle if available
    #[allow(dead_code)]
    pub fn get_event_loop_handle(py: Python) -> PyResult<Option<PyObject>> {
        let asyncio = py.import("asyncio")?;
        match asyncio.call_method0("get_running_loop") {
            Ok(loop_obj) => Ok(Some(loop_obj.into())),
            Err(_) => Ok(None),
        }
    }

    /// Execute a future in the appropriate context (sync or async)
    pub fn execute_future<F, T>(&self, py: Python, future: F) -> PyResult<PyObject>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: IntoPy<PyObject>,
    {
        if Self::is_async_context(py)? {
            // We're in an async context - return a coroutine
            Ok(future_into_py(py, future)?.into())
        } else {
            // We're in a sync context - block on the future
            let runtime = self.get_runtime();
            let result = runtime.block_on(future)?;
            Ok(result.into_py(py))
        }
    }

    /// Create a coroutine from a future for async contexts
    #[allow(dead_code)]
    pub fn create_coroutine<F, T>(py: Python, future: F) -> PyResult<PyObject>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: IntoPy<PyObject>,
    {
        Ok(future_into_py(py, future)?.into())
    }

    /// Block on a future in sync context
    #[allow(dead_code)]
    pub fn block_on_future<F, T>(&self, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
    {
        let runtime = self.get_runtime();
        runtime.block_on(future)
    }

    /// Check if we're currently inside a tokio runtime
    #[allow(dead_code)]
    pub fn is_in_tokio_runtime() -> bool {
        Handle::try_current().is_ok()
    }

    /// Get the current tokio runtime handle if available
    #[allow(dead_code)]
    pub fn get_tokio_handle() -> Option<Handle> {
        Handle::try_current().ok()
    }

    /// Spawn a task on the tokio runtime
    #[allow(dead_code)]
    pub fn spawn_task<F, T>(&self, future: F) -> tokio::task::JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let runtime = self.get_runtime();
        runtime.spawn(future)
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global runtime manager instance
static GLOBAL_RUNTIME_MANAGER: OnceLock<RuntimeManager> = OnceLock::new();

/// Get the global runtime manager instance
pub fn get_global_runtime_manager() -> &'static RuntimeManager {
    GLOBAL_RUNTIME_MANAGER.get_or_init(RuntimeManager::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::Python;

    #[test]
    fn test_runtime_manager_creation() {
        let manager = RuntimeManager::new();
        assert!(manager.custom_runtime.is_none());
    }

    #[test]
    fn test_runtime_manager_with_custom_runtime() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let manager = RuntimeManager::with_runtime(runtime);
        assert!(manager.custom_runtime.is_some());
    }

    #[test]
    fn test_global_runtime_manager() {
        let manager1 = get_global_runtime_manager();
        let manager2 = get_global_runtime_manager();

        // Should be the same instance
        assert!(std::ptr::eq(manager1, manager2));
    }

    #[test]
    fn test_tokio_runtime_detection() {
        // Outside of tokio runtime
        assert!(!RuntimeManager::is_in_tokio_runtime());

        // Inside tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(RuntimeManager::is_in_tokio_runtime());
        });
    }

    #[test]
    fn test_async_context_detection_no_asyncio() {
        Python::with_gil(|py| {
            // Without asyncio running, should return false
            let result = RuntimeManager::is_async_context(py);
            assert!(result.is_ok());
            assert!(!result.unwrap());
        });
    }

    #[tokio::test]
    async fn test_spawn_task() {
        let manager = RuntimeManager::new();

        let handle = manager.spawn_task(async { 42 });

        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }
}
