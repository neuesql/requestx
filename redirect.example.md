# RequestX Redirection Implementation

## Key Requirements from Unit Tests

1. `follow_redirects` parameter (default: False in httpx)
2. `response.history` - list of previous responses in redirect chain
3. `response.url` - final URL after redirects
4. `response.next_request` - for manual redirect following
5. Max redirect limit (default 20, raises `TooManyRedirects`)
6. Cross-domain auth header stripping
7. Body handling: 308 preserves body, 303 removes body
8. Cookie persistence across redirects

## Implementation

### 1. Enhanced Response Model
```rust
// src/models.rs
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass]
#[derive(Clone)]
pub struct Request {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: Url,
    pub headers: Headers,
    pub content: Option<Vec<u8>>,
}

#[pyclass]
#[derive(Clone)]
pub struct Url {
    inner: url::Url,
}

#[pymethods]
impl Url {
    #[getter]
    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }
    
    #[getter]
    pub fn host(&self) -> Option<&str> {
        self.inner.host_str()
    }
    
    #[getter]
    pub fn path(&self) -> &str {
        self.inner.path()
    }
    
    #[getter]
    pub fn query(&self) -> Option<&str> {
        self.inner.query()
    }
    
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
    
    pub fn __repr__(&self) -> String {
        format!("URL('{}')", self.inner)
    }
    
    pub fn __eq__(&self, other: &str) -> bool {
        self.inner.as_str() == other
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Response {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub url: Url,
    #[pyo3(get)]
    pub request: Request,
    #[pyo3(get)]
    pub history: Vec<Response>,        // Redirect chain
    #[pyo3(get)]
    pub next_request: Option<Request>, // For manual redirect following
    headers: Headers,
    content: Option<Vec<u8>>,
}

#[pymethods]
impl Response {
    #[getter]
    pub fn text(&self) -> String {
        self.content
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).to_string())
            .unwrap_or_default()
    }
    
    #[getter]
    pub fn headers(&self) -> Headers {
        self.headers.clone()
    }
    
    pub fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        let json_mod = py.import("json")?;
        json_mod.call_method1("loads", (self.text(),)).map(|o| o.into())
    }
}
```

### 2. Custom Redirect Policy
```rust
// src/redirect.rs
use reqwest::redirect::{Attempt, Policy};
use std::sync::{Arc, Mutex};

pub struct RedirectState {
    pub history: Vec<RedirectEntry>,
    pub max_redirects: usize,
}

pub struct RedirectEntry {
    pub url: String,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub request: RequestSnapshot,
}

pub struct RequestSnapshot {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
}

/// Custom redirect policy that captures history
pub fn create_redirect_policy(
    follow_redirects: bool,
    max_redirects: usize,
    state: Arc<Mutex<RedirectState>>,
) -> Policy {
    if !follow_redirects {
        return Policy::none();
    }
    
    Policy::custom(move |attempt: Attempt<'_>| {
        let mut state = state.lock().unwrap();
        
        // Check max redirects
        if attempt.previous().len() >= max_redirects {
            return attempt.error(TooManyRedirectsError);
        }
        
        // Record this redirect in history
        state.history.push(RedirectEntry {
            url: attempt.url().to_string(),
            status_code: attempt.status().as_u16(),
            // ... capture headers and request
        });
        
        // Handle cross-domain auth stripping
        let prev_url = attempt.previous().last().map(|u| u.clone());
        let next_url = attempt.url();
        
        if is_cross_domain(&prev_url, next_url) {
            // reqwest handles this, but we track it
        }
        
        attempt.follow()
    })
}

fn is_cross_domain(prev: &Option<url::Url>, next: &url::Url) -> bool {
    match prev {
        Some(p) => p.host() != next.host(),
        None => false,
    }
}
```

### 3. Client with Redirect Support
```rust
// src/client.rs
use crate::redirect::{RedirectState, create_redirect_policy};
use reqwest::redirect::Policy;
use std::sync::{Arc, Mutex};

#[pyclass]
pub struct Client {
    // Base client without redirects (we handle manually for history)
    inner: ReqwestClient,
    hooks: EventHooks,
    runtime: tokio::runtime::Runtime,
    max_redirects: usize,
    follow_redirects: bool,  // Default behavior
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (*, event_hooks=None, timeout=None, follow_redirects=false, max_redirects=20))]
    pub fn new(
        py: Python<'_>,
        event_hooks: Option<&Bound<'_, PyDict>>,
        timeout: Option<f64>,
        follow_redirects: bool,
        max_redirects: usize,
    ) -> PyResult<Self> {
        // Build client with NO automatic redirects - we handle manually
        let inner = ReqwestClient::builder()
            .redirect(Policy::none())  // Disable auto-redirect
            .timeout(timeout.map(Duration::from_secs_f64).unwrap_or(Duration::from_secs(30)))
            .build()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        
        Ok(Self {
            inner,
            hooks: EventHooks::from_py_dict(py, event_hooks)?,
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
            max_redirects,
            follow_redirects,
        })
    }
    
    #[pyo3(signature = (url, *, headers=None, follow_redirects=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        url: String,
        headers: Option<HashMap<String, String>>,
        follow_redirects: Option<bool>,  // Override per-request
    ) -> PyResult<Response> {
        self.request(py, "GET", url, headers, None, None, follow_redirects)
    }
    
    #[pyo3(signature = (method, url, *, headers=None, content=None, json=None, follow_redirects=None))]
    pub fn request(
        &self,
        py: Python<'_>,
        method: &str,
        url: String,
        headers: Option<HashMap<String, String>>,
        content: Option<Vec<u8>>,
        json: Option<PyObject>,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        let follow = follow_redirects.unwrap_or(self.follow_redirects);
        let mut headers = headers.unwrap_or_default();
        
        // Serialize JSON body
        let body = if let Some(j) = json {
            headers.insert("content-type".into(), "application/json".into());
            let json_mod = py.import("json")?;
            let s: String = json_mod.call_method1("dumps", (j,))?.extract()?;
            Some(s.into_bytes())
        } else {
            content
        };
        
        // Build initial request
        let mut current_url = url.clone();
        let mut current_method = method.to_string();
        let mut current_headers = headers.clone();
        let mut current_body = body.clone();
        let mut history: Vec<Response> = vec![];
        let original_request = Request::new(method.into(), url.clone(), Some(headers.clone()), body.clone());
        
        loop {
            // Execute request hooks
            let request = Request::new(
                current_method.clone(),
                current_url.clone(),
                Some(current_headers.clone()),
                current_body.clone(),
            );
            for hook in &self.hooks.request {
                hook.call_sync(py, request.clone().into_py(py))?;
            }
            
            // Send request
            let response = self.runtime.block_on(async {
                let mut req = self.inner.request(
                    reqwest::Method::from_bytes(current_method.as_bytes()).unwrap(),
                    &current_url,
                );
                for (k, v) in &current_headers {
                    req = req.header(k.as_str(), v.as_str());
                }
                if let Some(b) = &current_body {
                    req = req.body(b.clone());
                }
                req.send().await
            }).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            
            let status = response.status().as_u16();
            let resp_url = response.url().clone();
            let resp_headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            
            let content_bytes = self.runtime
                .block_on(response.bytes())
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
                .to_vec();
            
            // Check if redirect
            let is_redirect = matches!(status, 301 | 302 | 303 | 307 | 308);
            let location = resp_headers.get("location").cloned();
            
            if is_redirect && follow && location.is_some() {
                // Check max redirects
                if history.len() >= self.max_redirects {
                    return Err(TooManyRedirects::new_err(format!(
                        "Exceeded {} redirects", self.max_redirects
                    )));
                }
                
                let location = location.unwrap();
                let next_url = resolve_redirect_url(&current_url, &location)?;
                
                // Build response for history (with its own history)
                let hist_response = Response {
                    status_code: status,
                    url: Url::parse(&current_url)?,
                    request: request.clone(),
                    history: history.clone(),
                    next_request: None,
                    headers: Headers::from(resp_headers.clone()),
                    content: Some(content_bytes),
                };
                history.push(hist_response);
                
                // Determine next method and body per RFC
                let (next_method, next_body) = match status {
                    // 307/308: Preserve method and body
                    307 | 308 => (current_method.clone(), current_body.clone()),
                    // 303: Always GET, no body
                    303 => ("GET".to_string(), None),
                    // 301/302: GET for POST (historical behavior), preserve others
                    301 | 302 if current_method == "POST" => ("GET".to_string(), None),
                    _ => (current_method.clone(), None),
                };
                
                // Strip auth on cross-domain
                let mut next_headers = current_headers.clone();
                if is_cross_domain(&current_url, &next_url) {
                    next_headers.remove("authorization");
                }
                
                // Remove body headers if no body
                if next_body.is_none() {
                    next_headers.remove("content-length");
                    next_headers.remove("content-type");
                    next_headers.remove("transfer-encoding");
                }
                
                current_url = next_url;
                current_method = next_method;
                current_headers = next_headers;
                current_body = next_body;
                continue;
            }
            
            // Build next_request for manual following
            let next_request = if is_redirect && location.is_some() {
                let loc = location.unwrap();
                let next_url = resolve_redirect_url(&current_url, &loc)?;
                let (method, body) = compute_redirect_method_body(status, &current_method, &current_body);
                Some(Request::new(method, next_url, Some(current_headers.clone()), body))
            } else {
                None
            };
            
            // Final response
            let final_response = Response {
                status_code: status,
                url: Url::from(resp_url),
                request: original_request,
                history,
                next_request,
                headers: Headers::from(resp_headers),
                content: Some(content_bytes),
            };
            
            // Execute response hooks
            for hook in &self.hooks.response {
                hook.call_sync(py, final_response.clone().into_py(py))?;
            }
            
            return Ok(final_response);
        }
    }
    
    /// Build a request without sending
    pub fn build_request(
        &self,
        method: &str,
        url: String,
        headers: Option<HashMap<String, String>>,
        content: Option<Vec<u8>>,
    ) -> Request {
        Request::new(method.into(), url, headers, content)
    }
    
    /// Send a pre-built request
    #[pyo3(signature = (request, *, follow_redirects=None))]
    pub fn send(
        &self,
        py: Python<'_>,
        request: Request,
        follow_redirects: Option<bool>,
    ) -> PyResult<Response> {
        self.request(
            py,
            &request.method,
            request.url.to_string(),
            Some(request.headers.into()),
            request.content,
            None,
            follow_redirects,
        )
    }
}

// Helper functions
fn resolve_redirect_url(base: &str, location: &str) -> PyResult<String> {
    let base_url = url::Url::parse(base)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    
    base_url.join(location)
        .map(|u| u.to_string())
        .map_err(|e| RemoteProtocolError::new_err(e.to_string()))
}

fn is_cross_domain(prev: &str, next: &str) -> bool {
    let prev_url = url::Url::parse(prev).ok();
    let next_url = url::Url::parse(next).ok();
    match (prev_url, next_url) {
        (Some(p), Some(n)) => p.host() != n.host(),
        _ => false,
    }
}

fn compute_redirect_method_body(
    status: u16,
    method: &str,
    body: &Option<Vec<u8>>,
) -> (String, Option<Vec<u8>>) {
    match status {
        307 | 308 => (method.to_string(), body.clone()),
        303 => ("GET".to_string(), None),
        301 | 302 if method == "POST" => ("GET".to_string(), None),
        _ => (method.to_string(), None),
    }
}
```

### 4. Exception Types
```rust
// src/exceptions.rs
use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(requestx, HTTPError, PyException);
create_exception!(requestx, TooManyRedirects, HTTPError);
create_exception!(requestx, RemoteProtocolError, HTTPError);
create_exception!(requestx, UnsupportedProtocol, HTTPError);
create_exception!(requestx, StreamConsumed, HTTPError);
```

### 5. Module Registration
```rust
// src/lib.rs
#[pymodule]
fn requestx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_class::<AsyncClient>()?;
    m.add_class::<Request>()?;
    m.add_class::<Response>()?;
    m.add_class::<Url>()?;
    m.add_class::<Headers>()?;
    m.add_class::<MockTransport>()?;
    
    // Exceptions
    m.add("HTTPError", m.py().get_type::<HTTPError>())?;
    m.add("TooManyRedirects", m.py().get_type::<TooManyRedirects>())?;
    m.add("RemoteProtocolError", m.py().get_type::<RemoteProtocolError>())?;
    m.add("UnsupportedProtocol", m.py().get_type::<UnsupportedProtocol>())?;
    
    // Status codes
    m.add("codes", StatusCodes::new())?;
    
    Ok(())
}
```

## Redirect Behavior Summary

| Status | Method Change | Body Preserved | Auth Cross-Domain |
|--------|--------------|----------------|-------------------|
| 301    | POST→GET     | No             | Stripped          |
| 302    | POST→GET     | No             | Stripped          |
| 303    | Always GET   | No             | Stripped          |
| 307    | Preserved    | Yes            | Stripped          |
| 308    | Preserved    | Yes            | Stripped          |

## Python Usage
```python
import requestx

# Auto-follow redirects
client = requestx.Client(follow_redirects=True)
response = client.get("https://example.org/redirect_301")
print(response.url)           # Final URL
print(len(response.history))  # Number of redirects
print(response.history[0].url)  # First redirect URL

# Manual redirect following
client = requestx.Client()
response = client.get("https://example.org/redirect_303", follow_redirects=False)
if response.next_request:
    response = client.send(response.next_request)

# With build_request/send pattern
request = client.build_request("POST", "https://example.org/redirect_303")
response = client.send(request, follow_redirects=False)
```