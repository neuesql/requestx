//! Request body parsing utilities
//!
//! Provides functions for parsing request body data (text, bytes, JSON, files)
//! from Python objects.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use sonic_rs::Value;
use std::collections::HashMap;

use super::super::super::core::http_client::FilePart;
use super::super::super::core::http_client::RequestData;
use super::super::super::error::RequestxError;
use super::headers::detect_content_type;

/// Parse request data from Python object
pub fn parse_data(data_obj: &Bound<'_, PyAny>) -> PyResult<RequestData> {
    // Try string first
    if let Ok(text) = data_obj.extract::<String>() {
        return Ok(RequestData::Text(text));
    }

    // Try bytes
    if let Ok(bytes) = data_obj.extract::<Vec<u8>>() {
        return Ok(RequestData::Bytes(bytes));
    }

    // Try dict (form data)
    if let Ok(dict) = data_obj.downcast::<PyDict>() {
        let mut form_data = HashMap::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let value_str = value.extract::<String>()?;
            form_data.insert(key_str, value_str);
        }
        return Ok(RequestData::Form(form_data));
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Data must be string, bytes, or dict",
    ))
}

/// Parse JSON data from Python object
pub fn parse_json(py: Python, json_obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Use Python's json module to serialize the object
    let json_module = py.import("json")?;
    let json_str = json_module
        .call_method1("dumps", (json_obj,))?
        .extract::<String>()?;

    // Parse the JSON string into sonic_rs::Value
    sonic_rs::from_str(&json_str).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse JSON: {e}"))
    })
}

/// Parse files from Python object for multipart upload
///
/// Supports formats:
/// - `{'fieldname': file_object}` - file object with fieldname as key
/// - `{'fieldname': ('filename', file_object)}` - with custom filename
/// - `{'fieldname': ('filename', file_object, 'content_type')}` - with content type
/// - List format for multiple files with same fieldname
pub fn parse_files(files_obj: &Bound<'_, PyAny>) -> PyResult<Vec<FilePart>> {
    let mut file_parts = Vec::new();

    // Handle list format: [('fieldname', ('filename', data)), ...]
    if let Ok(list) = files_obj.downcast::<pyo3::types::PyList>() {
        for i in 0..list.len() {
            let item = list.get_item(i)?;
            // Try to extract as tuple: (field_name, file_info)
            let tuple = item.downcast::<pyo3::types::PyTuple>()?;
            if tuple.len() == 2 {
                let field_name = tuple.get_item(0).unwrap().extract::<String>()?;
                let file_info = tuple.get_item(1).unwrap();

                let parts = parse_single_file(&field_name, file_info)?;
                file_parts.extend(parts);
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "File tuple must have 2 elements: (fieldname, file_data)",
                ));
            }
        }
    }
    // Handle dict format: {'fieldname': file_or_tuple}
    else if let Ok(dict) = files_obj.downcast::<PyDict>() {
        for (key, value) in dict.iter() {
            let field_name = key.extract::<String>()?;
            let parts = parse_single_file(&field_name, value)?;
            file_parts.extend(parts);
        }
    }
    // Handle single file passed directly (not in dict)
    else {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Files must be a dictionary or list of tuples",
        ));
    }

    Ok(file_parts)
}

/// Parse a single file entry into FilePart(s)
pub fn parse_single_file(field_name: &str, file_info: Bound<'_, PyAny>) -> PyResult<Vec<FilePart>> {
    let mut parts = Vec::new();

    // Try: ('filename', file_object_or_data)
    if let Ok(tuple) = file_info.downcast::<pyo3::types::PyTuple>() {
        if tuple.len() == 2 || tuple.len() == 3 {
            let filename = tuple.get_item(0).unwrap().extract::<String>()?;
            let file_data = tuple.get_item(1).unwrap();

            let content_type = if tuple.len() == 3 {
                Some(tuple.get_item(2).unwrap().extract::<String>()?)
            } else {
                detect_content_type(&filename)
            };

            // Extract file data - can be file object, bytes, or string
            let data = extract_file_data(file_data)?;

            parts.push(FilePart {
                field_name: field_name.to_string(),
                filename,
                content_type,
                data,
            });
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "File tuple must have 2 or 3 elements: ('filename', data) or ('filename', data, 'content_type')",
            ));
        }
    }
    // Try: file object directly
    else {
        // For file objects, try to get name for filename
        let filename = if let Ok(name_attr) = file_info.getattr("name") {
            if let Ok(name) = name_attr.extract::<String>() {
                name
            } else {
                "file".to_string()
            }
        } else {
            "file".to_string()
        };

        let content_type = detect_content_type(&filename);
        let data = extract_file_data(file_info)?;

        parts.push(FilePart {
            field_name: field_name.to_string(),
            filename,
            content_type,
            data,
        });
    }

    Ok(parts)
}

/// Extract file data from various Python objects
pub fn extract_file_data(obj: Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    // Try: file object with read() method
    if let Ok(read_method) = obj.getattr("read") {
        let content = read_method.call0()?.extract::<Vec<u8>>()?;
        // Reset file position if possible
        let _ = obj.call_method1("seek", (0,));
        return Ok(content);
    }

    // Try: bytes
    if let Ok(bytes) = obj.extract::<Vec<u8>>() {
        return Ok(bytes);
    }

    // Try: string
    if let Ok(string) = obj.extract::<String>() {
        return Ok(string.into_bytes());
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "File data must be a file object, bytes, or string",
    ))
}
