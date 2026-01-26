//! Error types for requestx
//!
//! This module provides exception types compatible with HTTPX SDK.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// ============================================================================
// Base Exception Hierarchy (matches HTTPX)
// ============================================================================

// Base exception for all requestx errors
create_exception!(requestx, RequestError, PyException);

// Transport-level errors
create_exception!(requestx, TransportError, RequestError);
create_exception!(requestx, ConnectError, TransportError);
create_exception!(requestx, ReadError, TransportError);
create_exception!(requestx, WriteError, TransportError);
create_exception!(requestx, CloseError, TransportError);
create_exception!(requestx, ProxyError, TransportError);
create_exception!(requestx, UnsupportedProtocol, TransportError);

// Protocol errors
create_exception!(requestx, ProtocolError, TransportError);
create_exception!(requestx, LocalProtocolError, ProtocolError);
create_exception!(requestx, RemoteProtocolError, ProtocolError);

// Timeout errors
create_exception!(requestx, TimeoutException, TransportError);
create_exception!(requestx, ConnectTimeout, TimeoutException);
create_exception!(requestx, ReadTimeout, TimeoutException);
create_exception!(requestx, WriteTimeout, TimeoutException);
create_exception!(requestx, PoolTimeout, TimeoutException);

// HTTP status errors
create_exception!(requestx, HTTPStatusError, RequestError);

// Redirect errors
create_exception!(requestx, TooManyRedirects, RequestError);

// Decoding errors
create_exception!(requestx, DecodingError, RequestError);

// Stream errors
create_exception!(requestx, StreamError, RequestError);
create_exception!(requestx, StreamConsumed, StreamError);
create_exception!(requestx, StreamClosed, StreamError);
create_exception!(requestx, ResponseNotRead, StreamError);
create_exception!(requestx, RequestNotRead, StreamError);

// URL errors
create_exception!(requestx, InvalidURL, RequestError);

// Cookie errors
create_exception!(requestx, CookieConflict, RequestError);

// ============================================================================
// Internal Error Types
// ============================================================================

/// Error kind enumeration
#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Generic
    Request,

    // Transport
    Transport,
    Connect,
    Read,
    Write,
    Close,
    Proxy,
    UnsupportedProtocol,

    // Protocol
    Protocol,
    LocalProtocol,
    RemoteProtocol,

    // Timeout
    Timeout,
    ConnectTimeout,
    ReadTimeout,
    WriteTimeout,
    PoolTimeout,

    // HTTP
    Status(u16),
    Redirect,

    // Data
    Decode,
    InvalidUrl,
    InvalidHeader,

    // Stream
    Stream,
    StreamConsumed,
    StreamClosed,
    ResponseNotRead,
    RequestNotRead,

    // Cookie
    CookieConflict,

    // Other
    Other(String),
}

/// Internal error type
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    // Generic errors
    pub fn request(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Request, message)
    }

    // Transport errors
    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Transport, message)
    }

    pub fn connect(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Connect, message)
    }

    pub fn read(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Read, message)
    }

    pub fn write(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Write, message)
    }

    pub fn close(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Close, message)
    }

    pub fn proxy(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Proxy, message)
    }

    pub fn unsupported_protocol(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::UnsupportedProtocol, message)
    }

    // Protocol errors
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Protocol, message)
    }

    pub fn local_protocol(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::LocalProtocol, message)
    }

    pub fn remote_protocol(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RemoteProtocol, message)
    }

    // Timeout errors
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
    }

    pub fn connect_timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ConnectTimeout, message)
    }

    pub fn read_timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ReadTimeout, message)
    }

    pub fn write_timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::WriteTimeout, message)
    }

    pub fn pool_timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PoolTimeout, message)
    }

    // HTTP errors
    pub fn status(code: u16, message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Status(code), message)
    }

    pub fn redirect(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Redirect, message)
    }

    // Data errors
    pub fn decode(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Decode, message)
    }

    pub fn invalid_url(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidUrl, message)
    }

    pub fn invalid_header(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidHeader, message)
    }

    // Stream errors
    pub fn stream(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Stream, message)
    }

    pub fn stream_consumed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::StreamConsumed, message)
    }

    pub fn stream_closed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::StreamClosed, message)
    }

    pub fn response_not_read(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ResponseNotRead, message)
    }

    pub fn request_not_read(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RequestNotRead, message)
    }

    // Cookie errors
    pub fn cookie_conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::CookieConflict, message)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let err_string = err.to_string();

        if err.is_timeout() {
            // Try to determine specific timeout type from error message
            let lower = err_string.to_lowercase();
            if lower.contains("connect") {
                Error::connect_timeout(err_string)
            } else if lower.contains("read") {
                Error::read_timeout(err_string)
            } else if lower.contains("write") {
                Error::write_timeout(err_string)
            } else if lower.contains("pool") {
                Error::pool_timeout(err_string)
            } else {
                Error::timeout(err_string)
            }
        } else if err.is_connect() {
            Error::connect(err_string)
        } else if err.is_redirect() {
            Error::redirect(err_string)
        } else if err.is_decode() {
            Error::decode(err_string)
        } else if err.is_request() {
            // Check for specific request errors
            let lower = err_string.to_lowercase();
            if lower.contains("proxy") {
                Error::proxy(err_string)
            } else if lower.contains("protocol") || lower.contains("unsupported") {
                Error::unsupported_protocol(err_string)
            } else {
                Error::request(err_string)
            }
        } else if let Some(status) = err.status() {
            Error::status(status.as_u16(), err_string)
        } else {
            Error::request(err_string)
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::invalid_url(err.to_string())
    }
}

impl From<sonic_rs::Error> for Error {
    fn from(err: sonic_rs::Error) -> Self {
        Error::decode(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind as IoErrorKind;
        match err.kind() {
            IoErrorKind::TimedOut => Error::timeout(err.to_string()),
            IoErrorKind::ConnectionRefused
            | IoErrorKind::ConnectionReset
            | IoErrorKind::ConnectionAborted
            | IoErrorKind::NotConnected => Error::connect(err.to_string()),
            IoErrorKind::BrokenPipe | IoErrorKind::WriteZero => Error::write(err.to_string()),
            IoErrorKind::UnexpectedEof => Error::read(err.to_string()),
            _ => Error::transport(err.to_string()),
        }
    }
}

impl From<Error> for PyErr {
    fn from(err: Error) -> Self {
        match err.kind {
            // Transport errors
            ErrorKind::Transport => TransportError::new_err(err.message),
            ErrorKind::Connect => ConnectError::new_err(err.message),
            ErrorKind::Read => ReadError::new_err(err.message),
            ErrorKind::Write => WriteError::new_err(err.message),
            ErrorKind::Close => CloseError::new_err(err.message),
            ErrorKind::Proxy => ProxyError::new_err(err.message),
            ErrorKind::UnsupportedProtocol => UnsupportedProtocol::new_err(err.message),

            // Protocol errors
            ErrorKind::Protocol => ProtocolError::new_err(err.message),
            ErrorKind::LocalProtocol => LocalProtocolError::new_err(err.message),
            ErrorKind::RemoteProtocol => RemoteProtocolError::new_err(err.message),

            // Timeout errors
            ErrorKind::Timeout => TimeoutException::new_err(err.message),
            ErrorKind::ConnectTimeout => ConnectTimeout::new_err(err.message),
            ErrorKind::ReadTimeout => ReadTimeout::new_err(err.message),
            ErrorKind::WriteTimeout => WriteTimeout::new_err(err.message),
            ErrorKind::PoolTimeout => PoolTimeout::new_err(err.message),

            // HTTP errors
            ErrorKind::Status(code) => {
                HTTPStatusError::new_err(format!("{} (status code: {})", err.message, code))
            }
            ErrorKind::Redirect => TooManyRedirects::new_err(err.message),

            // Data errors
            ErrorKind::Decode => DecodingError::new_err(err.message),
            ErrorKind::InvalidUrl => InvalidURL::new_err(err.message),
            ErrorKind::InvalidHeader => RequestError::new_err(err.message),

            // Stream errors
            ErrorKind::Stream => StreamError::new_err(err.message),
            ErrorKind::StreamConsumed => StreamConsumed::new_err(err.message),
            ErrorKind::StreamClosed => StreamClosed::new_err(err.message),
            ErrorKind::ResponseNotRead => ResponseNotRead::new_err(err.message),
            ErrorKind::RequestNotRead => RequestNotRead::new_err(err.message),

            // Cookie errors
            ErrorKind::CookieConflict => CookieConflict::new_err(err.message),

            // Generic
            ErrorKind::Request | ErrorKind::Other(_) => RequestError::new_err(err.message),
        }
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;
