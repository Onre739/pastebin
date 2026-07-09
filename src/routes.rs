use axum::{
    routing::{get, post},
    Router,
    response::{Html, IntoResponse},
};

use crate::render;

pub fn create_router() -> Router {
    let app = Router::new()
        .route("/", get(render::render_homepage))
        .route("/paste", post(post_paste))
        .route("/paste/{uuid}", get(get_paste_by_uuid));
    app
}

async fn get_hello() -> Html<&'static str> {
    //Html::from("Hello, world!")
    Html("<h1>Vítejte na Rust backendu!</h1>")
}

async fn post_paste() -> impl IntoResponse {
    // Zde bude logika pro zpracování POST požadavku na /paste
    // Například získání dat z těla požadavku, uložení do databáze atd.
    "Paste received"
}

async fn get_paste_by_uuid() -> impl IntoResponse {
    // Zde bude logika pro získání paste podle UUID
    // Například vyhledání v databázi a vrácení obsahu
    "Paste content for given UUID"
}