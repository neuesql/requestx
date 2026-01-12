//! Redirect handling utilities
//!
//! Provides functions for handling HTTP redirects.

use bytes::Bytes;
use hyper::{Body, Client, HeaderMap, Method, Request, Response, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

/// Redirect history item
#[derive(Debug)]
pub struct RedirectHistoryItem {
    pub status_code: u16,
    pub headers: HeaderMap,
    pub url: Uri,
    pub body: Bytes,
}

/// Maximum allowed redirect count to prevent infinite loops
pub const MAX_REDIRECTS: u8 = 30;

/// Determine redirect method per RFC 7231
pub fn get_redirect_method(original_method: &Method, status_code: u16) -> Method {
    match status_code {
        // 307 Temporary Redirect - keep same method
        307 => original_method.clone(),
        // 308 Permanent Redirect - keep same method
        308 => original_method.clone(),
        // 303 See Other - convert to GET (except for HEAD)
        303 if original_method != Method::HEAD => Method::GET,
        // 301, 302 - convert POST to GET (legacy behavior)
        301 | 302 if original_method == Method::POST => Method::GET,
        // All others - keep original method
        _ => original_method.clone(),
    }
}

/// Check if status code is a redirect
#[inline]
pub fn is_redirect(status_code: u16) -> bool {
    matches!(status_code, 301 | 302 | 303 | 307 | 308)
}

/// Build redirect URL from Location header
pub fn build_redirect_url(location: &str, base_url: &Uri) -> Result<Uri, String> {
    if location.starts_with("http") {
        location.parse::<Uri>().map_err(|e| e.to_string())
    } else {
        // Relative URL - resolve against base URL
        let base_scheme = base_url.scheme_str().unwrap_or("https");
        let base_host = base_url.host().unwrap_or("");
        let base_port = base_url
            .port_u16()
            .map(|p| format!(":{p}"))
            .unwrap_or_default();
        format!("{base_scheme}://{base_host}{base_port}{location}")
            .parse::<Uri>()
            .map_err(|e| e.to_string())
    }
}

/// Process redirect response and build redirect request
pub fn process_redirect(
    response: &Response<Body>,
    redirect_url: Uri,
    redirect_method: Method,
) -> Result<Request<Body>, String> {
    Request::builder()
        .method(redirect_method)
        .uri(redirect_url)
        .body(Body::empty())
        .map_err(|e| e.to_string())
}

/// Follow redirects and collect history
pub async fn follow_redirects<F, Fut>(
    initial_request: Request<Body>,
    client: &Client<HttpsConnector<hyper::client::HttpConnector>>,
    should_redirect: impl Fn(StatusCode) -> bool,
    mut make_request: F,
) -> Result<(Response<Body>, Vec<RedirectHistoryItem>), String>
where
    F: FnMut(Request<Body>) -> Fut,
    Fut: Future<Output = Result<Response<Body>, hyper::Error>>,
{
    let mut redirect_count = 0;
    let mut history: Vec<RedirectHistoryItem> = Vec::new();
    let mut current_request = initial_request;
    let mut response: Option<Response<Body>> = None;

    while redirect_count < MAX_REDIRECTS {
        // Get the current request data before making the request
        let current_request_uri = current_request.uri().clone();
        let current_request_method = current_request.method().clone();

        // Make the request
        let resp = make_request(current_request)
            .await
            .map_err(|e| e.to_string())?;

        // Check if we should redirect
        if !should_redirect(resp.status()) {
            response = Some(resp);
            break;
        }

        // Get redirect URL
        let redirect_url = match resp.headers().get("location") {
            Some(location) => {
                let location_str = location.to_str().map_err(|e| e.to_string())?;
                build_redirect_url(location_str, &current_request_uri)?
            }
            None => {
                // No Location header - can't redirect
                response = Some(resp);
                break;
            }
        };

        // Get the redirect method
        let redirect_method = get_redirect_method(&current_request_method, resp.status().as_u16());

        // Capture redirect in history
        let history_item = RedirectHistoryItem {
            status_code: resp.status().as_u16(),
            headers: resp.headers().clone(),
            url: current_request_uri.clone(),
            body: Bytes::new(),
        };
        history.push(history_item);

        // Build redirect request
        current_request = process_redirect(&resp, redirect_url, redirect_method)?;

        redirect_count += 1;
    }

    response
        .ok_or_else(|| "Too many redirects".to_string())
        .map(|resp| (resp, history))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_redirect() {
        assert!(is_redirect(301));
        assert!(is_redirect(302));
        assert!(is_redirect(303));
        assert!(is_redirect(307));
        assert!(is_redirect(308));
        assert!(!is_redirect(200));
        assert!(!is_redirect(404));
        assert!(!is_redirect(500));
    }

    #[test]
    fn test_get_redirect_method_post_to_get() {
        assert_eq!(get_redirect_method(&Method::POST, 301), Method::GET);
        assert_eq!(get_redirect_method(&Method::POST, 302), Method::GET);
        assert_eq!(get_redirect_method(&Method::POST, 303), Method::GET);
        assert_eq!(get_redirect_method(&Method::POST, 307), Method::POST);
        assert_eq!(get_redirect_method(&Method::POST, 308), Method::POST);
    }

    #[test]
    fn test_get_redirect_method_get_unchanged() {
        assert_eq!(get_redirect_method(&Method::GET, 301), Method::GET);
        assert_eq!(get_redirect_method(&Method::GET, 302), Method::GET);
        assert_eq!(get_redirect_method(&Method::GET, 303), Method::GET);
        assert_eq!(get_redirect_method(&Method::GET, 307), Method::GET);
        assert_eq!(get_redirect_method(&Method::GET, 308), Method::GET);
    }

    #[test]
    fn test_get_redirect_method_head_unchanged() {
        assert_eq!(get_redirect_method(&Method::HEAD, 301), Method::HEAD);
        assert_eq!(get_redirect_method(&Method::HEAD, 303), Method::HEAD);
        assert_eq!(get_redirect_method(&Method::HEAD, 307), Method::HEAD);
    }

    #[test]
    fn test_build_redirect_url_absolute() {
        let uri: Uri = "https://example.com/path".parse().unwrap();
        let result = build_redirect_url("https://other.com/new", &uri).unwrap();
        assert_eq!(result, "https://other.com/new".parse::<Uri>().unwrap());
    }

    #[test]
    fn test_build_redirect_url_relative() {
        let uri: Uri = "https://example.com/path".parse().unwrap();
        let result = build_redirect_url("/new", &uri).unwrap();
        assert_eq!(result, "https://example.com/new".parse::<Uri>().unwrap());
    }
}
