//! Query parameter parsing utilities
//!
//! Provides functions for parsing URL query parameters from Python objects.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// Parse query parameters from Python object
#[inline]
pub fn parse_params(params_obj: &Bound<'_, PyAny>) -> PyResult<HashMap<String, String>> {
    let mut params = HashMap::new();

    if let Ok(dict) = params_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;
            params.insert(key_str, value_str);
        }
    }

    Ok(params)
}
