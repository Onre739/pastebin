use tokio::net::TcpListener;
use uuid::Uuid;
use std::sync::{Arc, Mutex};

mod routes;
mod render;
mod model;
mod store;

#[tokio::main]
async fn main() {

    println!("Starting server...");

    let state = Arc::new(Mutex::new( store::PasteStore { pastes: Vec::new() } ));

    let app = routes::create_router(state);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Server successfully started at http://127.0.0.1:3000");

    // Start axum serveru
    axum::serve(listener, app)
        .await
        .unwrap();

}
