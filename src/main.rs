use tracing_subscriber;
use manifestor::api;
use tracing::{info};
use std::{env, net::SocketAddr};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT debe ser un número entero válido");

    // 2. Construir dirección con tipo seguro
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let app = api::create_router();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server started at {:?}", &listener.local_addr().unwrap().ip());
    axum::serve(listener, app).await?;
    Ok(())
}
