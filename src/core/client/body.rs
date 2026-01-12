//! Body building utilities for HTTP requests
//!
//! Provides functions for building request bodies from various data types.

use crate::core::http_client::{FilePart, RequestData};
use crate::error::RequestxError;
use hyper::Body;
use sonic_rs::Value;
use std::collections::HashMap;

// Pre-allocated common strings to reduce allocations
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_MULTIPART: &str = "multipart/form-data";

/// Result of body building: the body, whether it has a content-type, and optional multipart content-type
pub struct BuiltBody {
    pub body: Body,
    pub has_content_type: bool,
    pub multipart_content_type: Option<String>,
}

/// Build request body from data, json, and files
pub fn build_body(
    data: Option<RequestData>,
    json: Option<Value>,
    files: Option<Vec<FilePart>>,
) -> Result<BuiltBody, RequestxError> {
    match (data, json, files) {
        // Handle multipart form data with files
        (data, None, Some(files)) if !files.is_empty() => {
            let boundary = generate_boundary();
            let fields = match data {
                Some(RequestData::Form(form)) => form,
                _ => HashMap::new(),
            };
            let multipart_body = build_multipart_body(&boundary, &fields, &files);
            let ct = format!("{}; boundary={}", CONTENT_TYPE_MULTIPART, boundary);
            Ok(BuiltBody {
                body: Body::from(multipart_body),
                has_content_type: true,
                multipart_content_type: Some(ct),
            })
        }
        // Handle multipart with data but no files
        (Some(RequestData::Form(form)), None, Some(_)) => {
            let estimated_size = form
                .iter()
                .map(|(k, v)| k.len() + v.len() + 10)
                .sum::<usize>();
            let mut form_data = String::with_capacity(estimated_size);

            let mut first = true;
            for (k, v) in form.iter() {
                if !first {
                    form_data.push('&');
                }
                form_data.push_str(&urlencoding::encode(k));
                form_data.push('=');
                form_data.push_str(&urlencoding::encode(v));
                first = false;
            }
            Ok(BuiltBody {
                body: Body::from(form_data),
                has_content_type: true,
                multipart_content_type: None,
            })
        }
        (Some(RequestData::Text(text)), None, None) => Ok(BuiltBody {
            body: Body::from(text),
            has_content_type: false,
            multipart_content_type: None,
        }),
        (Some(RequestData::Bytes(bytes)), None, None) => Ok(BuiltBody {
            body: Body::from(bytes),
            has_content_type: false,
            multipart_content_type: None,
        }),
        (Some(RequestData::Form(form)), None, None) => {
            let estimated_size = form
                .iter()
                .map(|(k, v)| k.len() + v.len() + 10)
                .sum::<usize>();
            let mut form_data = String::with_capacity(estimated_size);

            let mut first = true;
            for (k, v) in form.iter() {
                if !first {
                    form_data.push('&');
                }
                form_data.push_str(&urlencoding::encode(k));
                form_data.push('=');
                form_data.push_str(&urlencoding::encode(v));
                first = false;
            }
            Ok(BuiltBody {
                body: Body::from(form_data),
                has_content_type: true,
                multipart_content_type: None,
            })
        }
        (None, Some(json), None) => {
            let json_string = sonic_rs::to_string(&json)?;
            Ok(BuiltBody {
                body: Body::from(json_string),
                has_content_type: true,
                multipart_content_type: None,
            })
        }
        (None, None, None) => Ok(BuiltBody {
            body: Body::empty(),
            has_content_type: false,
            multipart_content_type: None,
        }),
        (Some(_), Some(_), _) => Err(RequestxError::RuntimeError(
            "Cannot specify both data and json parameters".to_string(),
        )),
        _ => Err(RequestxError::RuntimeError(
            "Invalid request data combination".to_string(),
        )),
    }
}

/// Generate a random boundary string for multipart form data
pub fn generate_boundary() -> String {
    use fastrand::Rng;
    let mut rng = Rng::new();
    format!(
        "----FormBoundary{}{:08x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u32,
        rng.u32(..u32::MAX)
    )
}

/// Build multipart form data body
pub fn build_multipart_body(
    boundary: &str,
    fields: &HashMap<String, String>,
    files: &[FilePart],
) -> Vec<u8> {
    let mut body = Vec::new();

    // Add text fields first
    for (name, value) in fields.iter() {
        write_part(&mut body, boundary, name, None, value.as_bytes());
    }

    // Add file fields
    for file in files.iter() {
        let content_type = file
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        write_part(
            &mut body,
            boundary,
            &file.field_name,
            Some((&file.filename, content_type)),
            &file.data,
        );
    }

    // Write closing boundary
    let closing = format!("--{}--\r\n", boundary);
    body.extend_from_slice(closing.as_bytes());

    body
}

/// Write a single part of multipart form data
fn write_part(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename_content_type: Option<(&str, &str)>,
    data: &[u8],
) {
    // Write boundary
    let boundary_line = format!("--{}\r\n", boundary);
    body.extend_from_slice(boundary_line.as_bytes());

    // Write Content-Disposition header
    if let Some((filename, content_type)) = filename_content_type {
        let disp = format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            name, filename
        );
        body.extend_from_slice(disp.as_bytes());

        let ct = format!("Content-Type: {}\r\n", content_type);
        body.extend_from_slice(ct.as_bytes());
    } else {
        let disp = format!("Content-Disposition: form-data; name=\"{}\"\r\n", name);
        body.extend_from_slice(disp.as_bytes());
    }

    // Write blank line and data
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
}
