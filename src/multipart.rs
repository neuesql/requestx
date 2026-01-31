//! Multipart form data encoding

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

/// Generate a random boundary string for multipart forms
pub fn generate_boundary() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("----WebKitFormBoundary{:x}", timestamp)
}

/// Extract boundary from Content-Type header
pub fn extract_boundary_from_content_type(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if part.starts_with("boundary=") {
            let boundary = part.strip_prefix("boundary=").unwrap();
            // Remove quotes if present
            let boundary = boundary.trim_matches('"').trim_matches('\'');
            return Some(boundary.trim().to_string());
        }
    }
    None
}

/// Build multipart body with auto-generated boundary
pub fn build_multipart_body(
    py: Python<'_>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
) -> PyResult<(Vec<u8>, String)> {
    let boundary = generate_boundary();
    let body = build_multipart_body_with_boundary(py, data, files, &boundary)?;
    Ok((body.0, boundary))
}

/// Build multipart body with specified boundary
pub fn build_multipart_body_with_boundary(
    py: Python<'_>,
    data: Option<&Bound<'_, PyDict>>,
    files: Option<&Bound<'_, PyAny>>,
    boundary: &str,
) -> PyResult<(Vec<u8>, String)> {
    let mut body = Vec::new();
    let boundary_bytes = boundary.as_bytes();

    // Add data fields first
    if let Some(d) = data {
        for (key, value) in d.iter() {
            // Validate key type - must be str
            if !key.is_instance_of::<pyo3::types::PyString>() {
                return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                    "Invalid type for name {}. Expected str.",
                    key.repr()?.to_str()?
                )));
            }
            let k: String = key.extract()?;
            // Handle different value types
            add_data_field(py, &mut body, boundary_bytes, &k, &value)?;
        }
    }

    // Add file fields
    if let Some(f) = files {
        // Handle both dict and list of tuples
        let file_items: Vec<(String, Bound<'_, PyAny>)> = if let Ok(dict) = f.downcast::<PyDict>() {
            dict.iter()
                .map(|(k, v)| (k.extract::<String>().unwrap_or_default(), v))
                .collect()
        } else if let Ok(list) = f.downcast::<pyo3::types::PyList>() {
            list.iter()
                .filter_map(|item| {
                    if let Ok(tuple) = item.downcast::<PyTuple>() {
                        if tuple.len() >= 2 {
                            let name = tuple.get_item(0).ok()?.extract::<String>().ok()?;
                            let value = tuple.get_item(1).ok()?;
                            Some((name, value))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        for (field_name, value) in file_items {
            // Files can be:
            // - file-like object (has read() method)
            // - tuple: (filename, file-content)
            // - tuple: (filename, file-content, content-type)
            // - tuple: (filename, file-content, content-type, headers)
            let (filename, content, content_type, extra_headers) = parse_file_value(py, &value, &field_name)?;

            body.extend_from_slice(b"--");
            body.extend_from_slice(boundary_bytes);
            body.extend_from_slice(b"\r\n");

            // Build Content-Disposition header with escaped filename
            if let Some(ref fname) = filename {
                let escaped_fname = escape_filename(fname);
                body.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                    field_name, escaped_fname
                ).as_bytes());
            } else {
                // No filename - just field name
                body.extend_from_slice(format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n",
                    field_name
                ).as_bytes());
            }

            // Add extra headers first (before Content-Type), but skip Content-Type if in headers
            let mut has_content_type_header = false;
            for (hk, hv) in &extra_headers {
                if hk.to_lowercase() == "content-type" {
                    has_content_type_header = true;
                } else {
                    body.extend_from_slice(format!("{}: {}\r\n", hk, hv).as_bytes());
                }
            }

            // Add content-type if we have a filename
            if filename.is_some() {
                // Use Content-Type from extra_headers if provided, otherwise use guessed type
                if has_content_type_header {
                    for (hk, hv) in &extra_headers {
                        if hk.to_lowercase() == "content-type" {
                            body.extend_from_slice(format!("Content-Type: {}\r\n", hv).as_bytes());
                            break;
                        }
                    }
                } else {
                    body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
                }
            }

            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(&content);
            body.extend_from_slice(b"\r\n");
        }
    }

    // Add closing boundary
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary_bytes);
    body.extend_from_slice(b"--\r\n");

    Ok((body, boundary.to_string()))
}

/// Add a data field to the multipart body
fn add_data_field(
    py: Python<'_>,
    body: &mut Vec<u8>,
    boundary_bytes: &[u8],
    key: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<()> {
    // Check if value is a list - if so, add multiple fields with same name
    if let Ok(list) = value.downcast::<PyList>() {
        for item in list.iter() {
            add_single_data_field(py, body, boundary_bytes, key, &item)?;
        }
        return Ok(());
    }

    // Single value
    add_single_data_field(py, body, boundary_bytes, key, value)
}

/// Add a single data field to the multipart body
fn add_single_data_field(
    _py: Python<'_>,
    body: &mut Vec<u8>,
    boundary_bytes: &[u8],
    key: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<()> {
    use pyo3::types::{PyBool, PyFloat, PyInt, PyString, PyBytes as PyBytesType};

    // Validate value type - must be str, bytes, int, float, bool, or None
    // Check for dict explicitly to give proper error message
    if value.downcast::<PyDict>().is_ok() {
        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "Invalid type for value: {}. Expected str.",
            value.get_type().name()?
        )));
    }

    // Handle different value types
    let v_bytes: Vec<u8> = if let Ok(s) = value.extract::<String>() {
        s.into_bytes()
    } else if let Ok(b) = value.extract::<Vec<u8>>() {
        b
    } else if value.downcast::<PyBool>().is_ok() {
        // Check bool before int (since bool is subclass of int in Python)
        let b: bool = value.extract()?;
        if b { b"true".to_vec() } else { b"false".to_vec() }
    } else if let Ok(i) = value.extract::<i64>() {
        i.to_string().into_bytes()
    } else if let Ok(f) = value.extract::<f64>() {
        f.to_string().into_bytes()
    } else if value.is_none() {
        b"".to_vec()
    } else if value.is_instance_of::<PyString>() || value.is_instance_of::<PyBytesType>()
           || value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>()
           || value.is_instance_of::<PyBool>() {
        value.str()?.to_string().into_bytes()
    } else {
        // Invalid type - raise TypeError
        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "Invalid type for value: {}. Expected str.",
            value.get_type().name()?
        )));
    };

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n", key).as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(&v_bytes);
    body.extend_from_slice(b"\r\n");

    Ok(())
}

/// Parse a file value which can be a file-like object or tuple
fn parse_file_value(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    field_name: &str,
) -> PyResult<(Option<String>, Vec<u8>, String, Vec<(String, String)>)> {
    // Check if it's a tuple: (filename, content) or (filename, content, content_type) or (filename, content, content_type, headers)
    if let Ok(tuple) = value.downcast::<PyTuple>() {
        let len = tuple.len();
        if len >= 2 {
            // Get filename (can be None)
            let filename: Option<String> = if tuple.get_item(0)?.is_none() {
                None
            } else {
                Some(tuple.get_item(0)?.extract::<String>().unwrap_or_else(|_| "upload".to_string()))
            };

            // Get content
            let content_item = tuple.get_item(1)?;
            let content = read_file_content(py, &content_item)?;

            // Get content type if provided
            let content_type = if len >= 3 {
                let ct_item = tuple.get_item(2)?;
                if ct_item.is_none() {
                    guess_content_type(filename.as_deref().unwrap_or(""))
                } else {
                    ct_item.extract::<String>().unwrap_or_else(|_| guess_content_type(filename.as_deref().unwrap_or("")))
                }
            } else {
                guess_content_type(filename.as_deref().unwrap_or(""))
            };

            // Get extra headers if provided
            let extra_headers = if len >= 4 {
                let headers_item = tuple.get_item(3)?;
                if let Ok(dict) = headers_item.downcast::<PyDict>() {
                    let mut headers = Vec::new();
                    for (k, v) in dict.iter() {
                        headers.push((k.extract::<String>()?, v.extract::<String>()?));
                    }
                    headers
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            return Ok((filename, content, content_type, extra_headers));
        }
    }

    // It's a file-like object
    let content = read_file_content(py, value)?;
    let filename = Some("upload".to_string());
    let content_type = "application/octet-stream".to_string();

    Ok((filename, content, content_type, Vec::new()))
}

/// Read content from a file-like object or bytes/string
pub fn read_file_content(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    // Try to extract as bytes directly
    if let Ok(bytes) = value.extract::<Vec<u8>>() {
        return Ok(bytes);
    }

    // Try to extract as string
    if let Ok(s) = value.extract::<String>() {
        return Ok(s.into_bytes());
    }

    // Check if it's a StringIO (text mode) - should raise TypeError
    let io_mod = py.import("io")?;
    let string_io_type = io_mod.getattr("StringIO")?;
    if value.is_instance(&string_io_type)? {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "Multipart file uploads require 'io.IOBase', not 'io.StringIO'."
        ));
    }

    // Check if it's a text mode file (TextIOWrapper)
    let text_io_wrapper_type = io_mod.getattr("TextIOWrapper")?;
    if value.is_instance(&text_io_wrapper_type)? {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "Attempted to upload a file-like object without 'rb' mode. Make sure to open the file with 'rb' mode."
        ));
    }

    // Try to call read() method (file-like object)
    if let Ok(read_method) = value.getattr("read") {
        // Rewind file if possible (seek to beginning)
        if let Ok(seek_method) = value.getattr("seek") {
            let _ = seek_method.call1((0i64,));
        }

        let content = read_method.call0()?;
        if let Ok(bytes) = content.extract::<Vec<u8>>() {
            return Ok(bytes);
        }
        // If read() returns string, it's text mode - raise TypeError
        if content.extract::<String>().is_ok() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Multipart file uploads must be opened in binary mode."
            ));
        }
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "File content must be bytes, str, or a file-like object with read() method"
    ))
}

/// Escape filename for Content-Disposition header (HTML5/RFC 5987)
/// - Backslash is escaped as \\
/// - Quote is percent-encoded as %22
/// - Control characters (except 0x1B escape) are percent-encoded
fn escape_filename(filename: &str) -> String {
    let mut result = String::new();
    for c in filename.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("%22"),
            // Control characters: 0x00-0x1F except 0x1B (escape)
            c if (c as u32) < 0x20 && c != '\x1B' => {
                result.push_str(&format!("%{:02X}", c as u32));
            }
            _ => result.push(c),
        }
    }
    result
}

/// Guess content type from filename
pub fn guess_content_type(filename: &str) -> String {
    if let Some(ext) = filename.rsplit('.').next() {
        match ext.to_lowercase().as_str() {
            "json" => "application/json".to_string(),
            "txt" => "text/plain".to_string(),
            "html" | "htm" => "text/html".to_string(),
            "xml" => "application/xml".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "pdf" => "application/pdf".to_string(),
            "zip" => "application/zip".to_string(),
            "css" => "text/css".to_string(),
            "js" => "application/javascript".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    } else {
        "application/octet-stream".to_string()
    }
}
