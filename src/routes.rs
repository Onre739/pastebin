use axum::{
    Json, Router, extract::Path, http::StatusCode, response::{Html, IntoResponse}, routing::{get, post},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::model;
use crate::render;
use crate::store::PasteStore;

pub fn create_router(state: Arc<Mutex<PasteStore>>) -> Router {
    let app = Router::new()
        .route("/", get(render::render_homepage))
        .route("/paste", post(post_paste))
        .route("/paste/{uuid}", get(render::get_paste_by_uuid))
        .with_state(state);
    app
}

async fn post_paste(Json(payload): Json<model::CreatePasteDto>) -> impl IntoResponse {
    
    let id = Uuid::new_v4();
    let paste = model::Paste {
        id,
        content: payload.content.into_bytes(),
        mimetype: payload.mimetype,
        hits: 0,
        last_seen_tick: 0
    };
    
    Json(paste)
}