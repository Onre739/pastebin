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
    
    // Loading resources
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let max_pastes: usize = env::var("MAX_PASTES")
        .expect("MAX_PASTES must be defined in .env!")
        .parse()
        .expect("MAX_PASTES must be valid number (usize)");

    // Create app state
    let state = Arc::new(Mutex::new( store::PasteStore { pastes: Vec::new(), max_pastes: max_pastes, syntax_set: ss, theme_set: ts } ));

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
