//! Requestx - High-performance Python HTTP client based on reqwest
//!
//! This library provides Python bindings for the reqwest HTTP client,
//! exposing an API compatible with HTTPX.

mod client;
mod error;
mod request;
mod response;
mod streaming;
mod types;

use pyo3::prelude::*;

/// Python module initialization
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<client::Client>()?;
    m.add_class::<client::AsyncClient>()?;
    m.add_class::<response::Response>()?;
    m.add_class::<types::Headers>()?;
    m.add_class::<types::Cookies>()?;
    m.add_class::<types::CookiesIterator>()?;
    m.add_class::<types::Timeout>()?;
    m.add_class::<types::Proxy>()?;
    m.add_class::<types::Auth>()?;
    m.add_class::<types::Limits>()?;
    m.add_class::<types::SSLConfig>()?;
    m.add_class::<types::URL>()?;
    m.add_class::<types::Request>()?;
    m.add_class::<types::QueryParams>()?;
    m.add_class::<types::QueryParamsIterator>()?;

    // Streaming response types
    m.add_class::<streaming::StreamingResponse>()?;
    m.add_class::<streaming::AsyncStreamingResponse>()?;
    m.add_class::<streaming::BytesIterator>()?;
    m.add_class::<streaming::TextIterator>()?;
    m.add_class::<streaming::LinesIterator>()?;
    m.add_class::<streaming::AsyncBytesIterator>()?;
    m.add_class::<streaming::AsyncTextIterator>()?;
    m.add_class::<streaming::AsyncLinesIterator>()?;

    // Register exception types - Base
    m.add("RequestError", m.py().get_type::<error::RequestError>())?;

    // Transport errors
    m.add("TransportError", m.py().get_type::<error::TransportError>())?;
    m.add("ConnectError", m.py().get_type::<error::ConnectError>())?;
    m.add("ReadError", m.py().get_type::<error::ReadError>())?;
    m.add("WriteError", m.py().get_type::<error::WriteError>())?;
    m.add("CloseError", m.py().get_type::<error::CloseError>())?;
    m.add("ProxyError", m.py().get_type::<error::ProxyError>())?;
    m.add("UnsupportedProtocol", m.py().get_type::<error::UnsupportedProtocol>())?;

    // Protocol errors
    m.add("ProtocolError", m.py().get_type::<error::ProtocolError>())?;
    m.add("LocalProtocolError", m.py().get_type::<error::LocalProtocolError>())?;
    m.add("RemoteProtocolError", m.py().get_type::<error::RemoteProtocolError>())?;

    // Timeout errors
    m.add("TimeoutException", m.py().get_type::<error::TimeoutException>())?;
    m.add("ConnectTimeout", m.py().get_type::<error::ConnectTimeout>())?;
    m.add("ReadTimeout", m.py().get_type::<error::ReadTimeout>())?;
    m.add("WriteTimeout", m.py().get_type::<error::WriteTimeout>())?;
    m.add("PoolTimeout", m.py().get_type::<error::PoolTimeout>())?;

    // HTTP status errors
    m.add("HTTPStatusError", m.py().get_type::<error::HTTPStatusError>())?;

    // Redirect errors
    m.add("TooManyRedirects", m.py().get_type::<error::TooManyRedirects>())?;

    // Decoding errors
    m.add("DecodingError", m.py().get_type::<error::DecodingError>())?;

    // Stream errors
    m.add("StreamError", m.py().get_type::<error::StreamError>())?;
    m.add("StreamConsumed", m.py().get_type::<error::StreamConsumed>())?;
    m.add("StreamClosed", m.py().get_type::<error::StreamClosed>())?;
    m.add("ResponseNotRead", m.py().get_type::<error::ResponseNotRead>())?;
    m.add("RequestNotRead", m.py().get_type::<error::RequestNotRead>())?;

    // URL errors
    m.add("InvalidURL", m.py().get_type::<error::InvalidURL>())?;

    // Cookie errors
    m.add("CookieConflict", m.py().get_type::<error::CookieConflict>())?;

    // Module-level convenience functions (sync)
    m.add_function(wrap_pyfunction!(request::request, m)?)?;
    m.add_function(wrap_pyfunction!(request::get, m)?)?;
    m.add_function(wrap_pyfunction!(request::post, m)?)?;
    m.add_function(wrap_pyfunction!(request::put, m)?)?;
    m.add_function(wrap_pyfunction!(request::patch, m)?)?;
    m.add_function(wrap_pyfunction!(request::delete, m)?)?;
    m.add_function(wrap_pyfunction!(request::head, m)?)?;
    m.add_function(wrap_pyfunction!(request::options, m)?)?;

    Ok(())
}
