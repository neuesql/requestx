# RequestX TODO List

**Total Tests:** 1407
**Initial Status:** 622 failed, 784 passed (56% pass rate)
**Current Status:** 606 failed, 800 passed (57% pass rate)
**Progress:** 16 more tests passing

## Completed Fixes

- [x] Exception type mapping - UnsupportedProtocol/LocalProtocolError for invalid URLs
- [x] Headers multi-value support - comma-joined values for __getitem__ and get()
- [x] Headers ordering - retain position when setting existing key
- [x] Headers setdefault() method
- [x] Response raise_for_status() - raise for non-2xx, include redirect location
- [x] URL trailing slash normalization - strip "/" path when no query/fragment
- [x] URL __eq__ with strings using to_string()
- [x] Client.send() - properly pass request headers to HTTP request
- [x] POST Content-Length: 0 for empty body

## Priority 1: Core Functionality (Blocking Many Tests)

### 1. [ ] Exception Type Mapping (blocks ~100+ tests)
- Invalid URLs should raise `UnsupportedProtocol` or `InvalidURL`, not `TransportError`
- URL validation should raise `LocalProtocolError` for malformed URLs
- Update `convert_reqwest_error` in `src/exceptions.rs` to detect URL scheme errors
- File: `src/exceptions.rs`, `src/client.rs`, `src/async_client.rs`

### 2. [ ] Headers Multi-Value Support (blocks ~21 tests)
- Headers should support multiple values for same key (e.g., "Set-Cookie")
- `get()` should return comma-joined values
- `get_list()` should return list of all values
- Need to change internal storage from `HashMap<String, String>` to `Vec<(String, String)>`
- Add `__iter__`, `keys()`, `values()`, `items()` methods
- File: `src/headers.rs`

### 3. [COMPLEX] build_request + request.headers.update() (blocks ~5 tests)
- PyO3 limitation: `request.headers` returns a clone, so `.update()` modifies the clone
- This is a fundamental issue with Python mutable reference semantics vs Rust ownership
- Requires complex solution involving shared Python objects (Py<Headers>)
- Previous attempt with Py<Headers> caused GIL deadlocks
- File: `src/request.rs`, `src/client.rs`

## Priority 2: Streaming & Async (blocks ~87 response tests)

### 4. [ ] Response Streaming Methods
- `iter_raw()` - sync iterator for raw bytes
- `iter_bytes()` with chunk_size support
- `iter_text()` with chunk_size support
- `iter_lines()` - should iterate over lines
- Need to track `is_stream_consumed` state
- File: `src/response.rs`

### 5. [ ] Async Response Methods
- `aread()` - async read content
- `aiter_raw()` - async iterator for raw bytes
- `aiter_bytes()` with chunk_size
- `aiter_text()` with chunk_size
- `aiter_lines()` - async line iterator
- `aclose()` - async close
- File: `src/response.rs`

### 6. [ ] Stream State Management
- Track `is_stream_consumed` flag
- Raise `StreamConsumed` when accessing content after streaming
- Raise `StreamClosed` when accessing after close
- File: `src/response.rs`

## Priority 3: URL Handling (blocks ~50 tests)

### 7. [ ] URL Path/Query Encoding
- Proper percent-encoding of path segments
- Proper percent-encoding of query parameters
- Handle already-encoded characters correctly
- File: `src/url.rs`

### 8. [ ] URL Validation
- Invalid hostname detection (raise `InvalidURL`)
- Excessively long component detection
- Non-printing character detection in components
- Port validation (raise `InvalidURL` for invalid ports)
- File: `src/url.rs`

### 9. [ ] URL copy_with() Method
- Support all URL components
- Handle relative paths correctly
- File: `src/url.rs`

## Priority 4: Auth Support (blocks ~77 tests)

### 10. [ ] Basic Auth Flow
- Auth with MockTransport
- Auth header hiding in repr
- Auth property getter/setter on clients
- File: `src/auth.rs`, `src/client.rs`

### 11. [ ] Digest Auth Implementation
- Full digest auth protocol with MD5, SHA, SHA-256, SHA-512
- Session variants (-SESS)
- QOP handling (auth, auth-int)
- Nonce count tracking
- Challenge reuse
- File: `src/auth.rs`

### 12. [ ] Custom Auth Support
- Allow callable auth functions
- Auth flow with multiple requests
- File: `src/auth.rs`

## Priority 5: Content Handling (blocks ~41 tests)

### 13. [ ] Request Content Stream Property
- `Request.stream` property for content streaming
- `SyncByteStream.from_data()` static method
- File: `src/request.rs`

### 14. [ ] Form Data Encoding
- Handle boolean values in form data
- Handle None values in form data
- Handle list values in form data
- File: `src/client.rs`

### 15. [ ] JSON Serialization Options
- Support `separators` for compact JSON
- Support `allow_nan=False` raising ValueError
- File: `src/client.rs`

## Priority 6: Redirects (blocks ~30 tests)

### 16. [ ] Redirect Handling
- `follow_redirects` parameter per-request
- Access to redirect history via `response.history`
- Max redirects limit with proper exception
- Cross-origin redirect cookie handling
- File: `src/client.rs`, `src/async_client.rs`, `src/response.rs`

## Priority 7: Transport & Mounts (blocks ~63 proxy tests + ~20 transport tests)

### 17. [ ] MockTransport Integration
- Use mounted transports based on URL pattern
- Proper transport selection logic
- File: `src/client.rs`, `src/async_client.rs`

### 18. [ ] Proxy Support
- HTTP proxy configuration
- HTTPS proxy configuration
- SOCKS proxy support
- NO_PROXY handling
- File: `src/client.rs`, `src/async_client.rs`

## Priority 8: Encoding/Decoding (blocks ~26 tests)

### 19. [ ] Content Decoding
- Proper gzip/deflate/brotli/zstd decoding
- Multi-encoding support (e.g., "gzip, deflate")
- DecodeError exception for invalid data
- File: `src/response.rs`

### 20. [ ] Charset Detection
- Autodetect encoding when no charset specified
- Support explicit encoding override
- BOM detection for UTF variants
- File: `src/response.rs`

## Priority 9: Event Hooks (blocks ~6 tests)

### 21. [ ] Event Hook Execution
- Call request hooks before sending
- Call response hooks after receiving
- File: `src/client.rs`, `src/async_client.rs`

## Priority 10: Cookies (blocks ~12 tests)

### 22. [ ] Cookie Jar Integration
- Domain-based cookie storage
- Path-based cookie storage
- Cookie expiration handling
- File: `src/cookies.rs`, `src/client.rs`

## Priority 11: Other

### 23. [ ] QueryParams Methods
- `multi_items()` method for repeated keys
- `get_list()` for multiple values
- File: `src/queryparams.rs`

### 24. [ ] Timeout Improvements
- Pool timeout support
- Per-request timeout override
- File: `src/timeout.rs`

### 25. [ ] Response Properties
- `response.num_bytes_downloaded` property
- `response.http_version` accuracy
- `response.extensions` property
- File: `src/response.rs`

### 26. [ ] ASGI Transport (blocks ~24 tests)
- ASGITransport implementation
- File: `src/transport.rs`

### 27. [ ] Multipart Improvements (blocks ~15 tests)
- Proper content-type header handling
- File tuple headers support
- StringIO/text mode file detection with error
- Non-seekable file support
- File: `src/multipart.rs`

---

## Quick Wins (Can Be Done Quickly)

1. Exception type mapping - straightforward mapping in `convert_reqwest_error`
2. `build_request` + `send()` header fix - copy headers in `send()` method
3. Response `raise_for_status()` method improvements

## Test Command Reference

```bash
# Run all target tests
pytest tests_requestx/ -v

# Run specific test file
pytest tests_requestx/models/test_responses.py -v

# Run single test
pytest tests_requestx/client/test_client.py::test_build_request -v

# Show short traceback
pytest tests_requestx/ --tb=short
```
