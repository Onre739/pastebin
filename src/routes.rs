use axum::{
    Json, Router, extract::{Path,State, Form}, http::{StatusCode, header}, response::{Html, IntoResponse, Redirect}, routing::{get, post},
};
use std::{option, result, sync::{Arc, Mutex}};
use uuid::Uuid;
use crate::{model::{self, AppError}, store::{AppState, StyleStore}};
use crate::store::PasteStore;
use crate::repository;

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
     
    let html = repository::process_homepage(&paste_store)?;
    Ok(html)
}

async fn post_paste_json(State(state): State<AppState>, Json(payload): Json<model::CreatePasteDto>) 
-> Result<impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = repository::process_post(&mut paste_store, payload.name, payload.content, payload.mimetype)?;
    
    Ok(Json(id))
}

async fn post_paste_form(State(state): State<AppState>, Form(form): Form<model::CreatePasteDto>) 
-> Result <impl IntoResponse, AppError> {
    let mut paste_store = state.paste_store.lock().unwrap();
    let id = repository::process_post(&mut paste_store, form.name, form.content, form.mimetype)?;

    Ok(Json(id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<AppState>) 
-> Result<impl IntoResponse, AppError> {
    println!("Looking for paste with UUID: {}", uuid);

    let mut paste_store = state.paste_store.lock().unwrap();

    let paste_store = &mut paste_store;
    let style_store = &state.style_store;

    repository::process_paste(paste_store, style_store, uuid)
}
