mod handlers;
mod kafka_producer;
mod db;

use axum::{
    routing::{get, post},
    Router,
};
use handlers::*;
use sea_orm::Database;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub kafka_producer: Arc<kafka_producer::KafkaProducer>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize database connection
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is required"))?;
    
    let db = Database::connect(&database_url).await?;

    // Initialize Kafka producer
    let kafka_producer = Arc::new(kafka_producer::KafkaProducer::new().await?);

    let app_state = AppState {
        db,
        kafka_producer,
    };

    // Build router
    let app = Router::new()
        .route("/api/exchange/orders", post(create_order))
        .route("/api/order_books", get(get_order_books))
        .route("/api/order_books/executed", get(get_executed_orders))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let server_address = std::env::var("SERVER_ADDRESS")
        .map_err(|_| anyhow::anyhow!("SERVER_ADDRESS environment variable is required"))?;
    let listener = tokio::net::TcpListener::bind(&server_address).await?;
    println!("Server running on http://{}", server_address);
    axum::serve(listener, app).await?;

    Ok(())
}

