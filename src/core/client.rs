use base64::prelude::*;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{Method, Request, Uri};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use hyper_tls::HttpsConnector;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use crate::config::get_http_client_config;
use crate::error::RequestxError;

/// Request data types that map directly to hyper body types
#[derive(Debug, Clone)]
pub enum RequestData {
    Text(String),
    Bytes(Vec<u8>),
    Form(HashMap<String, String>),
    Json(Value),
}

/// Simple request configuration that maps directly to hyper Request
#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub method: Method,
    pub url: Uri,
    pub headers: Option<hyper::HeaderMap>,
    pub params: Option<HashMap<String, String>>,
    pub data: Option<RequestData>,
    pub timeout: Option<Duration>,
    pub verify: bool,
    pub auth: Option<(String, String)>,
}

/// Response data that directly wraps hyper Response
#[derive(Debug)]
pub struct ResponseData {
    pub status_code: u16,
    pub headers: hyper::HeaderMap,
    pub body: Bytes,
    pub url: Uri,
}

/// Create a configured hyper client
pub fn create_client() -> Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>> {
    let config = get_http_client_config();
    let https = HttpsConnector::new();
    
    Client::builder(TokioExecutor::new())
        .pool_idle_timeout(config.pool_idle_timeout())
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .build(https)
}

/// Create a custom client with SSL verification settings
fn create_custom_client(
    verify: bool,
) -> Result<Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>>, RequestxError> {
    if verify {
        return Ok(create_client());
    }
    
    let config = get_http_client_config();

    // For verify=false, create a custom TLS connector that accepts invalid certs
    let mut tls_builder = hyper_tls::native_tls::TlsConnector::builder();
    tls_builder.danger_accept_invalid_certs(true);
    tls_builder.danger_accept_invalid_hostnames(true);

    let tls_connector = tls_builder
        .build()
        .map_err(|e| RequestxError::SslError(format!("Failed to create TLS connector: {e}")))?;

    let mut http_connector = HttpConnector::new();
    http_connector.enforce_http(false);

    let https_connector = HttpsConnector::from((http_connector, tls_connector.into()));

    Ok(Client::builder(TokioExecutor::new())
        .pool_idle_timeout(config.pool_idle_timeout())
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .build(https_connector))
}

// Global shared client for connection pooling
static GLOBAL_CLIENT: OnceLock<Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>>> =
    OnceLock::new();

fn get_global_client() -> &'static Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>> {
    GLOBAL_CLIENT.get_or_init(|| create_client())
}

/// Simplified HTTP client that directly uses hyper
pub struct RequestxClient {
    use_global_client: bool,
    custom_client: Option<Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>>>,
}

impl RequestxClient {
    /// Create a new RequestxClient using global shared client
    pub fn new() -> Result<Self, RequestxError> {
        Ok(RequestxClient {
            use_global_client: true,
            custom_client: None,
        })
    }

    /// Create a new RequestxClient with custom SSL verification
    pub fn with_verify(verify: bool) -> Result<Self, RequestxError> {
        if verify {
            Ok(RequestxClient {
                use_global_client: true,
                custom_client: None,
            })
        } else {
            let custom_client = create_custom_client(false)?;
            Ok(RequestxClient {
                use_global_client: false,
                custom_client: Some(custom_client),
            })
        }
    }

    /// Get the appropriate client
    fn get_client(&self) -> &Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, hyper::Error>> {
        if self.use_global_client {
            get_global_client()
        } else {
            self.custom_client.as_ref().unwrap()
        }
    }

    /// Execute HTTP request asynchronously - direct hyper usage
    pub async fn request_async(&self, config: RequestConfig) -> Result<ResponseData, RequestxError> {
        let client = self.get_client();
        
        // Build URL with query parameters
        let mut url = config.url.to_string();
        if let Some(params) = &config.params {
            if !params.is_empty() {
                let query_string = params
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                url = if config.url.query().is_some() {
                    format!("{}&{}", url, query_string)
                } else {
                    format!("{}?{}", url, query_string)
                };
            }
        }
        
        let uri: Uri = url.parse().map_err(|e| RequestxError::InvalidUrl(e))?;
        
        // Create hyper request directly
        let mut request_builder = Request::builder()
            .method(config.method)
            .uri(uri.clone());
        
        // Add headers
        if let Some(headers) = config.headers {
            for (name, value) in headers.iter() {
                request_builder = request_builder.header(name, value);
            }
        }
        
        // Add authentication
         if let Some((username, password)) = config.auth {
             let credentials = BASE64_STANDARD.encode(format!("{}:{}", username, password));
             request_builder = request_builder.header("Authorization", format!("Basic {}", credentials));
         }
        
        // Create request body
         let request = match config.data {
             Some(RequestData::Text(text)) => {
                 let body = Full::new(Bytes::from(text)).map_err(|e| match e {});
                 request_builder
                     .header("Content-Type", "text/plain")
                     .body(BoxBody::new(body))
                     .map_err(|e| RequestxError::HttpRequestError(e))?
             },
             Some(RequestData::Bytes(bytes)) => {
                 let body = Full::new(Bytes::from(bytes)).map_err(|e| match e {});
                 request_builder
                     .body(BoxBody::new(body))
                     .map_err(|e| RequestxError::HttpRequestError(e))?
              },
             Some(RequestData::Form(form_data)) => {
                 let form_string = form_data
                     .iter()
                     .map(|(k, v)| format!("{}={}", k, v))
                     .collect::<Vec<_>>()
                     .join("&");
                 let body = Full::new(Bytes::from(form_string)).map_err(|e| match e {});
                 request_builder
                     .header("Content-Type", "application/x-www-form-urlencoded")
                     .body(BoxBody::new(body))
                     .map_err(|e| RequestxError::HttpRequestError(e))?
             },
             Some(RequestData::Json(json)) => {
                 let json_string = serde_json::to_string(&json)
                     .map_err(|e| RequestxError::JsonDecodeError(e))?;
                 let body = Full::new(Bytes::from(json_string)).map_err(|e| match e {});
                 request_builder
                     .header("Content-Type", "application/json")
                     .body(BoxBody::new(body))
                     .map_err(|e| RequestxError::HttpRequestError(e))?
             },
             None => {
                 let body = Empty::new().map_err(|e| match e {});
                 request_builder
                     .body(BoxBody::new(body))
                     .map_err(|e| RequestxError::HttpRequestError(e))?
             },
         };
        
        // Execute request with timeout
         let response = if let Some(timeout) = config.timeout {
             tokio::time::timeout(timeout, client.request(request))
                 .await
                 .map_err(|_| RequestxError::ReadTimeout)?
                 .map_err(|e| RequestxError::RuntimeError(e.to_string()))?
         } else {
             client.request(request)
                 .await
                 .map_err(|e| RequestxError::RuntimeError(e.to_string()))?
         };
        
        // Extract response data
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        
        // Collect response body
         let body_bytes = response
             .into_body()
             .collect()
             .await
             .map_err(|e| RequestxError::RuntimeError(e.to_string()))?
             .to_bytes();
        
        Ok(ResponseData {
            status_code,
            headers,
            body: body_bytes,
            url: uri,
        })
    }
    
    /// Execute HTTP request synchronously using runtime
     pub fn request_sync(&self, config: RequestConfig) -> Result<ResponseData, RequestxError> {
         let runtime_manager = crate::core::runtime::get_global_runtime_manager();
         let runtime = runtime_manager.get_runtime();
         runtime.block_on(self.request_async(config))
     }
    
    /// Convenience methods for different HTTP methods
    pub async fn get_async(&self, url: Uri, config: Option<RequestConfig>) -> Result<ResponseData, RequestxError> {
        let config = config.unwrap_or(RequestConfig {
            method: Method::GET,
            url,
            headers: None,
            params: None,
            data: None,
            timeout: None,
            verify: true,
            auth: None,
        });
        self.request_async(config).await
    }
    
    pub async fn post_async(&self, url: Uri, config: Option<RequestConfig>) -> Result<ResponseData, RequestxError> {
        let mut config = config.unwrap_or(RequestConfig {
            method: Method::POST,
            url,
            headers: None,
            params: None,
            data: None,
            timeout: None,
            verify: true,
            auth: None,
        });
        config.method = Method::POST;
        self.request_async(config).await
    }
}

impl Default for RequestxClient {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Clone for RequestxClient {
    fn clone(&self) -> Self {
        if self.use_global_client {
            RequestxClient {
                use_global_client: true,
                custom_client: None,
            }
        } else {
            // For custom clients, create a new one with the same settings
            RequestxClient {
                use_global_client: false,
                custom_client: self.custom_client.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        let client = RequestxClient::new().unwrap();
        assert!(client.use_global_client);
    }
    
    #[tokio::test]
    async fn test_get_request() {
        let client = RequestxClient::new().unwrap();
        let config = RequestConfig {
            method: Method::GET,
            url: "https://httpbin.org/get".parse().unwrap(),
            headers: None,
            params: None,
            data: None,
            timeout: Some(Duration::from_secs(10)),
            verify: true,
            auth: None,
        };
        
        let result = client.request_async(config).await;
        assert!(result.is_ok());
    }
}
