use axum::{
    Json, Router, extract::{Path,State, Form}, http::{StatusCode, header}, response::{Html, IntoResponse, Redirect}, routing::{get, post},
};
use std::{option, result, sync::{Arc, Mutex}};
use uuid::Uuid;
use crate::model::{self, AppError};
use crate::store::PasteStore;
use crate::repository;

pub fn create_router(state: Arc<Mutex<PasteStore>>) -> Router {
    let app = Router::new()
        .route("/", get(get_home))
        .route("/paste/json", post(post_paste_json))
        .route("/paste/form", post(post_paste_form))
        .route("/paste/{uuid}", get(get_paste))
        .with_state(state);
    app
}

async fn get_home(State(state): State<Arc<Mutex<PasteStore>>>) 
-> Result<impl IntoResponse, AppError> {
    let store = state.lock().unwrap();
     
    let html = repository::process_homepage(&store);
    Ok(html)
}

async fn post_paste_json(State(state): State<Arc<Mutex<PasteStore>>>, Json(payload): Json<model::CreatePasteDto>) 
-> Result<impl IntoResponse, AppError> {
    let mut store = state.lock().unwrap();
    let id = repository::process_post(&mut store, payload.name, payload.content, payload.mimetype);
    
    Ok(Json(id))
}

async fn post_paste_form(State(state): State<Arc<Mutex<PasteStore>>>, Form(form): Form<model::CreatePasteDto>) 
-> Result <impl IntoResponse, AppError> {
    let mut store = state.lock().unwrap();
    let id = repository::process_post(&mut store, form.name, form.content, form.mimetype);

    Ok(Json(id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<Arc<Mutex<PasteStore>>>) 
-> Result<impl IntoResponse, AppError> {
    println!("Looking for paste with UUID: {}", uuid);

    let mut store = state.lock().unwrap();
    repository::process_paste(&mut store, uuid)
}
