use pyo3::prelude::*;
use pyo3::types::{PyDict, PyBytes};
use std::collections::HashMap;
use serde_json::Value;

use crate::error::RequestxError;

/// Response object compatible with requests.Response
#[pyclass]
pub struct Response {
    #[pyo3(get)]
    status_code: u16,
    
    #[pyo3(get)]
    url: String,
    
    headers: HashMap<String, String>,
    text_content: Option<String>,
    binary_content: Option<Vec<u8>>,
    encoding: Option<String>,
}

#[pymethods]
impl Response {
    #[new]
    fn new(
        status_code: u16,
        url: String,
        headers: HashMap<String, String>,
        content: Vec<u8>,
    ) -> Self {
        Response {
            status_code,
            url,
            headers,
            text_content: None,
            binary_content: Some(content),
            encoding: None,
        }
    }
    
    /// Get response headers as a dictionary
    #[getter]
    fn headers(&self) -> PyResult<HashMap<String, String>> {
        Ok(self.headers.clone())
    }
    
    /// Get response text content
    #[getter]
    fn text(&mut self) -> PyResult<String> {
        if let Some(ref text) = self.text_content {
            return Ok(text.clone());
        }
        
        if let Some(ref content) = self.binary_content {
            let text = String::from_utf8_lossy(content).to_string();
            self.text_content = Some(text.clone());
            Ok(text)
        } else {
            Ok(String::new())
        }
    }
    
    /// Get response binary content
    #[getter]
    fn content(&self, py: Python) -> PyResult<PyObject> {
        if let Some(ref content) = self.binary_content {
            Ok(PyBytes::new(py, content).into())
        } else {
            Ok(PyBytes::new(py, &[]).into())
        }
    }
    
    /// Parse response as JSON
    fn json(&mut self) -> PyResult<PyObject> {
        let text = self.text()?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| RequestxError::JsonDecodeError(e))?;
        
        Python::with_gil(|py| {
            pythonize::pythonize(py, &value)
                .map_err(|e| RequestxError::PythonError(e.to_string()).into())
        })
    }
    
    /// Raise an exception for HTTP error status codes
    fn raise_for_status(&self) -> PyResult<()> {
        if self.status_code >= 400 {
            let error = RequestxError::HttpError {
                status: self.status_code,
                message: format!("HTTP {} error", self.status_code),
            };
            return Err(error.into());
        }
        Ok(())
    }
    
    /// Get response encoding
    #[getter]
    fn encoding(&self) -> Option<String> {
        self.encoding.clone()
    }
    
    /// Set response encoding
    #[setter]
    fn set_encoding(&mut self, encoding: Option<String>) {
        self.encoding = encoding;
    }
}