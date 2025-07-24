use pyo3::prelude::*;

mod core;
mod error;
mod response;
mod session;

use error::RequestxError;
use response::Response;
use session::Session;

/// HTTP GET request
#[pyfunction]
fn get(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP GET implementation")
}

/// HTTP POST request
#[pyfunction]
fn post(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP POST implementation")
}

/// HTTP PUT request
#[pyfunction]
fn put(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP PUT implementation")
}

/// HTTP DELETE request
#[pyfunction]
fn delete(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP DELETE implementation")
}

/// HTTP HEAD request
#[pyfunction]
fn head(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP HEAD implementation")
}

/// HTTP OPTIONS request
#[pyfunction]
fn options(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP OPTIONS implementation")
}

/// HTTP PATCH request
#[pyfunction]
fn patch(py: Python, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("HTTP PATCH implementation")
}

/// Generic HTTP request
#[pyfunction]
fn request(py: Python, method: String, url: String, kwargs: Option<&pyo3::types::PyDict>) -> PyResult<PyObject> {
    // Placeholder implementation - will be implemented in task 2
    todo!("Generic HTTP request implementation")
}

/// RequestX Python module
#[pymodule]
fn requestx(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register HTTP method functions
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(head, m)?)?;
    m.add_function(wrap_pyfunction!(options, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(request, m)?)?;
    
    // Register classes
    m.add_class::<Response>()?;
    m.add_class::<Session>()?;
    
    Ok(())
}