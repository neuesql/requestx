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

    /// Get or create a tokio runtime
    pub fn get_or_create_runtime(&mut self) -> &Runtime {
        if self.runtime.is_none() {
            self.runtime = Some(Runtime::new().expect("Failed to create tokio runtime"));
        }
        self.runtime.as_ref().unwrap()
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
