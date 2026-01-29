//! Timeout and Limits configuration

use pyo3::prelude::*;
use std::time::Duration;

/// Marker type for unset values
#[pyclass(name = "UnsetType")]
#[derive(Clone, Debug)]
pub struct UnsetType;

#[pymethods]
impl UnsetType {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __repr__(&self) -> &'static str {
        "<UNSET>"
    }

    fn __bool__(&self) -> bool {
        false
    }
}

/// Check if a PyAny is the UNSET singleton
fn is_unset(obj: &Bound<'_, PyAny>) -> bool {
    obj.is_instance_of::<UnsetType>()
}

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
    #[pyo3(signature = (*args, **kwargs))]
    fn py_new(
        args: &Bound<'_, pyo3::types::PyTuple>,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Self> {
        // Helper to extract Option<f64> from Option<PyObject>
        let extract_opt_f64 = |obj: Option<&Bound<'_, PyAny>>| -> PyResult<Option<f64>> {
            match obj {
                None => Ok(None),
                Some(o) if o.is_none() => Ok(None),
                Some(o) => Ok(Some(o.extract::<f64>()?)),
            }
        };

        // Get positional timeout argument if provided
        let timeout_arg = if !args.is_empty() {
            if args.len() > 1 {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "Timeout() takes at most 1 positional argument"
                ));
            }
            Some(args.get_item(0)?)
        } else {
            // Check kwargs for 'timeout' key
            kwargs.and_then(|kw| kw.get_item("timeout").ok().flatten())
        };

        // Get individual timeout kwargs
        let connect_kwarg = kwargs.and_then(|kw| kw.get_item("connect").ok().flatten());
        let read_kwarg = kwargs.and_then(|kw| kw.get_item("read").ok().flatten());
        let write_kwarg = kwargs.and_then(|kw| kw.get_item("write").ok().flatten());
        let pool_kwarg = kwargs.and_then(|kw| kw.get_item("pool").ok().flatten());

        // Check if any individual fields were explicitly set
        let connect_set = connect_kwarg.is_some();
        let read_set = read_kwarg.is_some();
        let write_set = write_kwarg.is_some();
        let pool_set = pool_kwarg.is_some();

        // Extract individual timeout values
        let connect_val = extract_opt_f64(connect_kwarg.as_ref())?;
        let read_val = extract_opt_f64(read_kwarg.as_ref())?;
        let write_val = extract_opt_f64(write_kwarg.as_ref())?;
        let pool_val = extract_opt_f64(pool_kwarg.as_ref())?;

        // Handle timeout parameter based on whether it was provided
        match timeout_arg {
            None => {
                // timeout was not provided - check if individual fields are set
                if connect_set || read_set || write_set || pool_set {
                    // If any individual timeout is set, all must be set
                    if !connect_set || !read_set || !write_set || !pool_set {
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            "httpx.Timeout must either include a default, or set all four parameters explicitly."
                        ));
                    }
                    return Ok(Self {
                        connect: connect_val,
                        read: read_val,
                        write: write_val,
                        pool: pool_val,
                    });
                }
                // No timeout and no individual fields - error
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "httpx.Timeout must either include a default, or set all four parameters explicitly."
                ));
            }
            Some(ref timeout) => {
                // Check if timeout is None
                if timeout.is_none() {
                    return Ok(Self {
                        connect: connect_val,
                        read: read_val,
                        write: write_val,
                        pool: pool_val,
                    });
                }

                // Check if it's a Timeout instance
                if let Ok(timeout_obj) = timeout.extract::<Timeout>() {
                    // Copy from another Timeout, but allow overrides
                    return Ok(Self {
                        connect: if connect_set { connect_val } else { timeout_obj.connect },
                        read: if read_set { read_val } else { timeout_obj.read },
                        write: if write_set { write_val } else { timeout_obj.write },
                        pool: if pool_set { pool_val } else { timeout_obj.pool },
                    });
                }

                // Check if it's a tuple
                if let Ok(tuple) = timeout.downcast::<pyo3::types::PyTuple>() {
                    let len = tuple.len();
                    if len < 2 || len > 4 {
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            format!("Timeout tuple must have 2-4 elements, got {}", len)
                        ));
                    }
                    let extract_tuple_opt = |idx: usize| -> PyResult<Option<f64>> {
                        if idx >= len {
                            return Ok(None);
                        }
                        let item = tuple.get_item(idx)?;
                        if item.is_none() {
                            Ok(None)
                        } else {
                            Ok(Some(item.extract::<f64>()?))
                        }
                    };
                    return Ok(Self {
                        connect: if connect_set { connect_val } else { extract_tuple_opt(0)? },
                        read: if read_set { read_val } else { extract_tuple_opt(1)? },
                        write: if write_set { write_val } else { extract_tuple_opt(2)? },
                        pool: if pool_set { pool_val } else { extract_tuple_opt(3)? },
                    });
                }

                // Try to extract as a float
                if let Ok(f) = timeout.extract::<f64>() {
                    return Ok(Self {
                        connect: if connect_set { connect_val } else { Some(f) },
                        read: if read_set { read_val } else { Some(f) },
                        write: if write_set { write_val } else { Some(f) },
                        pool: if pool_set { pool_val } else { Some(f) },
                    });
                }

                Err(pyo3::exceptions::PyTypeError::new_err(
                    format!("timeout must be float, tuple, or Timeout instance, got {}", timeout.get_type().name()?)
                ))
            }
        }
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
