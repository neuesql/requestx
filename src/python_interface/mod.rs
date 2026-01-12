//! Python interface module
//!
//! Provides the bridge between Rust and Python, including kwargs parsing,
//! request building, and response mapping.

pub mod kwargs;

pub use kwargs::{parse_and_validate_url, parse_kwargs, RequestConfigBuilder};

pub mod response_mapper;
pub use response_mapper::response_data_to_py_response;
