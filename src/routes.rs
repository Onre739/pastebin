use axum::{
    Json, Router, extract::{Path,State}, http::StatusCode, response::{Html, IntoResponse, Redirect}, routing::{get, post},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::{model, store};
use crate::render;
use crate::store::PasteStore;

pub fn create_router(state: Arc<Mutex<PasteStore>>) -> Router {
    let app = Router::new()
        .route("/", get(get_home))
        .route("/paste", post(post_paste))
        .route("/paste/{uuid}", get(get_paste))
        .with_state(state);
    app
}

async fn get_home(State(state): State<Arc<Mutex<PasteStore>>>) -> impl IntoResponse {
    
    let store = state.lock().unwrap();
    
    let html_content = render::render_homepage(store.pastes.clone());
    Html(html_content)
}

async fn post_paste(State(state): State<Arc<Mutex<PasteStore>>>, Json(payload): Json<model::CreatePasteDto>) 
-> impl IntoResponse {
    
    let id = Uuid::new_v4();
    let paste = model::Paste {
        id,
        content: payload.content.into_bytes(),
        mimetype: payload.mimetype,
        hits: 0,
        last_seen_tick: 0
    };

    println!("Paste: {:#?}", paste);
    
    let mut store = state.lock().unwrap();
    store.pastes.push(paste);
    
    Redirect::to(&format!("/paste/{}", id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<Arc<Mutex<PasteStore>>>) -> impl IntoResponse {

    let store = state.lock().unwrap();
    let paste = store.pastes.iter().find(|p| p.id == uuid);
    
    match paste {
        // Místo Ok a Err se dá použít .into_response()
        Some(paste) => Ok(render::get_paste_by_uuid(paste.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }

}
