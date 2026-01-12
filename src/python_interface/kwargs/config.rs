//! Configuration parsing utilities
//!
//! Provides functions for parsing timeout, cert, and proxies from Python objects.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::time::Duration;

/// Parse timeout from Python object with comprehensive validation
pub fn parse_timeout(timeout_obj: &Bound<'_, PyAny>) -> PyResult<Duration> {
    if let Ok(seconds) = timeout_obj.extract::<f64>() {
        if seconds < 0.0 {
            return Err(
                RequestxError::RuntimeError("Timeout must be non-negative".to_string()).into(),
            );
        }
        if seconds > 3600.0 {
            // 1 hour max
            return Err(RequestxError::RuntimeError(
                "Timeout too large (max 3600 seconds)".to_string(),
            )
            .into());
        }
        Ok(Duration::from_secs_f64(seconds))
    } else if let Ok(seconds) = timeout_obj.extract::<u64>() {
        if seconds > 3600 {
            // 1 hour max
            return Err(RequestxError::RuntimeError(
                "Timeout too large (max 3600 seconds)".to_string(),
            )
            .into());
        }
        Ok(Duration::from_secs(seconds))
    } else {
        Err(RequestxError::RuntimeError("Timeout must be a number".to_string()).into())
    }
}

/// Parse certificate from Python object
#[inline]
pub fn parse_cert(cert_obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(cert_path) = cert_obj.extract::<String>() {
        Ok(cert_path)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Certificate must be a string path",
        ))
    }
}

/// Parse proxies from Python object
pub fn parse_proxies(proxies_obj: &Bound<'_, PyAny>) -> PyResult<HashMap<String, String>> {
    let mut proxies = HashMap::new();

    if let Ok(dict) = proxies_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let protocol = key.extract::<String>()?;
            let proxy_url = value.extract::<String>()?;

            // Validate proxy URL format
            if !proxy_url.contains("://") {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid proxy URL format: {proxy_url}"
                )));
            }

            proxies.insert(protocol, proxy_url);
        }
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Proxies must be a dictionary",
        ));
    }

    Ok(proxies)
}

use crate::error::RequestxError;
