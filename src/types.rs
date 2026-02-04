//! Additional types: streams, auth, status codes

use pyo3::prelude::*;

use crate::common::impl_byte_stream;

impl_byte_stream!(SyncByteStream, "SyncByteStream");
impl_byte_stream!(AsyncByteStream, "AsyncByteStream");

/// Basic authentication
#[pyclass(name = "BasicAuth")]
#[derive(Clone, Debug)]
pub struct BasicAuth {
    #[pyo3(get)]
    pub username: String,
    #[pyo3(get)]
    pub password: String,
}

#[pymethods]
impl BasicAuth {
    #[new]
    #[pyo3(signature = (username, password=""))]
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("BasicAuth(username={:?}, password=***)", self.username)
    }

    fn __eq__(&self, other: &BasicAuth) -> bool {
        self.username == other.username && self.password == other.password
    }
}

/// Digest authentication (placeholder)
#[pyclass(name = "DigestAuth")]
#[derive(Clone, Debug)]
pub struct DigestAuth {
    #[pyo3(get)]
    pub username: String,
    #[pyo3(get)]
    pub password: String,
}

#[pymethods]
impl DigestAuth {
    #[new]
    fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("DigestAuth(username={:?}, password=***)", self.username)
    }
}

/// NetRC authentication (placeholder)
#[pyclass(name = "NetRCAuth")]
#[derive(Clone, Debug)]
pub struct NetRCAuth {
    #[pyo3(get)]
    pub file: Option<String>,
}

#[pymethods]
impl NetRCAuth {
    #[new]
    #[pyo3(signature = (file=None))]
    fn new(file: Option<&str>) -> Self {
        Self { file: file.map(|s| s.to_string()) }
    }

    fn __repr__(&self) -> String {
        format!("NetRCAuth(file={:?})", self.file)
    }
}

/// HTTP status codes - provides flexible access patterns
#[pyclass(name = "codes", subclass)]
pub struct codes;

impl codes {
    fn name_to_code(name: &str) -> Option<u16> {
        match name.to_uppercase().as_str() {
            "CONTINUE" => Some(100),
            "SWITCHING_PROTOCOLS" => Some(101),
            "PROCESSING" => Some(102),
            "EARLY_HINTS" => Some(103),
            "OK" => Some(200),
            "CREATED" => Some(201),
            "ACCEPTED" => Some(202),
            "NON_AUTHORITATIVE_INFORMATION" => Some(203),
            "NO_CONTENT" => Some(204),
            "RESET_CONTENT" => Some(205),
            "PARTIAL_CONTENT" => Some(206),
            "MULTI_STATUS" => Some(207),
            "ALREADY_REPORTED" => Some(208),
            "IM_USED" => Some(226),
            "MULTIPLE_CHOICES" => Some(300),
            "MOVED_PERMANENTLY" => Some(301),
            "FOUND" => Some(302),
            "SEE_OTHER" => Some(303),
            "NOT_MODIFIED" => Some(304),
            "USE_PROXY" => Some(305),
            "TEMPORARY_REDIRECT" => Some(307),
            "PERMANENT_REDIRECT" => Some(308),
            "BAD_REQUEST" => Some(400),
            "UNAUTHORIZED" => Some(401),
            "PAYMENT_REQUIRED" => Some(402),
            "FORBIDDEN" => Some(403),
            "NOT_FOUND" => Some(404),
            "METHOD_NOT_ALLOWED" => Some(405),
            "NOT_ACCEPTABLE" => Some(406),
            "PROXY_AUTHENTICATION_REQUIRED" => Some(407),
            "REQUEST_TIMEOUT" => Some(408),
            "CONFLICT" => Some(409),
            "GONE" => Some(410),
            "LENGTH_REQUIRED" => Some(411),
            "PRECONDITION_FAILED" => Some(412),
            "PAYLOAD_TOO_LARGE" => Some(413),
            "URI_TOO_LONG" => Some(414),
            "UNSUPPORTED_MEDIA_TYPE" => Some(415),
            "RANGE_NOT_SATISFIABLE" => Some(416),
            "EXPECTATION_FAILED" => Some(417),
            "IM_A_TEAPOT" => Some(418),
            "MISDIRECTED_REQUEST" => Some(421),
            "UNPROCESSABLE_ENTITY" => Some(422),
            "LOCKED" => Some(423),
            "FAILED_DEPENDENCY" => Some(424),
            "TOO_EARLY" => Some(425),
            "UPGRADE_REQUIRED" => Some(426),
            "PRECONDITION_REQUIRED" => Some(428),
            "TOO_MANY_REQUESTS" => Some(429),
            "REQUEST_HEADER_FIELDS_TOO_LARGE" => Some(431),
            "UNAVAILABLE_FOR_LEGAL_REASONS" => Some(451),
            "INTERNAL_SERVER_ERROR" => Some(500),
            "NOT_IMPLEMENTED" => Some(501),
            "BAD_GATEWAY" => Some(502),
            "SERVICE_UNAVAILABLE" => Some(503),
            "GATEWAY_TIMEOUT" => Some(504),
            "HTTP_VERSION_NOT_SUPPORTED" => Some(505),
            "VARIANT_ALSO_NEGOTIATES" => Some(506),
            "INSUFFICIENT_STORAGE" => Some(507),
            "LOOP_DETECTED" => Some(508),
            "NOT_EXTENDED" => Some(510),
            "NETWORK_AUTHENTICATION_REQUIRED" => Some(511),
            _ => None,
        }
    }

    fn code_to_phrase(code: u16) -> &'static str {
        match code {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            226 => "IM Used",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a teapot",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            _ => "",
        }
    }
}

#[pymethods]
impl codes {
    /// Allow codes["NOT_FOUND"] access
    #[classmethod]
    fn __class_getitem__(_cls: &Bound<'_, pyo3::types::PyType>, name: &str) -> PyResult<u16> {
        Self::name_to_code(name).ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(name.to_string()))
    }

    /// Get reason phrase for a status code
    #[staticmethod]
    fn get_reason_phrase(code: u16) -> &'static str {
        Self::code_to_phrase(code)
    }

    // 1xx Informational
    #[classattr]
    const CONTINUE: u16 = 100;
    #[classattr]
    const SWITCHING_PROTOCOLS: u16 = 101;
    #[classattr]
    const PROCESSING: u16 = 102;
    #[classattr]
    const EARLY_HINTS: u16 = 103;

    // 2xx Success
    #[classattr]
    const OK: u16 = 200;
    #[classattr]
    const CREATED: u16 = 201;
    #[classattr]
    const ACCEPTED: u16 = 202;
    #[classattr]
    const NON_AUTHORITATIVE_INFORMATION: u16 = 203;
    #[classattr]
    const NO_CONTENT: u16 = 204;
    #[classattr]
    const RESET_CONTENT: u16 = 205;
    #[classattr]
    const PARTIAL_CONTENT: u16 = 206;
    #[classattr]
    const MULTI_STATUS: u16 = 207;
    #[classattr]
    const ALREADY_REPORTED: u16 = 208;
    #[classattr]
    const IM_USED: u16 = 226;

    // 3xx Redirection
    #[classattr]
    const MULTIPLE_CHOICES: u16 = 300;
    #[classattr]
    const MOVED_PERMANENTLY: u16 = 301;
    #[classattr]
    const FOUND: u16 = 302;
    #[classattr]
    const SEE_OTHER: u16 = 303;
    #[classattr]
    const NOT_MODIFIED: u16 = 304;
    #[classattr]
    const USE_PROXY: u16 = 305;
    #[classattr]
    const TEMPORARY_REDIRECT: u16 = 307;
    #[classattr]
    const PERMANENT_REDIRECT: u16 = 308;

    // 4xx Client Error
    #[classattr]
    const BAD_REQUEST: u16 = 400;
    #[classattr]
    const UNAUTHORIZED: u16 = 401;
    #[classattr]
    const PAYMENT_REQUIRED: u16 = 402;
    #[classattr]
    const FORBIDDEN: u16 = 403;
    #[classattr]
    const NOT_FOUND: u16 = 404;
    #[classattr]
    const METHOD_NOT_ALLOWED: u16 = 405;
    #[classattr]
    const NOT_ACCEPTABLE: u16 = 406;
    #[classattr]
    const PROXY_AUTHENTICATION_REQUIRED: u16 = 407;
    #[classattr]
    const REQUEST_TIMEOUT: u16 = 408;
    #[classattr]
    const CONFLICT: u16 = 409;
    #[classattr]
    const GONE: u16 = 410;
    #[classattr]
    const LENGTH_REQUIRED: u16 = 411;
    #[classattr]
    const PRECONDITION_FAILED: u16 = 412;
    #[classattr]
    const PAYLOAD_TOO_LARGE: u16 = 413;
    #[classattr]
    const URI_TOO_LONG: u16 = 414;
    #[classattr]
    const UNSUPPORTED_MEDIA_TYPE: u16 = 415;
    #[classattr]
    const RANGE_NOT_SATISFIABLE: u16 = 416;
    #[classattr]
    const EXPECTATION_FAILED: u16 = 417;
    #[classattr]
    const IM_A_TEAPOT: u16 = 418;
    #[classattr]
    const MISDIRECTED_REQUEST: u16 = 421;
    #[classattr]
    const UNPROCESSABLE_ENTITY: u16 = 422;
    #[classattr]
    const LOCKED: u16 = 423;
    #[classattr]
    const FAILED_DEPENDENCY: u16 = 424;
    #[classattr]
    const TOO_EARLY: u16 = 425;
    #[classattr]
    const UPGRADE_REQUIRED: u16 = 426;
    #[classattr]
    const PRECONDITION_REQUIRED: u16 = 428;
    #[classattr]
    const TOO_MANY_REQUESTS: u16 = 429;
    #[classattr]
    const REQUEST_HEADER_FIELDS_TOO_LARGE: u16 = 431;
    #[classattr]
    const UNAVAILABLE_FOR_LEGAL_REASONS: u16 = 451;

    // 5xx Server Error
    #[classattr]
    const INTERNAL_SERVER_ERROR: u16 = 500;
    #[classattr]
    const NOT_IMPLEMENTED: u16 = 501;
    #[classattr]
    const BAD_GATEWAY: u16 = 502;
    #[classattr]
    const SERVICE_UNAVAILABLE: u16 = 503;
    #[classattr]
    const GATEWAY_TIMEOUT: u16 = 504;
    #[classattr]
    const HTTP_VERSION_NOT_SUPPORTED: u16 = 505;
    #[classattr]
    const VARIANT_ALSO_NEGOTIATES: u16 = 506;
    #[classattr]
    const INSUFFICIENT_STORAGE: u16 = 507;
    #[classattr]
    const LOOP_DETECTED: u16 = 508;
    #[classattr]
    const NOT_EXTENDED: u16 = 510;
    #[classattr]
    const NETWORK_AUTHENTICATION_REQUIRED: u16 = 511;

    // Lowercase aliases for all status codes
    #[classattr]
    fn r#continue() -> u16 {
        100
    }
    #[classattr]
    fn switching_protocols() -> u16 {
        101
    }
    #[classattr]
    fn processing() -> u16 {
        102
    }
    #[classattr]
    fn early_hints() -> u16 {
        103
    }
    #[classattr]
    fn ok() -> u16 {
        200
    }
    #[classattr]
    fn created() -> u16 {
        201
    }
    #[classattr]
    fn accepted() -> u16 {
        202
    }
    #[classattr]
    fn non_authoritative_information() -> u16 {
        203
    }
    #[classattr]
    fn no_content() -> u16 {
        204
    }
    #[classattr]
    fn reset_content() -> u16 {
        205
    }
    #[classattr]
    fn partial_content() -> u16 {
        206
    }
    #[classattr]
    fn multi_status() -> u16 {
        207
    }
    #[classattr]
    fn already_reported() -> u16 {
        208
    }
    #[classattr]
    fn im_used() -> u16 {
        226
    }
    #[classattr]
    fn multiple_choices() -> u16 {
        300
    }
    #[classattr]
    fn moved_permanently() -> u16 {
        301
    }
    #[classattr]
    fn found() -> u16 {
        302
    }
    #[classattr]
    fn see_other() -> u16 {
        303
    }
    #[classattr]
    fn not_modified() -> u16 {
        304
    }
    #[classattr]
    fn use_proxy() -> u16 {
        305
    }
    #[classattr]
    fn temporary_redirect() -> u16 {
        307
    }
    #[classattr]
    fn permanent_redirect() -> u16 {
        308
    }
    #[classattr]
    fn bad_request() -> u16 {
        400
    }
    #[classattr]
    fn unauthorized() -> u16 {
        401
    }
    #[classattr]
    fn payment_required() -> u16 {
        402
    }
    #[classattr]
    fn forbidden() -> u16 {
        403
    }
    #[classattr]
    fn not_found() -> u16 {
        404
    }
    #[classattr]
    fn method_not_allowed() -> u16 {
        405
    }
    #[classattr]
    fn not_acceptable() -> u16 {
        406
    }
    #[classattr]
    fn proxy_authentication_required() -> u16 {
        407
    }
    #[classattr]
    fn request_timeout() -> u16 {
        408
    }
    #[classattr]
    fn conflict() -> u16 {
        409
    }
    #[classattr]
    fn gone() -> u16 {
        410
    }
    #[classattr]
    fn length_required() -> u16 {
        411
    }
    #[classattr]
    fn precondition_failed() -> u16 {
        412
    }
    #[classattr]
    fn payload_too_large() -> u16 {
        413
    }
    #[classattr]
    fn uri_too_long() -> u16 {
        414
    }
    #[classattr]
    fn unsupported_media_type() -> u16 {
        415
    }
    #[classattr]
    fn range_not_satisfiable() -> u16 {
        416
    }
    #[classattr]
    fn expectation_failed() -> u16 {
        417
    }
    #[classattr]
    fn im_a_teapot() -> u16 {
        418
    }
    #[classattr]
    fn misdirected_request() -> u16 {
        421
    }
    #[classattr]
    fn unprocessable_entity() -> u16 {
        422
    }
    #[classattr]
    fn locked() -> u16 {
        423
    }
    #[classattr]
    fn failed_dependency() -> u16 {
        424
    }
    #[classattr]
    fn too_early() -> u16 {
        425
    }
    #[classattr]
    fn upgrade_required() -> u16 {
        426
    }
    #[classattr]
    fn precondition_required() -> u16 {
        428
    }
    #[classattr]
    fn too_many_requests() -> u16 {
        429
    }
    #[classattr]
    fn request_header_fields_too_large() -> u16 {
        431
    }
    #[classattr]
    fn unavailable_for_legal_reasons() -> u16 {
        451
    }
    #[classattr]
    fn internal_server_error() -> u16 {
        500
    }
    #[classattr]
    fn not_implemented() -> u16 {
        501
    }
    #[classattr]
    fn bad_gateway() -> u16 {
        502
    }
    #[classattr]
    fn service_unavailable() -> u16 {
        503
    }
    #[classattr]
    fn gateway_timeout() -> u16 {
        504
    }
    #[classattr]
    fn http_version_not_supported() -> u16 {
        505
    }
    #[classattr]
    fn variant_also_negotiates() -> u16 {
        506
    }
    #[classattr]
    fn insufficient_storage() -> u16 {
        507
    }
    #[classattr]
    fn loop_detected() -> u16 {
        508
    }
    #[classattr]
    fn not_extended() -> u16 {
        510
    }
    #[classattr]
    fn network_authentication_required() -> u16 {
        511
    }
}
