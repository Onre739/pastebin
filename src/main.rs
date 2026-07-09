use tokio::net::TcpListener;
use uuid::Uuid;

mod routes;
mod render;

#[tokio::main]
async fn main() {

    println!("Starting server...");

    let app = routes::create_router();

    // Vrací to Result, takže v produkčním kódu bys použil `match` nebo `?`
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Server successfully started at http://127.0.0.1:3000");

    // Start axum serveru
    axum::serve(listener, app)
        .await
        .unwrap();


    let id = Uuid::new_v4();


}
