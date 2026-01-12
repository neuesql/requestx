//! HTTP client implementation
//!
//! This module provides the core HTTP client functionality using hyper.

pub mod auth;
pub mod body;
pub mod redirect;

pub use auth::{add_auth_header, add_query_params, build_basic_auth_header};
pub use body::{build_body, build_multipart_body, generate_boundary, BuiltBody};
pub use redirect::{
    build_redirect_url, follow_redirects, get_redirect_method, is_redirect, process_redirect,
    RedirectHistoryItem, MAX_REDIRECTS,
};
