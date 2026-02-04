//! Timeout, Limits, and Proxy configuration

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::collections::HashMap;
use std::time::Duration;

use crate::url::URL;

/// Timeout configuration for HTTP requests
#[pyclass(name = "Timeout")]
#[derive(Clone, Debug)]
pub struct Timeout {
    #[pyo3(get)]
    pub connect: Option<f64>,
    #[pyo3(get)]
    pub read: Option<f64>,
    #[pyo3(get)]
    pub write: Option<f64>,
    #[pyo3(get)]
    pub pool: Option<f64>,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            connect: Some(5.0),
            read: Some(5.0),
            write: Some(5.0),
            pool: Some(5.0),
        }
    }
}

impl Timeout {
    /// Create a new Timeout with the given values
    pub fn new(
        timeout: Option<f64>,
        connect: Option<f64>,
        read: Option<f64>,
        write: Option<f64>,
        pool: Option<f64>,
    ) -> Self {
        if let Some(t) = timeout {
            Self {
                connect: connect.or(Some(t)),
                read: read.or(Some(t)),
                write: write.or(Some(t)),
                pool: pool.or(Some(t)),
            }
        } else {
            Self {
                connect,
                read,
                write,
                pool,
            }
        }
    }

    pub fn to_duration(&self) -> Option<Duration> {
        // Use the minimum of all timeouts as the overall timeout
        let timeouts = [self.connect, self.read, self.write];
        let min_timeout = timeouts
            .iter()
            .filter_map(|&t| t)
            .min_by(|a, b| a.partial_cmp(b).unwrap());
        min_timeout.map(Duration::from_secs_f64)
    }

    pub fn connect_duration(&self) -> Option<Duration> {
        self.connect.map(Duration::from_secs_f64)
    }

    pub fn read_duration(&self) -> Option<Duration> {
        self.read.map(Duration::from_secs_f64)
    }

    pub fn write_duration(&self) -> Option<Duration> {
        self.write.map(Duration::from_secs_f64)
    }

    pub fn pool_duration(&self) -> Option<Duration> {
        self.pool.map(Duration::from_secs_f64)
    }

    /// Determine which timeout type triggered (when only one is set and active)
    /// Returns: "connect", "write", "read", "pool", or None if multiple or none set
    pub fn timeout_context(&self) -> Option<&'static str> {
        let set_count = [self.connect, self.write, self.read, self.pool]
            .iter()
            .filter(|t| t.is_some())
            .count();

        // Only return specific context if exactly one timeout is set
        if set_count == 1 {
            if self.connect.is_some() {
                return Some("connect");
            }
            if self.write.is_some() {
                return Some("write");
            }
            if self.read.is_some() {
                return Some("read");
            }
            if self.pool.is_some() {
                return Some("pool");
            }
        }
        None
    }
}

#[pymethods]
impl Timeout {
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    fn py_new(
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        // Extract keyword arguments
        let (timeout_kwarg, connect, read, write, pool) = if let Some(kw) = kwargs {
            let timeout_kw = kw.get_item("timeout")?;
            let connect: Option<f64> = kw.get_item("connect")?.and_then(|v| v.extract().ok());
            let read: Option<f64> = kw.get_item("read")?.and_then(|v| v.extract().ok());
            let write: Option<f64> = kw.get_item("write")?.and_then(|v| v.extract().ok());
            let pool: Option<f64> = kw.get_item("pool")?.and_then(|v| v.extract().ok());
            (timeout_kw, connect, read, write, pool)
        } else {
            (None, None, None, None, None)
        };

        // Determine the timeout value from either positional or keyword argument
        // has_timeout_arg indicates whether timeout was explicitly provided (even if None)
        let (timeout_value, has_timeout_arg): (Option<Bound<'_, PyAny>>, bool) = if !args.is_empty() {
            (Some(args.get_item(0)?), true)
        } else if let Some(t) = timeout_kwarg {
            (Some(t), true)
        } else {
            (None, false)
        };

        // Handle based on whether a timeout argument was provided
        if !has_timeout_arg {
            // Check if any individual timeout was provided without a default
            let any_individual_set = connect.is_some() || read.is_some() || write.is_some() || pool.is_some();
            let all_individual_set = connect.is_some() && read.is_some() && write.is_some() && pool.is_some();

            if any_individual_set && !all_individual_set {
                // Some individual timeouts provided without a default or all four
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "httpx.Timeout must either include a default, or set all four parameters explicitly."
                ));
            }

            // Timeout() - no timeout arg provided, use default values (5.0 for all)
            // OR all four individual timeouts were explicitly set
            return Ok(Self {
                connect: connect.or(Some(5.0)),
                read: read.or(Some(5.0)),
                write: write.or(Some(5.0)),
                pool: pool.or(Some(5.0)),
            });
        }

        let timeout = timeout_value.unwrap();

        // Check if timeout is explicitly Python None
        if timeout.is_none() {
            // Timeout(None) or Timeout(timeout=None) - all values are None (unless keyword args override)
            return Ok(Self {
                connect,
                read,
                write,
                pool,
            });
        }

        // Try tuple format: Timeout(timeout=(connect, read, write, pool))
        if let Ok(tuple) = timeout.downcast::<PyTuple>() {
            let len = tuple.len();
            if len != 4 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "timeout tuple must have 4 elements (connect, read, write, pool)",
                ));
            }
            let c: Option<f64> = tuple.get_item(0)?.extract()?;
            let r: Option<f64> = tuple.get_item(1)?.extract()?;
            let w: Option<f64> = tuple.get_item(2)?.extract()?;
            let p: Option<f64> = tuple.get_item(3)?.extract()?;
            return Ok(Self {
                connect: c,
                read: r,
                write: w,
                pool: p,
            });
        }

        // Try Timeout instance: Timeout(existing_timeout)
        if timeout.is_instance_of::<Timeout>() {
            let c: Option<f64> = timeout.getattr("connect")?.extract()?;
            let r: Option<f64> = timeout.getattr("read")?.extract()?;
            let w: Option<f64> = timeout.getattr("write")?.extract()?;
            let p: Option<f64> = timeout.getattr("pool")?.extract()?;
            return Ok(Self {
                connect: c,
                read: r,
                write: w,
                pool: p,
            });
        }

        // Try float: Timeout(5.0) or Timeout(timeout=5.0)
        if let Ok(seconds) = timeout.extract::<f64>() {
            return Ok(Self {
                connect: connect.or(Some(seconds)),
                read: read.or(Some(seconds)),
                write: write.or(Some(seconds)),
                pool: pool.or(Some(seconds)),
            });
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "timeout must be a float, tuple, Timeout instance, or None",
        ))
    }

    fn as_dict(&self) -> std::collections::HashMap<String, Option<f64>> {
        let mut map = std::collections::HashMap::new();
        map.insert("connect".to_string(), self.connect);
        map.insert("read".to_string(), self.read);
        map.insert("write".to_string(), self.write);
        map.insert("pool".to_string(), self.pool);
        map
    }

    fn __eq__(&self, other: &Timeout) -> bool {
        self.connect == other.connect
            && self.read == other.read
            && self.write == other.write
            && self.pool == other.pool
    }

    fn __repr__(&self) -> String {
        // Helper to format f64 with at least one decimal place
        let fmt_f64 = |v: f64| {
            if v.fract() == 0.0 {
                format!("{:.1}", v)  // 5 -> 5.0
            } else {
                format!("{}", v)     // 5.5 -> 5.5
            }
        };

        // If all values are the same and not None, use short form
        if self.connect == self.read && self.read == self.write && self.write == self.pool {
            if let Some(t) = self.connect {
                return format!("Timeout(timeout={})", fmt_f64(t));
            }
        }
        // Otherwise use long form
        let fmt_opt = |opt: Option<f64>| {
            match opt {
                Some(v) => fmt_f64(v),
                None => "None".to_string(),
            }
        };
        format!(
            "Timeout(connect={}, read={}, write={}, pool={})",
            fmt_opt(self.connect),
            fmt_opt(self.read),
            fmt_opt(self.write),
            fmt_opt(self.pool)
        )
    }
}

/// Connection pool limits
#[pyclass(name = "Limits")]
#[derive(Clone, Debug)]
pub struct Limits {
    #[pyo3(get)]
    pub max_connections: Option<usize>,
    #[pyo3(get)]
    pub max_keepalive_connections: Option<usize>,
    #[pyo3(get)]
    pub keepalive_expiry: Option<f64>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_connections: Some(100),
            max_keepalive_connections: Some(20),
            keepalive_expiry: Some(5.0),
        }
    }
}

#[pymethods]
impl Limits {
    #[new]
    #[pyo3(signature = (*, max_connections=None, max_keepalive_connections=None, keepalive_expiry=None))]
    fn new(
        max_connections: Option<usize>,
        max_keepalive_connections: Option<usize>,
        keepalive_expiry: Option<f64>,
    ) -> Self {
        // Only apply defaults for keepalive_expiry, others stay None if not provided
        Self {
            max_connections,
            max_keepalive_connections,
            keepalive_expiry: keepalive_expiry.or(Some(5.0)),
        }
    }

    fn __eq__(&self, other: &Limits) -> bool {
        self.max_connections == other.max_connections
            && self.max_keepalive_connections == other.max_keepalive_connections
            && self.keepalive_expiry == other.keepalive_expiry
    }

    fn __repr__(&self) -> String {
        let fmt_opt_usize = |opt: Option<usize>| match opt {
            Some(v) => format!("{}", v),
            None => "None".to_string(),
        };
        let fmt_opt_f64 = |opt: Option<f64>| match opt {
            Some(v) => {
                if v.fract() == 0.0 {
                    format!("{:.1}", v)  // 5 -> 5.0
                } else {
                    format!("{}", v)
                }
            },
            None => "None".to_string(),
        };
        format!(
            "Limits(max_connections={}, max_keepalive_connections={}, keepalive_expiry={})",
            fmt_opt_usize(self.max_connections),
            fmt_opt_usize(self.max_keepalive_connections),
            fmt_opt_f64(self.keepalive_expiry)
        )
    }
}

/// Proxy configuration
#[pyclass(name = "Proxy")]
#[derive(Clone, Debug)]
pub struct Proxy {
    url: URL,
    auth: Option<(String, String)>,
    headers_map: HashMap<String, String>,
}

#[pymethods]
impl Proxy {
    #[new]
    #[pyo3(signature = (url, *, auth=None, headers=None))]
    fn new(
        url: &str,
        auth: Option<(String, String)>,
        headers: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let parsed_url = URL::parse(url)?;

        // Validate proxy scheme
        let inner_url = parsed_url.inner();
        let scheme = inner_url.scheme();
        if scheme != "http" && scheme != "https" && scheme != "socks4" && scheme != "socks5" {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid proxy scheme '{}'. Must be http, https, socks4, or socks5.",
                scheme
            )));
        }

        // Extract auth from URL if present and no explicit auth provided
        let final_auth = if auth.is_some() {
            auth
        } else {
            let username = inner_url.username();
            let password = inner_url.password();
            if !username.is_empty() {
                Some((
                    username.to_string(),
                    password.unwrap_or("").to_string(),
                ))
            } else {
                None
            }
        };

        // Parse headers if provided
        let mut headers_map = HashMap::new();
        if let Some(h) = headers {
            for (key, value) in h.iter() {
                let k: String = key.extract()?;
                let v: String = value.extract()?;
                headers_map.insert(k, v);
            }
        }

        // Create clean URL (without auth, with normalized path)
        let host = inner_url.host_str().unwrap_or("");
        let port = inner_url.port();
        let path = inner_url.path();
        // Only include path if it's not just "/"
        let path_str = if path == "/" { "" } else { path };

        let url_str = if let Some(p) = port {
            format!("{}://{}:{}{}", scheme, host, p, path_str)
        } else {
            format!("{}://{}{}", scheme, host, path_str)
        };
        let clean_url = URL::parse(&url_str)?;

        Ok(Self {
            url: clean_url,
            auth: final_auth,
            headers_map,
        })
    }

    #[getter]
    fn url(&self) -> URL {
        self.url.clone()
    }

    #[getter]
    fn auth(&self) -> Option<(String, String)> {
        self.auth.clone()
    }

    #[getter]
    fn headers<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers_map {
            dict.set_item(k, v)?;
        }
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        if let Some(ref auth) = self.auth {
            format!(
                "Proxy('{}', auth=('{}', '********'))",
                self.url.to_string(),
                auth.0
            )
        } else {
            format!("Proxy('{}')", self.url.to_string())
        }
    }
}
