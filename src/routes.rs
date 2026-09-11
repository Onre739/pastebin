use axum::{
    Json, Router, body::Bytes, extract::{DefaultBodyLimit, Form, Path, State}, response::IntoResponse, routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use crate::{model::{AppError, MimeKind}, store::AppState};
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
    let id = paste_store.insert(payload.content.into_bytes(), payload.mimetype)?;

    Ok(Json(id))
}

async fn post_paste_form(State(state): State<AppState>, Form(form): Form<CreatePasteDto>)
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(form.content.into_bytes(), form.mimetype)?;

    Ok(Json(id))
}

async fn post_paste_binary(State(state): State<AppState>, body: Bytes)
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(Vec::<u8>::from(body), MimeKind::OctetStream)?;

    Ok(Json(id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<AppState>) 
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let style_store = &state.style_store;

    let paste = paste_store.record_view(uuid)?;
    render::render_paste_page(&paste, style_store)
}
