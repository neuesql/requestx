//! Timeout and Limits configuration

use pyo3::prelude::*;
use std::time::Duration;

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
        let timeouts = [self.connect, self.read, self.write, self.pool];
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
}

#[pymethods]
impl Timeout {
    #[new]
    #[pyo3(signature = (timeout=None, *, connect=None, read=None, write=None, pool=None))]
    fn py_new(
        timeout: Option<f64>,
        connect: Option<f64>,
        read: Option<f64>,
        write: Option<f64>,
        pool: Option<f64>,
    ) -> Self {
        Self::new(timeout, connect, read, write, pool)
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
        // Format float to always show decimal point
        let format_float = |v: f64| {
            if v.fract() == 0.0 {
                format!("{:.1}", v)
            } else {
                format!("{}", v)
            }
        };
        let format_opt = |v: Option<f64>| match v {
            Some(x) => format_float(x),
            None => "None".to_string(),
        };
        // If all values are the same, show condensed format
        if self.connect == self.read && self.read == self.write && self.write == self.pool {
            if let Some(t) = self.connect {
                return format!("Timeout(timeout={})", format_float(t));
            } else {
                return "Timeout(timeout=None)".to_string();
            }
        }
        // Otherwise show individual values
        format!(
            "Timeout(connect={}, read={}, write={}, pool={})",
            format_opt(self.connect), format_opt(self.read), format_opt(self.write), format_opt(self.pool)
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
        Self {
            max_connections,
            max_keepalive_connections,
            keepalive_expiry: keepalive_expiry.or(Some(5.0)),  // keepalive_expiry has a default
        }
    }

    fn __eq__(&self, other: &Limits) -> bool {
        self.max_connections == other.max_connections
            && self.max_keepalive_connections == other.max_keepalive_connections
            && self.keepalive_expiry == other.keepalive_expiry
    }

    fn __repr__(&self) -> String {
        let format_opt_usize = |v: Option<usize>| match v {
            Some(x) => format!("{}", x),
            None => "None".to_string(),
        };
        let format_opt_f64 = |v: Option<f64>| match v {
            Some(x) => {
                if x.fract() == 0.0 {
                    format!("{:.1}", x)
                } else {
                    format!("{}", x)
                }
            }
            None => "None".to_string(),
        };
        format!(
            "Limits(max_connections={}, max_keepalive_connections={}, keepalive_expiry={})",
            format_opt_usize(self.max_connections), format_opt_usize(self.max_keepalive_connections), format_opt_f64(self.keepalive_expiry)
        )
    }
}
