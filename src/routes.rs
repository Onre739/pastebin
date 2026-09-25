use axum::{
    Json, Router, body::Bytes, extract::{DefaultBodyLimit, Form, Path, State}, http::HeaderMap, response::IntoResponse, routing::{get, post},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use uuid::Uuid;
use crate::{model::{AppError, MimeKind, sanitize_file_name}, store::AppState};
use crate::render;
use crate::mcp::mcp_service;

#[derive(Deserialize)]
pub struct CreatePasteDto {
    pub content: String,
    pub mimetype: MimeKind,
}

pub fn create_router(state: AppState) -> Router {
    let (max_paste_size, max_file_size) = {
        let paste_store = state.paste_store.lock().unwrap();
        (paste_store.max_paste_size, paste_store.max_file_size)
    };
    
    let max_body_size = max_paste_size.max(max_file_size);
    let mcp_service = mcp_service(state.clone());

    let app = Router::new()
        .route("/", get(get_home))
        .route("/paste/json", post(post_paste_json))
        .route("/paste/form", post(post_paste_form))
        .route ("/paste/binary", post(post_paste_binary))
        .route("/paste/{uuid}", get(get_paste))
        .route("/paste/{uuid}/raw", get(get_paste_raw))
        .layer(DefaultBodyLimit::max(max_body_size))

        .nest_service("/mcp", mcp_service)
        .with_state(state);
    app
}

async fn get_home(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let max_file_size = state.paste_store.lock().unwrap().max_file_size;
    render::render_home_page(max_file_size)
}

async fn post_paste_json(State(state): State<AppState>, Json(payload): Json<CreatePasteDto>) 
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(payload.content.into_bytes(), payload.mimetype, None)?;

    Ok(Json(id))
}

async fn post_paste_form(State(state): State<AppState>, Form(form): Form<CreatePasteDto>)
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(form.content.into_bytes(), form.mimetype, None)?;

    Ok(Json(id))
}

async fn post_paste_binary(State(state): State<AppState>, headers: HeaderMap, body: Bytes)
-> Result<impl IntoResponse, AppError> {
    let file_name = headers.get("x-file-name-b64")
        .and_then(|v| v.to_str().ok())
        .and_then(|b64| BASE64.decode(b64).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .and_then(|name| sanitize_file_name(&name));

    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(Vec::<u8>::from(body), MimeKind::OctetStream, file_name)?;

    Ok(Json(id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<AppState>)
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let style_store = &state.style_store;

    let paste = paste_store.record_view(uuid)?;
    render::render_paste_page(paste, style_store)
}

async fn get_paste_raw(Path(uuid): Path<Uuid>, State(state): State<AppState>)
-> Result<impl IntoResponse, AppError> {
    let paste_store = state.paste_store.lock().unwrap();

    // Use the read-only `get` method to avoid incrementing hits/last_seen_tick for raw fetches.
    let paste = paste_store.get(uuid)?;
    Ok(render::render_octet_stream_raw(paste))
}
