//! Authentication implementations

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::request::Request;

/// Base Auth class that can be subclassed in Python
#[pyclass(name = "Auth", subclass)]
#[derive(Clone)]
pub struct Auth {
    requires_request_body: bool,
    requires_response_body: bool,
}

impl Default for Auth {
    fn default() -> Self {
        Self {
            requires_request_body: false,
            requires_response_body: false,
        }
    }
}

#[pymethods]
impl Auth {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(_args: &Bound<'_, pyo3::types::PyTuple>, _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>) -> Self {
        Self::default()
    }

    /// Called to get authentication flow generator
    /// Returns an iterator that yields requests
    #[pyo3(signature = (request))]
    fn auth_flow<'py>(
        &self,
        py: Python<'py>,
        request: &Request,
    ) -> PyResult<Bound<'py, PyList>> {
        // Return a list that can be iterated
        // Subclasses can override this
        let request = request.clone();
        let list = PyList::new(py, vec![request.into_pyobject(py)?])?;
        Ok(list)
    }

    /// Sync auth flow - calls auth_flow and iterates
    fn sync_auth_flow<'py>(
        &self,
        py: Python<'py>,
        request: &Request,
    ) -> PyResult<Bound<'py, PyList>> {
        self.auth_flow(py, request)
    }

    /// Async auth flow - calls auth_flow and iterates asynchronously
    fn async_auth_flow<'py>(
        &self,
        py: Python<'py>,
        request: &Request,
    ) -> PyResult<Bound<'py, PyList>> {
        self.auth_flow(py, request)
    }

    #[getter]
    fn requires_request_body(&self) -> bool {
        self.requires_request_body
    }

    #[getter]
    fn requires_response_body(&self) -> bool {
        self.requires_response_body
    }

    fn __repr__(&self) -> String {
        "<Auth>".to_string()
    }
}

/// Function-based auth that wraps a callable
#[pyclass(name = "FunctionAuth", extends = Auth)]
pub struct FunctionAuth {
    func: Py<PyAny>,
}

#[pymethods]
impl FunctionAuth {
    #[new]
    fn new(func: Py<PyAny>) -> (Self, Auth) {
        (Self { func }, Auth::default())
    }

    #[pyo3(signature = (request))]
    fn auth_flow<'py>(
        &self,
        py: Python<'py>,
        request: &Request,
    ) -> PyResult<Bound<'py, PyList>> {
        // Call the function with the request
        let result = self.func.call1(py, (request.clone(),))?;

        // If it returns a Request, wrap it in a list
        if let Ok(req) = result.extract::<Request>(py) {
            let list = PyList::new(py, vec![req.into_pyobject(py)?])?;
            return Ok(list);
        }

        // Otherwise assume it's already a list/iterable and convert to list
        let bound = result.bind(py);
        if let Ok(list) = bound.downcast::<PyList>() {
            return Ok(list.clone());
        }

        // Use Python's list() builtin to convert any iterable to list
        let builtins = py.import("builtins")?;
        let list_func = builtins.getattr("list")?;
        let py_list = list_func.call1((bound,))?;
        Ok(py_list.downcast::<PyList>()?.clone())
    }
}
