use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use tokio::net::TcpListener;
use uuid::Uuid;
use std::{sync::{Arc, Mutex}, env};

mod routes;
mod render;
mod model;
mod store;
mod repository;

#[tokio::main]
async fn main() {

    println!("Starting server...");

    // Load .env
    dotenvy::dotenv();
    
    let max_pastes: usize = env::var("MAX_PASTES")
        .expect("MAX_PASTES must be defined in .env!")
        .parse()
        .expect("MAX_PASTES must be valid number (usize)");
    
    // Create app state
    let paste_store =  Arc::new(Mutex::new(store::PasteStore::new(max_pastes)));
    let style_store = Arc::new(store::StyleStore::new());
    let state = store::AppState{paste_store: paste_store, style_store: style_store};

    let app = routes::create_router(state);

    let host = env::var("URL").expect("URL must be defined in .env!");
    let port = env::var("PORT").expect("PORT must be defined in .env!");
    let addr = format!("{}:{}", host, port); 

    let listener = TcpListener::bind(&addr)
        .await
        .unwrap();
    
    println!("Server successfully started at {}", &addr);

    // Start axum serveru
    axum::serve(listener, app)
        .await
        .unwrap();

}
