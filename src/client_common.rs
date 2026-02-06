//! Shared utilities for Client and AsyncClient request building.
//!
//! This module contains common logic used by both sync and async HTTP clients
//! to reduce code duplication.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

use crate::cookies::Cookies;
use crate::headers::Headers;
use crate::types::BasicAuth;

/// Result of extracting auth from a Python parameter.
/// Used to determine what authentication to apply to a request.
pub enum AuthAction {
    /// Use the client's default auth (if any)
    UseClientDefault,
    /// Explicitly disable auth for this request
    Disabled,
    /// Use Basic auth with these credentials
    Basic(String, String),
    /// Use a callable auth that will modify the request
    Callable(Py<PyAny>),
}

/// Extract auth action from a Python auth parameter.
///
/// Handles the three-way auth logic:
/// 1. `_AuthUnset` sentinel → use client auth
/// 2. `_AuthDisabled` sentinel or Python None → disable auth
/// 3. `BasicAuth` or `(user, pass)` tuple → use Basic auth
/// 4. Callable → use callable auth
pub fn extract_auth_action(py: Python<'_>, auth: Option<&Py<PyAny>>) -> AuthAction {
    if let Some(a) = auth {
        let a_bound = a.bind(py);

        // Check type name for sentinels
        if let Ok(type_name) = a_bound.get_type().name() {
            let type_str = type_name.to_string();
            // _AuthUnset sentinel - use client auth
            if type_str == "_AuthUnset" {
                return AuthAction::UseClientDefault;
            }
            // _AuthDisabled sentinel - disable auth
            if type_str == "_AuthDisabled" {
                return AuthAction::Disabled;
            }
        }

        // Check if it's Python's None
        if a_bound.is_none() {
            return AuthAction::Disabled;
        }

        // Try BasicAuth extraction
        if let Ok(basic) = a_bound.extract::<BasicAuth>() {
            return AuthAction::Basic(basic.username, basic.password);
        }

        // Try tuple extraction
        if let Ok(tuple) = a_bound.extract::<(String, String)>() {
            return AuthAction::Basic(tuple.0, tuple.1);
        }

        // Check if callable
        if a_bound.is_callable() {
            return AuthAction::Callable(a.clone_ref(py));
        }

        // Unknown auth type, disable auth
        AuthAction::Disabled
    } else {
        // No per-request auth specified (Rust None), fall back to client-level auth
        AuthAction::UseClientDefault
    }
}

/// Extract auth action from a Bound PyAny reference (for sync client).
///
/// Same logic as `extract_auth_action` but takes a direct reference.
pub fn extract_auth_action_bound(auth: Option<&Bound<'_, PyAny>>) -> AuthAction {
    if let Some(a) = auth {
        // Check type name for sentinels
        if let Ok(type_name) = a.get_type().name() {
            let type_str = type_name.to_string();
            // _AuthUnset sentinel - use client auth
            if type_str == "_AuthUnset" {
                return AuthAction::UseClientDefault;
            }
            // _AuthDisabled sentinel - disable auth
            if type_str == "_AuthDisabled" {
                return AuthAction::Disabled;
            }
        }

        // Check if it's Python's None
        if a.is_none() {
            return AuthAction::Disabled;
        }

        // Try BasicAuth extraction
        if let Ok(basic) = a.extract::<BasicAuth>() {
            return AuthAction::Basic(basic.username, basic.password);
        }

        // Try tuple extraction
        if let Ok(tuple) = a.extract::<(String, String)>() {
            return AuthAction::Basic(tuple.0, tuple.1);
        }

        // Check if callable - clone the reference before unbinding
        if a.is_callable() {
            return AuthAction::Callable(a.clone().unbind());
        }

        // Unknown auth type, disable auth
        AuthAction::Disabled
    } else {
        // No per-request auth specified (Rust None), fall back to client-level auth
        AuthAction::UseClientDefault
    }
}

/// Merge headers from a Python object into a target Headers instance.
///
/// Handles:
/// - `Headers` object: merge all key-value pairs
/// - `dict`: merge as key-value pairs
/// - `list` of tuples: append each (preserves duplicate headers)
pub fn merge_headers_from_py(source: &Bound<'_, PyAny>, target: &mut Headers) -> PyResult<()> {
    if let Ok(headers_obj) = source.extract::<Headers>() {
        for (k, v) in headers_obj.inner() {
            target.set(k.clone(), v.clone());
        }
    } else if let Ok(dict) = source.cast::<PyDict>() {
        for (key, value) in dict.iter() {
            let k: String = key.extract()?;
            let v: String = value.extract()?;
            target.set(k, v);
        }
    } else if let Ok(list) = source.cast::<PyList>() {
        // Handle list of tuples (for repeated headers)
        for item in list.iter() {
            let tuple = item.cast::<PyTuple>()?;
            let k: String = tuple.get_item(0)?.extract()?;
            let v: String = tuple.get_item(1)?.extract()?;
            // For repeated headers, we need to append not replace
            target.append(k, v);
        }
    }
    Ok(())
}

/// Merge cookies from a Python Cookies object into a target Cookies instance.
pub fn merge_cookies_from_py(source: &Bound<'_, PyAny>, target: &mut Cookies) -> PyResult<()> {
    if let Ok(cookies_obj) = source.extract::<Cookies>() {
        for (k, v) in cookies_obj.inner() {
            target.set(&k, &v);
        }
    } else if let Ok(dict) = source.cast::<PyDict>() {
        for (key, value) in dict.iter() {
            if let (Ok(k), Ok(v)) = (key.extract::<String>(), value.extract::<String>()) {
                target.set(&k, &v);
            }
        }
    }
    Ok(())
}

/// Create event_hooks dict for Python getter.
pub fn create_event_hooks_dict<'py>(py: Python<'py>, request_hooks: &[Py<PyAny>], response_hooks: &[Py<PyAny>]) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);

    let request_list = PyList::new(py, request_hooks.iter().map(|h| h.bind(py)))?;
    let response_list = PyList::new(py, response_hooks.iter().map(|h| h.bind(py)))?;

    dict.set_item("request", request_list)?;
    dict.set_item("response", response_list)?;

    Ok(dict)
}

/// Parse event_hooks dict from Python setter.
///
/// Returns (request_hooks, response_hooks) vectors.
#[allow(clippy::type_complexity)]
pub fn parse_event_hooks_dict(hooks: &Bound<'_, PyDict>) -> PyResult<(Vec<Py<PyAny>>, Vec<Py<PyAny>>)> {
    let mut request_hooks = Vec::new();
    let mut response_hooks = Vec::new();

    if let Some(request_list) = hooks.get_item("request")? {
        if let Ok(list) = request_list.cast::<PyList>() {
            for item in list.iter() {
                request_hooks.push(item.unbind());
            }
        }
    }

    if let Some(response_list) = hooks.get_item("response")? {
        if let Ok(list) = response_list.cast::<PyList>() {
            for item in list.iter() {
                response_hooks.push(item.unbind());
            }
        }
    }

    Ok((request_hooks, response_hooks))
}

/// Apply Basic auth credentials to headers.
pub fn apply_basic_auth(headers: &mut Headers, username: &str, password: &str) {
    let credentials = format!("{}:{}", username, password);
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, credentials.as_bytes());
    headers.set("Authorization".to_string(), format!("Basic {}", encoded));
}

/// Apply auth from URL userinfo to headers if no Authorization header is set.
pub fn apply_url_auth(headers: &mut Headers, url: &crate::url::URL) {
    if !headers.contains("authorization") {
        let url_username = url.get_username();
        if !url_username.is_empty() {
            let url_password = url.get_password().unwrap_or_default();
            apply_basic_auth(headers, &url_username, &url_password);
        }
    }
}

/// Resolve effective auth and apply to headers.
///
/// This combines auth extraction and application:
/// - For `UseClientDefault`: apply client auth if present
/// - For `Basic`: apply the provided credentials
/// - For `Disabled` or `Callable`: do nothing (callable handled separately)
///
/// Returns the callable auth if present (needs special handling by caller).
/// Takes ownership of the AuthAction since Py<PyAny> cannot be cloned.
pub fn resolve_and_apply_auth(auth_action: AuthAction, client_auth: &Option<(String, String)>, headers: &mut Headers) -> Option<Py<PyAny>> {
    match auth_action {
        AuthAction::UseClientDefault => {
            if let Some((username, password)) = client_auth {
                apply_basic_auth(headers, username, password);
            }
            None
        }
        AuthAction::Disabled => None,
        AuthAction::Basic(username, password) => {
            apply_basic_auth(headers, &username, &password);
            None
        }
        AuthAction::Callable(auth_fn) => Some(auth_fn),
    }
}
