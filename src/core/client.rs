use bytes::Bytes;
use hyper::{Body, Client, HeaderMap, Method, Request, Uri};
use hyper_tls::HttpsConnector;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::error::RequestxError;

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
    client: Client<HttpsConnector<hyper::client::HttpConnector>>,
    runtime: Option<Runtime>,
}

impl RequestxClient {
    /// Create a new RequestxClient
    pub fn new() -> Result<Self, RequestxError> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        Ok(RequestxClient {
            client,
            runtime: None,
        })
    }

    /// Create a new RequestxClient with custom runtime
    pub fn with_runtime(runtime: Runtime) -> Result<Self, RequestxError> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        Ok(RequestxClient {
            client,
            runtime: Some(runtime),
        })
    }

    /// Perform an async HTTP GET request
    pub async fn get_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::GET,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::GET;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP POST request
    pub async fn post_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::POST,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::POST;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP PUT request
    pub async fn put_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::PUT,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::PUT;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP DELETE request
    pub async fn delete_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::DELETE,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::DELETE;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP HEAD request
    pub async fn head_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::HEAD,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::HEAD;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP OPTIONS request
    pub async fn options_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::OPTIONS,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::OPTIONS;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform an async HTTP PATCH request
    pub async fn patch_async(
        &self,
        url: Uri,
        config: Option<RequestConfig>,
    ) -> Result<ResponseData, RequestxError> {
        let mut request_config = config.unwrap_or_else(|| RequestConfig {
            method: Method::PATCH,
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
        });
        request_config.method = Method::PATCH;
        request_config.url = url;

        self.request_async(request_config).await
    }

    /// Perform a generic async HTTP request
    pub async fn request_async(
        &self,
        config: RequestConfig,
    ) -> Result<ResponseData, RequestxError> {
        // Build the request
        let mut request_builder = Request::builder()
            .method(config.method)
            .uri(config.url.clone());

        // Add headers
        if let Some(headers) = config.headers {
            for (name, value) in headers.iter() {
                request_builder = request_builder.header(name, value);
            }
        }

        // Build request body
        let body = match (&config.data, &config.json) {
            (Some(RequestData::Text(text)), None) => Body::from(text.clone()),
            (Some(RequestData::Bytes(bytes)), None) => Body::from(bytes.clone()),
            (Some(RequestData::Form(form)), None) => {
                // Convert form data to URL-encoded string
                let form_data = form
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                request_builder =
                    request_builder.header("content-type", "application/x-www-form-urlencoded");
                Body::from(form_data)
            }
            (None, Some(json)) => {
                let json_string = serde_json::to_string(json)?;
                request_builder = request_builder.header("content-type", "application/json");
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
            tokio::time::timeout(timeout, self.client.request(request)).await??
        } else {
            self.client.request(request).await?
        };

        // Extract response data
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        let url = config.url;

        // Read response body
        let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

        Ok(ResponseData {
            status_code,
            headers,
            body: body_bytes,
            url,
        })
    }

    /// Perform a synchronous HTTP request by blocking on async
    pub fn request_sync(&self, config: RequestConfig) -> Result<ResponseData, RequestxError> {
        if let Some(ref runtime) = self.runtime {
            runtime.block_on(self.request_async(config))
        } else {
            // Create a new runtime for this request
            let rt = Runtime::new().map_err(|e| {
                RequestxError::RuntimeError(format!("Failed to create runtime: {}", e))
            })?;
            rt.block_on(self.request_async(config))
        }
    }
}

impl Default for RequestxClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default RequestxClient")
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
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: Some(json_data),
            timeout: None,
            allow_redirects: true,
            verify: true,
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
            url: url.clone(),
            headers: None,
            params: None,
            data: Some(RequestData::Form(form_data)),
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
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
            url: url.clone(),
            headers: None,
            params: None,
            data: Some(RequestData::Text("Hello, World!".to_string())),
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
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
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: Some(Duration::from_secs(1)), // 1 second timeout for 5 second delay
            allow_redirects: true,
            verify: true,
        };

        let result = client.request_async(config).await;
        assert!(result.is_err());

        // Should be a timeout error
        match result.unwrap_err() {
            RequestxError::TimeoutError(_) => (),
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
            url: url.clone(),
            headers: None,
            params: None,
            data: None,
            json: None,
            timeout: None,
            allow_redirects: true,
            verify: true,
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
