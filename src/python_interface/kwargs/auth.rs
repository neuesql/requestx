//! Authentication parsing utilities
//!
//! Provides functions for parsing authentication credentials from Python objects.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Parse authentication from Python object - supports tuple, list, and auth objects
pub fn parse_auth(auth_obj: &Bound<'_, PyAny>) -> PyResult<Option<(String, String)>> {
    // Try tuple/list first
    if let Ok(tuple) = auth_obj.extract::<(String, String)>() {
        return Ok(Some(tuple));
    }

    // Try list
    if let Ok(list) = auth_obj.downcast::<PyList>() {
        if list.len() == 2 {
            let username = list.get_item(0).unwrap().extract::<String>()?;
            let password = list.get_item(1).unwrap().extract::<String>()?;
            return Ok(Some((username, password)));
        }
    }

    // Try extracting as dict with 'username' and 'password' keys (auth object style)
    if let Ok(dict) = auth_obj.downcast::<PyDict>() {
        if let (Some(username_obj), Some(password_obj)) =
            (dict.get_item("username")?, dict.get_item("password")?)
        {
            let username = username_obj.extract::<String>()?;
            let password = password_obj.extract::<String>()?;
            return Ok(Some((username, password)));
        }
    }

    // Check for auth object with username/password attributes (e.g., HTTPDigestAuth)
    if let Ok(username_attr) = auth_obj.getattr("username") {
        if let Ok(password_attr) = auth_obj.getattr("password") {
            let username = username_attr.extract::<String>()?;
            let password = password_attr.extract::<String>()?;
            return Ok(Some((username, password)));
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Auth must be a tuple or list of (username, password)",
    ))
}
