use pyo3::prelude::*;
use tokio::runtime::Runtime;

/// Manages async runtime for sync/async context detection
pub struct RuntimeManager {
    runtime: Option<Runtime>,
}

impl RuntimeManager {
    /// Create a new RuntimeManager
    pub fn new() -> Self {
        RuntimeManager { runtime: None }
    }

    /// Get or create a tokio runtime with optimized settings
    pub fn get_or_create_runtime(&mut self) -> &Runtime {
        self.runtime.get_or_insert_with(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)                    // Optimize for typical workloads
                .thread_name("requestx-worker")       // Named threads for debugging
                .thread_stack_size(2 * 1024 * 1024)  // 2MB stack size
                .enable_all()                         // Enable all tokio features
                .build()
                .expect("Failed to create optimized tokio runtime")
        })
    }

    /// Check if we're in an async context
    pub fn is_async_context(py: Python) -> PyResult<bool> {
        // Check if we're in an asyncio event loop
        match pyo3_asyncio::tokio::get_current_loop(py) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}
