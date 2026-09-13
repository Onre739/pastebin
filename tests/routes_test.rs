use tower::ServiceExt;
use axum::{body, body::Body, http::{Request, header}};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::{str, sync::{Arc, Mutex}};
use uuid::Uuid;
use pastebin::{model, routes::create_router, store::AppState, store::PasteStore, store::StyleStore};

fn create_state(max_pastes: usize, max_paste_size: usize, max_file_size: usize) -> (axum::Router, Arc<Mutex<PasteStore>>) {
    let paste_store = Arc::new(Mutex::new(PasteStore::new(max_pastes, max_paste_size, max_file_size)));
    let style_store = Arc::new(StyleStore::new());

    let state = AppState { paste_store: paste_store.clone(), style_store };
    let app = create_router(state);
    (app, paste_store)
}

#[tokio::test]
async fn test_get_home() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
    
#[tokio::test]
async fn test_post_paste_json() {
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"content":"Hello, World!","mimetype":"PlainText"}"#))
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
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/form")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(r#"content=Hello, World!&mimetype=PlainText"#))
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
async fn test_post_paste_binary() {
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    // Bytes, which are not valid UTF-8 - exactly what /paste/json cannot accept
    let binary_content: Vec<u8> = vec![0u8, 159, 146, 150, 1, 2, 3, 255];
    let request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .body(Body::from(binary_content.clone()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK for binary paste creation");

    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read response body");
    let body_str = str::from_utf8(&body)
        .expect("response body should be valid UTF-8");
    let returned_id = Uuid::parse_str(body_str.trim_matches('"'))
        .expect("response body should contain a valid UUID");

    let paste_store = arc_paste_store.lock().unwrap();
    let stored = paste_store.pastes.iter().find(|p| p.id == returned_id)
        .expect("returned id should match a paste actually stored in PasteStore");
    assert!(matches!(stored.mimetype, model::MimeKind::OctetStream), "paste created via /paste/binary should have mimetype OctetStream");
    assert_eq!(stored.content, binary_content, "stored content should exactly match the raw request body, including non-UTF8 bytes");
}

#[tokio::test]
async fn test_post_paste_binary_roundtrip_via_http() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let binary_content: Vec<u8> = vec![0u8, 159, 146, 150, 1, 2, 3, 255];
    let create_request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .body(Body::from(binary_content.clone()))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), 200);

    let body = body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let id = Uuid::parse_str(str::from_utf8(&body).unwrap().trim_matches('"')).unwrap();

    let get_response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(get_response.status(), 200);
    let content_type = get_response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
    assert_eq!(content_type, "application/octet-stream");

    let downloaded = body::to_bytes(get_response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(downloaded.as_ref(), binary_content.as_slice(), "downloaded content should exactly match what was uploaded via /paste/binary");
}

#[tokio::test]
async fn test_post_paste_binary_empty() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 400, "Expected 400 Bad Request for empty binary content");
}

#[tokio::test]
async fn test_post_paste_binary_too_large() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 500);

    let content = vec![7u8; 1000]; // nad max_file_size (500 B)
    let request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .body(Body::from(content))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 413, "Expected 413 Payload Too Large for binary content over max_file_size");
}

#[tokio::test]
async fn test_get_non_existent_paste() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let response = app
        .oneshot(Request::builder().uri("/paste/123e4567-e89b-12d3-a456-426614174000").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 404, "Expected 404 for non-existent paste");
}

#[tokio::test]
async fn test_bad_mimetype() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"content":"Hello, World!","mimetype":"InvalidMimeType"}"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 422, "Expected 422 Unprocessable Entity for invalid mimetype");
}

#[tokio::test]
async fn test_payload_too_large() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let large_content = "A".repeat(2_000_000); // 2 MB content
    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"content":"{}","mimetype":"PlainText"}}"#, large_content)))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 413, "Expected 413 Payload Too Large for oversized content");
}

#[tokio::test]
async fn test_octet_stream_allows_larger_payload_than_text() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    // 5 MB - over max_paste_size (1 MiB), well under max_file_size (20 MiB)
    let large_content = "A".repeat(5_000_000);
    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"content":"{}","mimetype":"OctetStream"}}"#, large_content)))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 200, "Expected 200 OK - OctetStream should allow content larger than max_paste_size");
}

#[tokio::test]
async fn test_text_still_rejects_payload_over_paste_size_limit() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    // Same 5 MB size, but PlainText - should still be rejected by the smaller max_paste_size limit
    let large_content = "A".repeat(5_000_000);
    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"content":"{}","mimetype":"PlainText"}}"#, large_content)))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), 413, "Expected 413 - PlainText should still be capped at max_paste_size, not max_file_size");
}

#[tokio::test]
async fn test_empty_content() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let request = Request::builder()
        .method("POST")
        .uri("/paste/json")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"content":"","mimetype":"PlainText"}"#))
        .unwrap();

    let response = app
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), 400, "Expected 400 Bad Request for empty content");
}

#[tokio::test]
async fn test_get_paste_plain_text() {
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert(b"Hello, World!".to_vec(), model::MimeKind::PlainText, None).expect("Insert failed");
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
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert(b"<b>Hello, World!</b>".to_vec(), model::MimeKind::Html, None).expect("Insert failed");
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
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert(b"# Heading\n\nSome **bold** text.".to_vec(), model::MimeKind::Markdown, None).expect("Insert failed");
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
    let (app, arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let binary_content = vec![0u8, 159, 146, 150, 1, 2, 3, 255];

    let mut paste_store = arc_paste_store.lock().unwrap();
    let id = paste_store.insert(binary_content.clone(), model::MimeKind::OctetStream, None).expect("Insert failed");
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

#[tokio::test]
async fn test_post_paste_binary_preserves_file_name() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let binary_content: Vec<u8> = vec![1, 2, 3, 4];
    let create_request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .header("x-file-name-b64", BASE64.encode("photo.png"))
        .body(Body::from(binary_content))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(create_response.status(), 200);

    let body = body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let id = Uuid::parse_str(str::from_utf8(&body).unwrap().trim_matches('"')).unwrap();

    let get_response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let content_disposition = get_response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
    assert_eq!(content_disposition, "attachment; filename=\"photo.png\"", "Content-Disposition should use the file name from the X-File-Name-B64 header");
}

#[tokio::test]
async fn test_post_paste_binary_without_file_name_falls_back_to_uuid() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let binary_content: Vec<u8> = vec![1, 2, 3, 4];
    let create_request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .body(Body::from(binary_content))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    let body = body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let id = Uuid::parse_str(str::from_utf8(&body).unwrap().trim_matches('"')).unwrap();

    let get_response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let content_disposition = get_response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
    assert_eq!(content_disposition, format!("attachment; filename=\"{}.bin\"", id), "Missing X-File-Name-B64 header should fall back to {{uuid}}.bin");
}

#[tokio::test]
async fn test_post_paste_binary_sanitizes_file_name() {
    let (app, _arc_paste_store) = create_state(5, 1_048_576, 20_971_520);

    let binary_content: Vec<u8> = vec![1, 2, 3, 4];
    let create_request = Request::builder()
        .method("POST")
        .uri("/paste/binary")
        .header("content-type", "application/octet-stream")
        .header("x-file-name-b64", BASE64.encode("evil\".txt"))
        .body(Body::from(binary_content))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    let body = body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let id = Uuid::parse_str(str::from_utf8(&body).unwrap().trim_matches('"')).unwrap();

    let get_response = app
        .oneshot(Request::builder().uri(&format!("/paste/{}", id)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let content_disposition = get_response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
    assert_eq!(content_disposition, "attachment; filename=\"evil.txt\"", "Double quotes in the file name should be stripped so the header stays well-formed");
}
