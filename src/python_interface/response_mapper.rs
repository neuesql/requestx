//! Response data to Python Response mapping
//!
//! Provides conversion from Rust ResponseData to Python Response objects.

use pyo3::prelude::*;

use super::super::core::http_client::ResponseData;
use super::super::response::Response;

/// Convert ResponseData to Python Response object
pub fn response_data_to_py_response(response_data: ResponseData) -> PyResult<Response> {
    let headers = response_data
        .headers
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    // Check if we have pre-chunked body (from streaming response)
    let body_vec = response_data.body.to_vec();
    if let Some(chunks) = response_data.body_chunks {
        let mut response = Response::new_with_chunks(
            response_data.status_code,
            response_data.url.to_string(),
            headers,
            body_vec,
            chunks,
            response_data.is_stream,
            response_data.elapsed_us,
        );

        // Convert history ResponseData items to Response objects
        let history: Vec<Response> = response_data
            .history
            .into_iter()
            .map(|history_data| {
                let history_headers = history_data
                    .headers
                    .iter()
                    .map(|(name, value)| {
                        (name.to_string(), value.to_str().unwrap_or("").to_string())
                    })
                    .collect();

                let history_body = history_data.body.to_vec();
                if let Some(history_chunks) = history_data.body_chunks {
                    Response::new_with_chunks(
                        history_data.status_code,
                        history_data.url.to_string(),
                        history_headers,
                        history_body,
                        history_chunks,
                        history_data.is_stream,
                        history_data.elapsed_us,
                    )
                } else {
                    Response::new(
                        history_data.status_code,
                        history_data.url.to_string(),
                        history_headers,
                        history_body,
                        history_data.is_stream,
                        history_data.elapsed_us,
                    )
                }
            })
            .collect();

        response.history = history;
        return Ok(response);
    }

    let mut response = Response::new(
        response_data.status_code,
        response_data.url.to_string(),
        headers,
        body_vec,
        response_data.is_stream,
        response_data.elapsed_us,
    );

    // Convert history ResponseData items to Response objects
    let history: Vec<Response> = response_data
        .history
        .into_iter()
        .map(|history_data| {
            let history_headers = history_data
                .headers
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
                .collect();

            Response::new(
                history_data.status_code,
                history_data.url.to_string(),
                history_headers,
                history_data.body.to_vec(),
                history_data.is_stream,
                history_data.elapsed_us,
            )
        })
        .collect();

    response.history = history;

    Ok(response)
}
