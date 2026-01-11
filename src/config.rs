use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Configuration for HTTP client settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    pub pool_idle_timeout_secs: u64,
    pub pool_max_idle_per_host: usize,
    pub http2_only: bool,
    pub http2_keep_alive_interval_secs: u64,
    pub http2_keep_alive_timeout_secs: u64,
    pub http2_initial_stream_window_size: u32,
    pub http2_initial_connection_window_size: u32,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            pool_idle_timeout_secs: 90,
            pool_max_idle_per_host: 512,
            http2_only: false,
            http2_keep_alive_interval_secs: 30,
            http2_keep_alive_timeout_secs: 10,
            http2_initial_stream_window_size: 65536,
            http2_initial_connection_window_size: 1048576,
        }
    }
}

impl HttpClientConfig {
    pub fn pool_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.pool_idle_timeout_secs)
    }

    pub fn http2_keep_alive_interval(&self) -> Duration {
        Duration::from_secs(self.http2_keep_alive_interval_secs)
    }

    pub fn http2_keep_alive_timeout(&self) -> Duration {
        Duration::from_secs(self.http2_keep_alive_timeout_secs)
    }
}

/// Configuration for Tokio runtime settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub worker_threads: usize,
    pub max_blocking_threads: usize,
    pub thread_name: String,
    pub thread_stack_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0, // 0 means auto-detect
            max_blocking_threads: 512,
            thread_name: "requestx-worker".to_string(),
            thread_stack_size: 512 * 1024,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestxConfig {
    pub client: HttpClientConfig,
    pub runtime: RuntimeConfig,
}

impl Default for RequestxConfig {
    fn default() -> Self {
        Self {
            client: HttpClientConfig::default(),
            runtime: RuntimeConfig::default(),
        }
    }
}

impl RequestxConfig {
    /// Load configuration from TOML file, falling back to defaults if file doesn't exist or has errors
    pub fn load() -> Self {
        Self::load_from_path("config.toml")
    }

    /// Load configuration from a specific path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();

        // Check if config file exists
        if !path.exists() {
            eprintln!(
                "Info: Config file '{}' not found. Using default configuration.",
                path.display()
            );
            return Self::default();
        }

        // Try to read and parse the file
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read config file '{}': {}. Using defaults.",
                    path.display(),
                    e
                );
                return Self::default();
            }
        };

        match toml::from_str::<RequestxConfig>(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse config file '{}': {}. Using defaults.",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }
}

/// Global configuration instance
static CONFIG: std::sync::OnceLock<RequestxConfig> = std::sync::OnceLock::new();

/// Get the global configuration instance
#[inline]
pub fn get_config() -> &'static RequestxConfig {
    CONFIG.get_or_init(RequestxConfig::load)
}

/// Get HTTP client configuration
#[inline]
pub fn get_http_client_config() -> &'static HttpClientConfig {
    &get_config().client
}

/// Get runtime configuration
#[inline]
pub fn get_runtime_config() -> &'static RuntimeConfig {
    &get_config().runtime
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RequestxConfig::default();
        assert_eq!(config.client.pool_idle_timeout_secs, 90);
        assert_eq!(config.runtime.max_blocking_threads, 512);
        assert_eq!(config.client.pool_max_idle_per_host, 512);
    }

    #[test]
    fn test_duration_conversions() {
        let http_config = HttpClientConfig::default();
        assert_eq!(http_config.pool_idle_timeout(), Duration::from_secs(90));
        assert_eq!(
            http_config.http2_keep_alive_interval(),
            Duration::from_secs(30)
        );
    }
}
