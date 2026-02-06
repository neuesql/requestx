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

// Cookie exceptions
create_exception!(requestx, CookieConflict, PyException);

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
    m.add("CookieConflict", m.py().get_type::<CookieConflict>())?;
    Ok(())
}

/// Convert reqwest error to appropriate Python exception
pub fn convert_reqwest_error(e: reqwest::Error) -> PyErr {
    convert_reqwest_error_with_context(e, None)
}

/// Convert reqwest error with optional timeout context
/// The timeout_context indicates which specific timeout was configured:
/// - "connect" if only connect timeout was set
/// - "write" if only write timeout was set
/// - "read" if only read timeout was set
/// - "pool" if only pool timeout was set
/// - None for general timeouts or when all are set
pub fn convert_reqwest_error_with_context(e: reqwest::Error, timeout_context: Option<&str>) -> PyErr {
    let error_str = format!("{}", e);
    let lower_error = error_str.to_lowercase();

    // Check for unsupported protocol/scheme errors
    if e.is_builder() {
        // Builder errors often indicate URL scheme issues
        if lower_error.contains("url") || lower_error.contains("scheme") || lower_error.contains("builder error") {
            // Check if it's a scheme/protocol issue by looking at the URL
            if let Some(url) = e.url() {
                let scheme = url.scheme();
                if scheme != "http" && scheme != "https" {
                    return UnsupportedProtocol::new_err(format!("Request URL has unsupported protocol '{}://': {}", scheme, url));
                }
            }
            // Generic unsupported protocol for builder URL errors
            return UnsupportedProtocol::new_err(error_str);
        }
    }

    if e.is_timeout() {
        // If we have context about which timeout was specifically set, use that
        if let Some(ctx) = timeout_context {
            return match ctx {
                "connect" => ConnectTimeout::new_err(error_str),
                "write" => WriteTimeout::new_err(error_str),
                "read" => ReadTimeout::new_err(error_str),
                "pool" => PoolTimeout::new_err(error_str),
                _ => TimeoutException::new_err(error_str),
            };
        }

        // Determine timeout type based on reqwest's error flags
        // reqwest distinguishes connect timeouts reliably via is_connect()
        if e.is_connect() {
            return ConnectTimeout::new_err(error_str);
        }

        // Check error message for connect-related indicators
        // Non-routable IPs and DNS failures indicate connect timeout
        if lower_error.contains("connect") || lower_error.contains("dns") || lower_error.contains("resolve") || lower_error.contains("10.255.255") || lower_error.contains("connection refused") {
            return ConnectTimeout::new_err(error_str);
        }

        // Check for pool-related indicators
        if lower_error.contains("pool") || lower_error.contains("acquire connection") {
            return PoolTimeout::new_err(error_str);
        }

        // Check for write-related indicators
        // "sending request" or "request body" indicates write phase
        if lower_error.contains("sending request") || lower_error.contains("request body") || lower_error.contains("send body") {
            // Only classify as WriteTimeout if we're sure it's during write
            // Check if it's body-related but not response-related
            if !lower_error.contains("response") && !lower_error.contains("decoding") {
                return WriteTimeout::new_err(error_str);
            }
        }

        // Check for read-related indicators
        if lower_error.contains("response body") || lower_error.contains("decoding") || lower_error.contains("receiving") {
            return ReadTimeout::new_err(error_str);
        }

        // Default to read timeout for other timeout errors
        ReadTimeout::new_err(error_str)
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
