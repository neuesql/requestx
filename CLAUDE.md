# RequestX - httpx-compatible HTTP client powered by Rust

## Project Goal
ALL business logic must be in Rust. The Python `requestx/__init__.py` contains ONLY re-exports.
`pytest tests_requestx/ -v` must ALL PASS (100%).

## Current Status
- **811 passed** / 1407 total tests
- **595 failed** tests remaining

---

## Test Failure Analysis & Checkpoints

### Summary by Test File

| Test File | Failed | Feature Area | Priority | Dependencies |
|-----------|--------|--------------|----------|--------------|
| models/test_responses.py | 77 | Response streaming, async | P1 | AsyncByteStream, aiter, aread |
| client/test_auth.py | 77 | Client auth integration | P2 | DigestAuth flow, async client |
| client/test_proxies.py | 66 | Proxy configuration | P3 | Proxy class, mounts |
| client/test_async_client.py | 52 | AsyncClient operations | P1 | Trio support, async transport |
| models/test_url.py | 44 | URL edge cases | P1 | Empty scheme, IPv6, netloc |
| test_content.py | 30 | Content streaming | P1 | Iterator/async iterator content |
| client/test_redirects.py | 30 | Redirect handling | P2 | Response.next_request, history |
| client/test_client.py | 27 | Client operations | P0 | MockTransport, base_url |
| test_decoders.py | 26 | Content decoders | P2 | gzip, brotli, zstd, deflate |
| test_asgi.py | 24 | ASGI transport | P3 | ASGITransport class |
| test_config.py | 20 | Config objects | P0 | Timeout tuple, Proxy class |
| models/test_headers.py | 17 | Headers edge cases | P1 | Encoding, raw headers |
| test_multipart.py | 15 | Multipart encoding | P1 | Error messages, edge cases |
| client/test_headers.py | 15 | Client header handling | P1 | Host header, auth headers |
| models/test_requests.py | 13 | Request streaming | P1 | Generator content, pickling |
| test_utils.py | 10 | Utility functions | P2 | BOM detection, logging |
| test_timeouts.py | 10 | Timeout handling | P2 | Timeout exceptions |
| test_auth.py | 7 | Auth models | P0 | DigestAuth flow |
| models/test_queryparams.py | 7 | QueryParams edge cases | P1 | types, hashing |
| client/test_event_hooks.py | 7 | Event hooks | P2 | Hook callbacks |
| client/test_cookies.py | 7 | Cookie persistence | P1 | Response cookies extraction |
| client/test_properties.py | 6 | Client properties | P1 | base_url, headers getters |
| test_api.py | 3 | Top-level API | P0 | get, post functions |
| client/test_queryparams.py | 3 | Client params | P1 | Params merging |
| test_exceptions.py | 2 | Exception mapping | P1 | httpcore mapping |

---

## P0 - Critical (Must Fix First)

### [ ] CP-001: Timeout Construction from Tuple/Float
**File:** `test_config.py`
**Tests:** 5 failing
- `test_timeout_from_one_value`
- `test_timeout_from_tuple`
- `test_timeout_from_config_instance`
- `test_timeout_missing_default`
- `test_timeout_from_one_value_and_default`

**Issue:** Timeout constructor doesn't accept tuple or single float value
**Fix:** Add `Timeout::from_py()` that handles `float`, `tuple`, and `Timeout` inputs

### [ ] CP-002: DigestAuth Flow Implementation
**File:** `test_auth.py`
**Tests:** 7 failing
- `test_digest_auth_with_200`
- `test_digest_auth_with_401`
- `test_digest_auth_with_401_nonce_counting`
- `test_digest_auth_setting_cookie_in_request`
- `test_digest_auth_rfc_2069`
- `test_digest_auth_rfc_7616_md5`
- `test_digest_auth_rfc_7616_sha_256`

**Issue:** DigestAuth lacks `sync_auth_flow`/`async_auth_flow` methods
**Fix:** Implement DigestAuth flow with challenge parsing and response generation
**Dependencies:** None (auth model only)

### [ ] CP-003: Top-Level API Functions
**File:** `test_api.py`
**Tests:** 3 failing
- `test_get`
- `test_get_ssl`
- `test_post_with_body`

**Issue:** Module-level `get()`, `post()` functions not properly integrated
**Fix:** Ensure api.rs functions work correctly
**Dependencies:** Client, Response

### [ ] CP-004: Client Response Extensions
**File:** `client/test_client.py`
**Tests:** 5 failing (critical path)
- `test_get`
- `test_build_request`
- `test_build_post_request`
- `test_raise_for_status`
- `test_server_extensions`

**Issue:** Response missing `extensions` dict, request not attached
**Fix:** Add extensions support, ensure request is attached to response
**Dependencies:** Response class

---

## P1 - High Priority (Core Functionality)

### [ ] CP-005: Response Async Iteration (aiter, aread)
**File:** `models/test_responses.py`
**Tests:** ~40 failing
**Issue:** Response needs proper `aiter_bytes()`, `aiter_text()`, `aiter_lines()`, `aread()`
**Fix:** Implement async iteration methods returning async generators
**Dependencies:** AsyncByteStream

### [ ] CP-006: URL Edge Cases
**File:** `models/test_url.py`
**Tests:** 44 failing
**Issues:**
- Empty scheme URLs (`://example.com`)
- No authority URLs (`http://`)
- IPv6 handling (`[::1]`)
- `copy_with()` method edge cases
- `netloc` property

**Fix:** Store scheme/host/port separately, handle edge cases
**Dependencies:** None

### [ ] CP-007: Content Iterator Support
**File:** `test_content.py`
**Tests:** ~20 failing
**Issue:** Request doesn't accept iterator/generator as content
**Fix:** Add iterator/async iterator content support with Transfer-Encoding: chunked
**Dependencies:** Request class

### [ ] CP-008: Headers Raw/Encoding
**File:** `models/test_headers.py`
**Tests:** 17 failing
**Issues:**
- `raw` property missing
- Encoding detection
- Multiple header repr

**Fix:** Add `raw` property returning list of bytes tuples
**Dependencies:** None

### [ ] CP-009: Request Streaming Content
**File:** `models/test_requests.py`
**Tests:** 13 failing
**Issue:** Generator content, Transfer-Encoding header
**Fix:** Support iterator content with chunked encoding
**Dependencies:** CP-007

### [ ] CP-010: Client Properties
**File:** `client/test_properties.py`
**Tests:** 6 failing
**Issue:** Client missing property getters for base_url, headers, etc.
**Fix:** Add property getters to Client class
**Dependencies:** None

### [ ] CP-011: Cookie Persistence
**File:** `client/test_cookies.py`
**Tests:** 7 failing
**Issue:** Cookies not persisted across requests
**Fix:** Extract cookies from response Set-Cookie headers
**Dependencies:** Cookies.extract_cookies

### [ ] CP-012: QueryParams Edge Cases
**File:** `models/test_queryparams.py`
**Tests:** 7 failing
**Issues:**
- `__hash__` implementation
- Type coercion (bool, None)
- Deprecation warnings

**Fix:** Fix hash, add type coercion
**Dependencies:** None

### [ ] CP-013: Exception Mapping
**File:** `test_exceptions.py`
**Tests:** 2 failing
**Issue:** httpcore exceptions not mapped
**Fix:** Add httpcore exception mapping
**Dependencies:** None

### [ ] CP-014: Multipart Error Messages
**File:** `test_multipart.py`
**Tests:** 15 failing
**Issue:** Error messages don't match expected format
**Fix:** Update error message formatting
**Dependencies:** None

### [ ] CP-015: Client Headers Integration
**File:** `client/test_headers.py`
**Tests:** 15 failing
**Issue:** Host header with port, auth headers
**Fix:** Proper header merging and Host header generation
**Dependencies:** None

---

## P2 - Medium Priority (Extended Functionality)

### [ ] CP-016: DigestAuth Client Integration
**File:** `client/test_auth.py`
**Tests:** 77 failing
**Issue:** DigestAuth not integrated with client
**Fix:** Integrate DigestAuth flow with Client.send()
**Dependencies:** CP-002, AsyncClient

### [ ] CP-017: Redirect Handling
**File:** `client/test_redirects.py`
**Tests:** 30 failing
**Issue:** Response.next_request, follow_redirects
**Fix:** Implement redirect following with history
**Dependencies:** Response.next_request

### [ ] CP-018: Content Decoders
**File:** `test_decoders.py`
**Tests:** 26 failing
**Issues:**
- gzip decoder
- brotli decoder
- zstd decoder
- deflate decoder

**Fix:** Implement content-encoding decoders
**Dependencies:** Response.content

### [ ] CP-019: Event Hooks
**File:** `client/test_event_hooks.py`
**Tests:** 7 failing
**Issue:** Event hooks not called
**Fix:** Add hook support to Client
**Dependencies:** Client

### [ ] CP-020: Timeout Exceptions
**File:** `test_timeouts.py`
**Tests:** 10 failing
**Issue:** Timeout exceptions not raised properly
**Fix:** Map reqwest timeout errors
**Dependencies:** None

### [ ] CP-021: Utils BOM Detection
**File:** `test_utils.py`
**Tests:** 10 failing
**Issue:** UTF-32 BOM detection, logging
**Fix:** Fix encoding detection
**Dependencies:** None

---

## P3 - Lower Priority (Advanced Features)

### [ ] CP-022: Proxy Support
**File:** `client/test_proxies.py`
**Tests:** 66 failing
**Issue:** Proxy class and proxy mounts
**Fix:** Implement Proxy class with URL and auth
**Dependencies:** Client mounts

### [ ] CP-023: AsyncClient Full Support
**File:** `client/test_async_client.py`
**Tests:** 52 failing
**Issue:** AsyncClient operations, Trio support
**Fix:** Complete AsyncClient implementation
**Dependencies:** Async transport

### [ ] CP-024: ASGI Transport
**File:** `test_asgi.py`
**Tests:** 24 failing
**Issue:** ASGITransport not implemented
**Fix:** Implement ASGI transport
**Dependencies:** None

### [ ] CP-025: SSL Configuration
**File:** `test_config.py`
**Tests:** 12 failing
**Issue:** SSL context configuration
**Fix:** Implement SSL config options
**Dependencies:** None

---

## Implementation Order

1. **Phase 1 (P0):** CP-001 → CP-002 → CP-003 → CP-004
2. **Phase 2 (P1-Core):** CP-005 → CP-006 → CP-007 → CP-008
3. **Phase 3 (P1-Client):** CP-010 → CP-011 → CP-015
4. **Phase 4 (P1-Polish):** CP-009 → CP-012 → CP-013 → CP-014
5. **Phase 5 (P2):** CP-016 → CP-017 → CP-018 → CP-019 → CP-020 → CP-021
6. **Phase 6 (P3):** CP-022 → CP-023 → CP-024 → CP-025

---

## Progress Tracking

| Checkpoint | Status | Tests Fixed | Date |
|------------|--------|-------------|------|
| CP-001 | ⬜ Pending | 0/5 | - |
| CP-002 | ⬜ Pending | 0/7 | - |
| CP-003 | ⬜ Pending | 0/3 | - |
| CP-004 | ⬜ Pending | 0/5 | - |
| CP-005 | ⬜ Pending | 0/40 | - |
| CP-006 | ⬜ Pending | 0/44 | - |
| CP-007 | ⬜ Pending | 0/20 | - |
| CP-008 | ⬜ Pending | 0/17 | - |
| CP-009 | ⬜ Pending | 0/13 | - |
| CP-010 | ⬜ Pending | 0/6 | - |
| CP-011 | ⬜ Pending | 0/7 | - |
| CP-012 | ⬜ Pending | 0/7 | - |
| CP-013 | ⬜ Pending | 0/2 | - |
| CP-014 | ⬜ Pending | 0/15 | - |
| CP-015 | ⬜ Pending | 0/15 | - |

---

## Notes

- Trio tests are duplicates of asyncio tests - fixing asyncio often fixes both
- Many client tests depend on Response having proper async methods
- MockTransport must return Response with request attached
