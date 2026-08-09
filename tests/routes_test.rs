use tower::ServiceExt;
use axum::{body, body::Body, http::{Request, header}};
use std::{str, sync::{Arc, Mutex}};
use uuid::Uuid;
use pastebin::{model, routes::create_router, store::AppState, store::PasteStore, store::StyleStore};

fn create_state(max_pastes: usize, max_paste_size: usize) -> (axum::Router, Arc<Mutex<PasteStore>>) {
    let paste_store = Arc::new(Mutex::new(PasteStore::new(max_pastes, max_paste_size)));
    let style_store = Arc::new(StyleStore::new());

    let state = AppState { paste_store: paste_store.clone(), style_store };
    let app = create_router(state);
    (app, paste_store)
}

#[tokio::test]
async fn test_get_home() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
    
#[tokio::test]
async fn test_post_paste_json() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Test Paste","content":"Hello, World!","mimetype":"PlainText"}"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for paste creation");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");
    let returned_id = Uuid::parse_str(body_str.trim_matches('"'))
        .expect("response body should contain a valid UUID");

    let paste_store = arc_paste_store.lock().unwrap();
    assert!(
        paste_store.pastes.iter().any(|p| p.id == returned_id),
        "returned id should match a paste actually stored in PasteStore"
    );
}

#[tokio::test]
async fn test_post_paste_form() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/form")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(r#"name=Test Paste&content=Hello, World!&mimetype=PlainText"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for paste creation");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");
    let returned_id = Uuid::parse_str(body_str.trim_matches('"'))
        .expect("response body should contain a valid UUID");

    let paste_store = arc_paste_store.lock().unwrap();
    assert!(
        paste_store.pastes.iter().any(|p| p.id == returned_id),
        "returned id should match a paste actually stored in PasteStore"
    );
}

#[tokio::test]
async fn test_get_non_existent_paste() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576);

    let response = app
        .oneshot(Request::builder().uri("/paste/123e4567-e89b-12d3-a456-426614174000").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 404, "Expected 404 for non-existent paste");
}

#[tokio::test]
async fn test_bad_mimetype() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Test Paste","content":"Hello, World!","mimetype":"InvalidMimeType"}"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 422, "Expected 422 Unprocessable Entity for invalid mimetype");
}

#[tokio::test]
async fn test_payload_too_large() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576);

    let large_content = "A".repeat(2_000_000); // 2 MB content
    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"name":"Test Paste","content":"{}","mimetype":"PlainText"}}"#, large_content)))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 413, "Expected 413 Payload Too Large for oversized content");
}

#[tokio::test]
async fn test_empty_content() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"Test Paste","content":"","mimetype":"PlainText"}"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 400, "Expected 400 Bad Request for empty content");
}

#[tokio::test]
async fn test_get_paste_plain_text() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert("Test Paste".to_string(), b"Hello, World!".to_vec(), model::MimeKind::PlainText).expect("Insert failed");
    drop(paste_store); // Necessary to release the lock before making the request

    let response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for existing paste");

    let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");

    assert!(body_str.contains("Hello, World!"), "Response body should contain the paste content");
}

#[tokio::test]
async fn test_get_paste_html() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert("Test Paste".to_string(), b"<b>Hello, World!</b>".to_vec(), model::MimeKind::Html).expect("Insert failed");
    drop(paste_store); // Necessary to release the lock before making the request

    let response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for existing paste");

    let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
    assert_eq!(content_type, "text/html; charset=utf-8");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");

    assert!(body_str.contains("<b>Hello, World!</b>"), "Response body should contain the raw HTML content unchanged");
}

#[tokio::test]
async fn test_get_paste_markdown() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert("Test Paste".to_string(), b"# Heading\n\nSome **bold** text.".to_vec(), model::MimeKind::Markdown).expect("Insert failed");
    drop(paste_store); // Necessary to release the lock before making the request

    let response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for existing paste");

    let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
    assert_eq!(content_type, "text/html; charset=utf-8");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");

    assert!(body_str.contains("<h1>Heading</h1>"), "Markdown heading should be converted to HTML");
    assert!(body_str.contains("<strong>bold</strong>"), "Markdown bold text should be converted to HTML");
}

#[tokio::test]
async fn test_get_paste_octet_stream() {
    let (app, arc_paste_store) = create_state(5, 1_048_576);

    let binary_content = vec![0u8, 159, 146, 150, 1, 2, 3, 255];

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert("Test Paste".to_string(), binary_content.clone(), model::MimeKind::OctetStream).expect("Insert failed");
    drop(paste_store); // Necessary to release the lock before making the request

    let response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for existing paste");

    let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
    assert_eq!(content_type, "application/octet-stream");

    let content_disposition = response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
    assert!(content_disposition.contains("attachment"), "Content-Disposition should mark the response as an attachment");
    assert!(content_disposition.contains(&id.to_string()), "Content-Disposition filename should contain the paste id");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");

    assert_eq!(body.as_ref(), binary_content.as_slice(), "Response body should contain the raw binary content unchanged");
}
