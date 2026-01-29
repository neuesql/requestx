//! RequestX - High-performance Python HTTP client
//!
//! API-compatible with httpx, powered by Rust's reqwest via PyO3.

use pyo3::prelude::*;

mod api;
mod async_client;
mod auth;
mod client;
mod cookies;
mod exceptions;
mod headers;
mod multipart;
mod queryparams;
mod request;
mod response;
mod timeout;
mod transport;
mod types;
mod url;

use async_client::AsyncClient;
use auth::{Auth, FunctionAuth};
use client::Client;
use cookies::Cookies;
use exceptions::*;
use headers::Headers;
use queryparams::QueryParams;
use request::Request;
use response::{
    Response, BytesIterator, TextIterator, LinesIterator, RawIterator,
    AsyncRawIterator, AsyncBytesIterator, AsyncTextIterator, AsyncLinesIterator,
};
use timeout::{Limits, Timeout};
use transport::{AsyncHTTPTransport, AsyncMockTransport, HTTPTransport, MockTransport, WSGITransport};
use types::*;
use url::URL;

/// RequestX Python module
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__title__", "requestx")?;
    m.add("__description__", "High-performance Python HTTP client")?;

    // Core types
    m.add_class::<URL>()?;
    m.add_class::<Headers>()?;
    m.add_class::<QueryParams>()?;
    m.add_class::<Cookies>()?;
    m.add_class::<Request>()?;
    m.add_class::<Response>()?;
    m.add_class::<Client>()?;
    m.add_class::<AsyncClient>()?;
    m.add_class::<Timeout>()?;
    m.add_class::<Limits>()?;

    // Stream types
    m.add_class::<SyncByteStream>()?;
    m.add_class::<AsyncByteStream>()?;

    // Iterator types
    m.add_class::<BytesIterator>()?;
    m.add_class::<TextIterator>()?;
    m.add_class::<LinesIterator>()?;
    m.add_class::<RawIterator>()?;
    m.add_class::<AsyncRawIterator>()?;
    m.add_class::<AsyncBytesIterator>()?;
    m.add_class::<AsyncTextIterator>()?;
    m.add_class::<AsyncLinesIterator>()?;

    // Auth types
    m.add_class::<BasicAuth>()?;
    m.add_class::<DigestAuth>()?;
    m.add_class::<NetRCAuth>()?;
    m.add_class::<Auth>()?;
    m.add_class::<FunctionAuth>()?;

    // Transport types
    m.add_class::<MockTransport>()?;
    m.add_class::<AsyncMockTransport>()?;
    m.add_class::<HTTPTransport>()?;
    m.add_class::<AsyncHTTPTransport>()?;
    m.add_class::<WSGITransport>()?;

    // Top-level functions
    m.add_function(wrap_pyfunction!(api::get, m)?)?;
    m.add_function(wrap_pyfunction!(api::post, m)?)?;
    m.add_function(wrap_pyfunction!(api::put, m)?)?;
    m.add_function(wrap_pyfunction!(api::patch, m)?)?;
    m.add_function(wrap_pyfunction!(api::delete, m)?)?;
    m.add_function(wrap_pyfunction!(api::head, m)?)?;
    m.add_function(wrap_pyfunction!(api::options, m)?)?;
    m.add_function(wrap_pyfunction!(api::request, m)?)?;
    m.add_function(wrap_pyfunction!(api::stream, m)?)?;

    // Exceptions
    register_exceptions(m)?;

    // Status code constants
    m.add_class::<codes>()?;

    Ok(())
}
