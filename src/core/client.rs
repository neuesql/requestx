use base64::prelude::*;
use bytes::Bytes;
use hyper::{Body, Client, HeaderMap, Method, Request, Uri};
use hyper_tls::HttpsConnector;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::error::RequestxError;

// Pre-allocated common strings to reduce allocations
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded";

// Global shared client for connection pooling
static GLOBAL_CLIENT: OnceLock<Client<HttpsConnector<hyper::client::HttpConnector>>> =
    OnceLock::new();

fn get_global_client() -> &'static Client<HttpsConnector<hyper::client::HttpConnector>> {
    GLOBAL_CLIENT.get_or_init(|| {
        let https = HttpsConnector::new();
        Client::builder()
            .pool_idle_timeout(Duration::from_secs(90)) // Longer idle timeout for better reuse
            .pool_max_idle_per_host(50) // More connections per host
            .http2_only(false) // Allow HTTP/1.1 fallback
            .http2_initial_stream_window_size(Some(65536)) // Optimize HTTP/2 streams
            .http2_initial_connection_window_size(Some(1048576)) // 1MB connection window
            .build::<_, hyper::Body>(https)
    })
}

/// Create a custom client with specific SSL verification settings
fn create_custom_client(
    verify: bool,
) -> Result<Client<HttpsConnector<hyper::client::HttpConnector>>, RequestxError> {
    if verify {
        // For verify=true, just use the default HTTPS connector
        let https = HttpsConnector::new();
        return Ok(Client::builder()
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(50)
            .http2_only(false)
            .http2_initial_stream_window_size(Some(65536))
            .http2_initial_connection_window_size(Some(1048576))
            .build::<_, hyper::Body>(https));
    }

    // For verify=false, create a custom TLS connector that accepts invalid certs
    let mut https_builder = hyper_tls::native_tls::TlsConnector::builder();
    https_builder.danger_accept_invalid_certs(true);
    https_builder.danger_accept_invalid_hostnames(true);

    let tls_connector = https_builder
        .build()
        .map_err(|e| RequestxError::SslError(format!("Failed to create TLS connector: {}", e)))?;

    let mut http_connector = hyper::client::HttpConnector::new();
    http_connector.enforce_http(false);

    let https_connector = HttpsConnector::from((http_connector, tls_connector.into()));

    Ok(Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(50)
        .http2_only(false)
        .http2_initial_stream_window_size(Some(65536))
        .http2_initial_connection_window_size(Some(1048576))
        .build::<_, hyper::Body>(https_connector))
}

/// Request configuration for HTTP requests
#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub method: Method,
    pub url: Uri,
    pub headers: Option<HeaderMap>,
    pub params: Option<HashMap<String, String>>,
    pub data: Option<RequestData>,
    pub json: Option<Value>,
    pub timeout: Option<Duration>,
    pub allow_redirects: bool,
    pub verify: bool,
    #[allow(dead_code)]
    pub cert: Option<String>,
    #[allow(dead_code)]
    pub proxies: Option<HashMap<String, String>>,
    pub auth: Option<(String, String)>,
    #[allow(dead_code)]
    pub stream: bool,
}

/// Request data types
#[derive(Debug, Clone)]
pub enum RequestData {
    Text(String),
    Bytes(Vec<u8>),
    Form(HashMap<String, String>),
}

/// Response data from HTTP requests
#[derive(Debug)]
pub struct ResponseData {
    pub status_code: u16,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub url: Uri,
}

/// Core HTTP client using hyper
pub struct RequestxClient {
    // Use reference to global client for better performance
    use_global_client: bool,
    custom_client: Option<Client<HttpsConnector<hyper::client::HttpConnector>>>,
    #[allow(dead_code)]
    custom_runtime: Option<Arc<Runtime>>,
}

#[allow(dead_code)]
impl RequestxClient {
    // Constants for default values to reduce allocations
    #[allow(dead_code)]
    const DEFAULT_ALLOW_REDIRECTS: bool = true;
    #[allow(dead_code)]
    const DEFAULT_VERIFY: bool = true;

    /// Create a new RequestxClient using global shared resources
    pub fn new() -> Result<Self, RequestxError> {
        Ok(RequestxClient {
            use_global_client: true,
            custom_client: None,
            custom_runtime: None,
        })
    }

    /// Create a new RequestxClient with custom runtime
    #[allow(dead_code)]
    pub fn with_runtime(runtime: Runtime) -> Result<Self, RequestxError> {
        Ok(RequestxClient {
            use_global_client: true,
            custom_client: None,
            custom_runtime: Some(Arc::new(runtime)),
        })
    }

    /// Create a new RequestxClient with custom client configuration
    pub fn with_custom_client(
        client: Client<HttpsConnector<hyper::client::HttpConnector>>,
    ) -> Result<Self, RequestxError> {
        Ok(RequestxClient {
            use_global_client: false,
            custom_client: Some(client),
            custom_runtime: None,
        })
    }

    /// Get the HTTP client to use (global or custom)
    #[allow(dead_code)]
    fn get_client(&self) -> &Client<HttpsConnector<hyper::client::HttpConnector>> {
        if self.use_global_client {
            get_global_client()
        } else {
            self.custom_client.as_ref().unwrap()
        }
    }

    /// Get the runtime to use (global or custom)
    #[allow(dead_code)]
    fn get_runtime(&self) -> &Runtime {
        if let Some(ref custom_runtime) = self.custom_runtime {
            custom_runtime
        } else {
            crate::core::runtime::get_global_runtime_manager().get_runtime()
        }
    }

    /// Create a default RequestConfig for a given method and URL
    #[allow(dead_code)]
    fn create_default_config(&self, method: Method, url: Uri) -> RequestConfig {
        RequestConfig {
            method,
            url,
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: Self::DEFAULT_ALLOW_REDIRECTS,
            verify: Self::DEFAULT_VERIFY,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        }
    }

    /// Perform an async HTTP GET request
    pub async fn get_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config = config.unwrap_or_else(|| self.create_default_config(Method::GET, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP POST request
    pub async fn post_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config =
            config.unwrap_or_else(|| self.create_default_config(Method::POST, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP PUT request
    pub async fn put_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config = config.unwrap_or_else(|| self.create_default_config(Method::PUT, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP DELETE request
    pub async fn delete_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config =
            config.unwrap_or_else(|| self.create_default_config(Method::DELETE, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP HEAD request
    pub async fn head_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config =
            config.unwrap_or_else(|| self.create_default_config(Method::HEAD, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP OPTIONS request
    pub async fn options_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config =
            config.unwrap_or_else(|| self.create_default_config(Method::OPTIONS, url));
        self.request_async(request_config).await
    }

    /// Perform an async HTTP PATCH request
    pub async fn patch_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let request_config =
            config.unwrap_or_else(|| self.create_default_config(Method::PATCH, url));
        self.request_async(request_config).await
    }

    /// Perform a generic async HTTP request
    pub async fn request_async(
        &self,
        config: RequestConfig,
    ) -> Result<ResponseData, RequestxError> {
        let client = self.get_client().clone();
        Self::execute_request_async(client, config).await
    }

    /// Perform a synchronous HTTP request by spawning on async runtime
    pub fn request_sync(&self, config: RequestConfig) -> Result<ResponseData, RequestxError> {
        // Use the appropriate runtime (custom or global)
        let runtime = self.get_runtime();

        // Clone necessary data for the spawned task
        let client = self.get_client().clone();

        // Spawn the async task with cloned client
        let handle =
            runtime.spawn(async move { Self::execute_request_async(client, config).await });

        // Block on the spawned task handle instead of the runtime directly
        runtime
            .block_on(handle)
            .map_err(|e| RequestxError::RuntimeError(format!("Task execution failed: {}", e)))?
    }

    /// Static method to execute async request with a given client
    async fn execute_request_async(
        client: Client<HttpsConnector<hyper::client::HttpConnector>>,
        config: RequestConfig,
    ) -> Result<ResponseData, RequestxError> {
        // Use custom client only if SSL verification is disabled
        let actual_client = if !config.verify {
            create_custom_client(false)?
        } else {
            client
        };
        // Build URL with query parameters if provided
        let final_url = if let Some(ref params) = config.params {
            let mut url_with_params = config.url.to_string();
            if !params.is_empty() {
                let query_string = params
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");

                if config.url.query().is_some() {
                    url_with_params.push('&');
                } else {
                    url_with_params.push('?');
                }
                url_with_params.push_str(&query_string);
            }
            url_with_params
                .parse::<Uri>()
                .map_err(RequestxError::InvalidUrl)?
        } else {
            config.url.clone()
        };

        // Build the request more efficiently
        let mut request_builder = Request::builder()
            .method(&config.method) // Use reference instead of clone
            .uri(&final_url); // Use the final URL with params

        // Add headers efficiently
        if let Some(ref headers) = config.headers {
            for (name, value) in headers.iter() {
                request_builder = request_builder.header(name, value);
            }
        }

        // Add authentication header if provided
        if let Some(ref auth) = config.auth {
            let credentials = format!("{}:{}", auth.0, auth.1);
            let encoded = BASE64_STANDARD.encode(credentials.as_bytes());
            request_builder = request_builder.header("authorization", format!("Basic {}", encoded));
        }

        // Build request body more efficiently
        let body = match (&config.data, &config.json) {
            (Some(RequestData::Text(text)), None) => {
                Body::from(text.clone()) // Need to clone for lifetime
            }
            (Some(RequestData::Bytes(bytes)), None) => {
                Body::from(bytes.clone()) // Need to clone for lifetime
            }
            (Some(RequestData::Form(form)), None) => {
                // More efficient form encoding with pre-allocated capacity
                let estimated_size = form
                    .iter()
                    .map(|(k, v)| k.len() + v.len() + 10) // +10 for encoding overhead
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

                request_builder = request_builder.header("content-type", CONTENT_TYPE_FORM);
                Body::from(form_data)
            }
            (None, Some(json)) => {
                let json_string = serde_json::to_string(json)?;
                request_builder = request_builder.header("content-type", CONTENT_TYPE_JSON);
                Body::from(json_string)
            }
            (None, None) => Body::empty(),
            (Some(_), Some(_)) => {
                return Err(RequestxError::RuntimeError(
                    "Cannot specify both data and json parameters".to_string(),
                ));
            }
        };

        let request = request_builder
            .body(body)
            .map_err(|e| RequestxError::RuntimeError(format!("Failed to build request: {}", e)))?;

        // Execute the request with optional timeout
        let response = if let Some(timeout) = config.timeout {
            match tokio::time::timeout(timeout, actual_client.request(request)).await {
                Ok(result) => result,
                Err(_) => return Err(RequestxError::ReadTimeout), // Timeout elapsed
            }
        } else {
            actual_client.request(request).await
        };

        // Handle specific hyper errors and convert them to appropriate RequestxError types
        let mut response = match response {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();

                // Map specific hyper errors to appropriate RequestxError types
                if error_msg.contains("dns") || error_msg.contains("resolve") {
                    return Err(RequestxError::NetworkError(e));
                } else if error_msg.contains("connect") || error_msg.contains("connection refused")
                {
                    return Err(RequestxError::NetworkError(e));
                } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    return Err(RequestxError::ConnectTimeout);
                } else if error_msg.contains("ssl")
                    || error_msg.contains("tls")
                    || error_msg.contains("certificate")
                {
                    return Err(RequestxError::SslError(error_msg));
                } else if error_msg.contains("absolute-form uris")
                    || error_msg.contains("invalid uri")
                {
                    return Err(RequestxError::RuntimeError(format!(
                        "Invalid URL: {}",
                        error_msg
                    )));
                } else if error_msg.contains("proxy") {
                    return Err(RequestxError::ProxyError(error_msg));
                } else {
                    return Err(RequestxError::NetworkError(e));
                }
            }
        };

        // Handle redirects
        if config.allow_redirects && response.status().is_redirection() {
            // Follow redirects (up to 10 redirects max)
            let mut redirect_count = 0;
            const MAX_REDIRECTS: u8 = 10;

            while response.status().is_redirection() && redirect_count < MAX_REDIRECTS {
                if let Some(location) = response.headers().get("location") {
                    let location_str = location.to_str().map_err(|_| {
                        RequestxError::RuntimeError("Invalid redirect location header".to_string())
                    })?;

                    // Parse the redirect URL
                    let redirect_url: Uri = if location_str.starts_with("http") {
                        // Absolute URL
                        location_str.parse().map_err(|e| {
                            RequestxError::RuntimeError(format!("Invalid redirect URL: {}", e))
                        })?
                    } else {
                        // Relative URL - construct absolute URL
                        let base_scheme = final_url.scheme_str().unwrap_or("https");
                        let base_host = final_url.host().unwrap_or("");
                        let base_port = if let Some(port) = final_url.port_u16() {
                            format!(":{}", port)
                        } else {
                            String::new()
                        };

                        format!(
                            "{}://{}{}{}",
                            base_scheme, base_host, base_port, location_str
                        )
                        .parse()
                        .map_err(|e| {
                            RequestxError::RuntimeError(format!("Invalid redirect URL: {}", e))
                        })?
                    };

                    // Create new request for redirect
                    let redirect_request = Request::builder()
                        .method(Method::GET) // Redirects typically use GET
                        .uri(&redirect_url)
                        .body(Body::empty())
                        .map_err(|e| {
                            RequestxError::RuntimeError(format!(
                                "Failed to build redirect request: {}",
                                e
                            ))
                        })?;

                    // Execute redirect request
                    response = actual_client.request(redirect_request).await?;
                    redirect_count += 1;
                } else {
                    // No location header, break out of redirect loop
                    break;
                }
            }

            if redirect_count >= MAX_REDIRECTS {
                return Err(RequestxError::TooManyRedirects);
            }
        }

        // Extract response data efficiently
        let status_code = response.status().as_u16();
        let headers = response.headers().clone(); // This clone is necessary
        let url = config.url; // Move instead of clone

        // Read response body
        let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(ResponseData {
            status_code,
            headers,
            body: body_bytes,
            url,
        })
    }
}

impl Default for RequestxClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default RequestxClient")
    }
}

impl Clone for RequestxClient {
    fn clone(&self) -> Self {
        if self.use_global_client {
            // For global client usage, just create a new instance
            RequestxClient::new().expect("Failed to clone RequestxClient")
        } else if let Some(ref custom_client) = self.custom_client {
            // For custom client, create a new instance with the same client
            RequestxClient::with_custom_client(custom_client.clone())
                .expect("Failed to clone RequestxClient with custom client")
        } else {
            // Fallback to default
            RequestxClient::new().expect("Failed to clone RequestxClient")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RequestxClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_with_runtime() {
        let rt = Runtime::new().unwrap();
        let client = RequestxClient::with_runtime(rt);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/get".parse().unwrap();

        let result = client.get_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
        assert!(!response.body.is_empty());
    }

    #[tokio::test]
    async fn test_post_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/post".parse().unwrap();

        let result = client.post_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_put_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/put".parse().unwrap();

        let result = client.put_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_delete_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/delete".parse().unwrap();

        let result = client.delete_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_head_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/get".parse().unwrap();

        let result = client.head_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
        // HEAD requests should have empty body
        assert!(response.body.is_empty());
    }

    #[tokio::test]
    async fn test_options_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/get".parse().unwrap();

        let result = client.options_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        // OPTIONS requests typically return 200 or 204
        assert!(response.status_code == 200 || response.status_code == 204);
    }

    #[tokio::test]
    async fn test_patch_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/patch".parse().unwrap();

        let result = client.patch_async(url, None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_request_with_json_data() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/post".parse().unwrap();

        let json_data = serde_json::json!({
            "key": "value",
            "number": 42
        });

        let config = RequestConfig {
            method: Method::POST,
            url, // Remove unnecessary clone
            headers: None,
            params: None,
            data: None,
            json: Some(json_data),
            timeout: None,
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        };

        let result = client.request_async(config).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_request_with_form_data() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/post".parse().unwrap();

        let mut form_data = HashMap::new();
        form_data.insert("key1".to_string(), "value1".to_string());
        form_data.insert("key2".to_string(), "value2".to_string());

        let config = RequestConfig {
            method: Method::POST,
            url, // Remove unnecessary clone
            headers: None,
            params: None,
            data: Some(RequestData::Form(form_data)),
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        };

        let result = client.request_async(config).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_request_with_text_data() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/post".parse().unwrap();

        let config = RequestConfig {
            method: Method::POST,
            url, // Remove unnecessary clone
            headers: None,
            params: None,
            data: Some(RequestData::Text("Hello, World!".to_string())),
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        };

        let result = client.request_async(config).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[tokio::test]
    async fn test_request_with_timeout() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/delay/5".parse().unwrap();

        let config = RequestConfig {
            method: Method::GET,
            url, // Remove unnecessary clone
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: Some(Duration::from_secs(1)), // 1 second timeout for 5 second delay
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        };

        let result = client.request_async(config).await;
        assert!(result.is_err());

        // Should be a timeout error
        match result.unwrap_err() {
            RequestxError::ReadTimeout => (),
            _ => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let _client = RequestxClient::new().unwrap();
        let invalid_url = "not-a-valid-url";

        let result: Result<Uri, _> = invalid_url.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_request() {
        let client = RequestxClient::new().unwrap();
        let url: Uri = "https://httpbin.org/get".parse().unwrap();

        let config = RequestConfig {
            method: Method::GET,
            url, // Remove unnecessary clone
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
            cert: None,
            proxies: None,
            auth: None,
            stream: false,
        };

        let result = client.request_sync(config);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status_code, 200);
    }

    #[test]
    fn test_error_conversion() {
        // Test that our error types can be created and converted
        let network_error = RequestxError::RuntimeError("Test error".to_string());
        let py_err: pyo3::PyErr = network_error.into();
        assert!(py_err.to_string().contains("Test error"));
    }
}
