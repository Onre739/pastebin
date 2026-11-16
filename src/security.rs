use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

/// CSP for every page that can show user content (`/paste/*` and anything unknown).
/// `sandbox` without `allow-scripts` means no script in a paste ever runs -
/// `<script>`, inline event handlers and `javascript:` links alike (see ADR 0016).
pub const PASTE_CSP: &str = "sandbox allow-same-origin allow-downloads; default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data: https:; frame-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'self'";

/// CSP for the home page - the only page with our own script (form, drag & drop).
pub const HOME_CSP: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'";

/// Adds security headers to every response. The policy is chosen by path:
/// the home page gets its own CSP, `/mcp` (JSON-RPC, not a document) gets none,
/// everything else gets the sandboxed paste CSP - deny by default, so a new
/// route showing user content is covered without touching this function.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));

    if path == "/" {
        headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(HOME_CSP));
        headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    } else if path != "/mcp" && !path.starts_with("/mcp/") {
        headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(PASTE_CSP));
    }

    response
}
