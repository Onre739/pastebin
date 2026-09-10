use tokio::net::TcpListener;
use std::{sync::{Arc, Mutex}, env};
use pastebin::store;
use pastebin::routes;

#[tokio::main]
async fn main() {

    println!("Starting server...");

    // Load .env
    let _ = dotenvy::dotenv();
    
    let max_pastes: usize = env::var("MAX_PASTES")
        .expect("MAX_PASTES must be defined in .env!")
        .parse()
        .expect("MAX_PASTES must be valid number (usize)");
    
    let max_paste_size: usize = env::var("MAX_PASTE_SIZE")
        .expect("MAX_PASTE_SIZE must be defined in .env!")
        .parse()
        .expect("MAX_PASTE_SIZE must be valid number (usize)");

    let max_file_size: usize = env::var("MAX_FILE_SIZE")
        .expect("MAX_FILE_SIZE must be defined in .env!")
        .parse()
        .expect("MAX_FILE_SIZE must be valid number (usize)");

    // Create app state
    let paste_store =  Arc::new(Mutex::new(store::PasteStore::new(max_pastes, max_paste_size, max_file_size)));
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
