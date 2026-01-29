//! Exception hierarchy matching httpx

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// Base exceptions
create_exception!(requestx, HTTPStatusError, PyException);
create_exception!(requestx, RequestError, PyException);
create_exception!(requestx, TransportError, RequestError);
create_exception!(requestx, TimeoutException, TransportError);
create_exception!(requestx, ConnectTimeout, TimeoutException);
create_exception!(requestx, ReadTimeout, TimeoutException);
create_exception!(requestx, WriteTimeout, TimeoutException);
create_exception!(requestx, PoolTimeout, TimeoutException);
create_exception!(requestx, NetworkError, TransportError);
create_exception!(requestx, ConnectError, NetworkError);
create_exception!(requestx, ReadError, NetworkError);
create_exception!(requestx, WriteError, NetworkError);
create_exception!(requestx, CloseError, NetworkError);
create_exception!(requestx, ProxyError, TransportError);
create_exception!(requestx, ProtocolError, TransportError);
create_exception!(requestx, LocalProtocolError, ProtocolError);
create_exception!(requestx, RemoteProtocolError, ProtocolError);
create_exception!(requestx, UnsupportedProtocol, TransportError);
create_exception!(requestx, DecodingError, RequestError);
create_exception!(requestx, TooManyRedirects, RequestError);
create_exception!(requestx, StreamError, RequestError);
create_exception!(requestx, StreamConsumed, StreamError);
create_exception!(requestx, StreamClosed, StreamError);
create_exception!(requestx, ResponseNotRead, StreamError);
create_exception!(requestx, RequestNotRead, StreamError);

// URL exceptions
create_exception!(requestx, InvalidURL, PyException);

// HTTP error (alias)
create_exception!(requestx, HTTPError, PyException);

/// Register all exceptions with the module
pub fn register_exceptions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("HTTPStatusError", m.py().get_type::<HTTPStatusError>())?;
    m.add("RequestError", m.py().get_type::<RequestError>())?;
    m.add("TransportError", m.py().get_type::<TransportError>())?;
    m.add("TimeoutException", m.py().get_type::<TimeoutException>())?;
    m.add("ConnectTimeout", m.py().get_type::<ConnectTimeout>())?;
    m.add("ReadTimeout", m.py().get_type::<ReadTimeout>())?;
    m.add("WriteTimeout", m.py().get_type::<WriteTimeout>())?;
    m.add("PoolTimeout", m.py().get_type::<PoolTimeout>())?;
    m.add("NetworkError", m.py().get_type::<NetworkError>())?;
    m.add("ConnectError", m.py().get_type::<ConnectError>())?;
    m.add("ReadError", m.py().get_type::<ReadError>())?;
    m.add("WriteError", m.py().get_type::<WriteError>())?;
    m.add("CloseError", m.py().get_type::<CloseError>())?;
    m.add("ProxyError", m.py().get_type::<ProxyError>())?;
    m.add("ProtocolError", m.py().get_type::<ProtocolError>())?;
    m.add("LocalProtocolError", m.py().get_type::<LocalProtocolError>())?;
    m.add("RemoteProtocolError", m.py().get_type::<RemoteProtocolError>())?;
    m.add("UnsupportedProtocol", m.py().get_type::<UnsupportedProtocol>())?;
    m.add("DecodingError", m.py().get_type::<DecodingError>())?;
    m.add("TooManyRedirects", m.py().get_type::<TooManyRedirects>())?;
    m.add("StreamError", m.py().get_type::<StreamError>())?;
    m.add("StreamConsumed", m.py().get_type::<StreamConsumed>())?;
    m.add("StreamClosed", m.py().get_type::<StreamClosed>())?;
    m.add("ResponseNotRead", m.py().get_type::<ResponseNotRead>())?;
    m.add("RequestNotRead", m.py().get_type::<RequestNotRead>())?;
    m.add("InvalidURL", m.py().get_type::<InvalidURL>())?;
    m.add("HTTPError", m.py().get_type::<HTTPError>())?;
    Ok(())
}

/// Convert reqwest error to appropriate Python exception
pub fn convert_reqwest_error(e: reqwest::Error) -> PyErr {
    let error_str = format!("{}", e);
    let error_str_lower = error_str.to_lowercase();

    // Check for URL-related errors first
    if e.is_builder() {
        // Check for unsupported scheme
        if error_str_lower.contains("invalid://")
            || error_str_lower.contains("unsupported url scheme")
            || (error_str_lower.contains("builder error") && error_str_lower.contains("url"))
        {
            return UnsupportedProtocol::new_err(error_str);
        }
        // Other builder errors are likely URL parsing issues
        return LocalProtocolError::new_err(error_str);
    }

    if e.is_timeout() {
        if e.is_connect() {
            ConnectTimeout::new_err(error_str)
        } else {
            ReadTimeout::new_err(error_str)
        }
    } else if e.is_connect() {
        ConnectError::new_err(error_str)
    } else if e.is_request() {
        RequestError::new_err(error_str)
    } else if e.is_redirect() {
        TooManyRedirects::new_err(error_str)
    } else {
        TransportError::new_err(error_str)
    }
}
