use axum::{
    Json, Router, extract::{Path,State, Form}, response::{IntoResponse}, routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use crate::{model::{AppError, MimeKind}, store::AppState};
use crate::render;

#[derive(Deserialize)]
pub struct CreatePasteDto {
    pub name: String,
    pub content: String,
    pub mimetype: MimeKind,
}

pub fn create_router(state: AppState) -> Router {
    let app = Router::new()
        .route("/", get(get_home))
        .route("/paste/json", post(post_paste_json))
        .route("/paste/form", post(post_paste_form))
        .route("/paste/{uuid}", get(get_paste))
        .with_state(state);
    app
}

async fn get_home(State(state): State<AppState>) 
-> Result<impl IntoResponse, AppError> {
    let paste_store = state.paste_store.lock().unwrap();
     
    let html = render::render_home_page(&paste_store)?;
    Ok(html)
}

async fn post_paste_json(State(state): State<AppState>, Json(payload): Json<CreatePasteDto>) 
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(payload.name, payload.content.into_bytes(), payload.mimetype)?;
    
    Ok(Json(id))
}

async fn post_paste_form(State(state): State<AppState>, Form(form): Form<CreatePasteDto>) 
-> Result <impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = paste_store.insert(form.name, form.content.into_bytes(), form.mimetype)?;

    Ok(Json(id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<AppState>) 
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let style_store = &state.style_store;

    let paste = paste_store.record_view(uuid)?;
    render::render_paste_page(&paste, style_store)
}
