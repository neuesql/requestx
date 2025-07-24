use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::error::RequestxError;

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
}

impl Default for RequestxClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default RequestxClient")
    }
}