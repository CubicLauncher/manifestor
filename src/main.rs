use tracing_subscriber;
use manifestor::api;
use tracing::{info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app = api::create_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server started at {:?}", &listener.local_addr().unwrap().ip());
    axum::serve(listener, app).await?;
    Ok(())
}
