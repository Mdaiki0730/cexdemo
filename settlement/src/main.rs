mod kafka_consumer;
mod db;

use anyhow::Result;
use kafka_consumer::KafkaConsumer;
use db::SettlementDB;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    println!("Starting settlement layer...");

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable is required"))?;
    
    let db = SettlementDB::new(&database_url).await?;

    // Initialize Kafka consumer
    let consumer = KafkaConsumer::new("matched-orders")?;

    println!("Settlement layer ready, consuming matched orders...");

    // Consume matched orders and settle them
    loop {
        match consumer.consume_message().await {
            Ok(Some(matched_order)) => {
                println!("Processing matched order: buy={}, sell={}, amount={}", 
                    matched_order.buy_order_id, 
                    matched_order.sell_order_id, 
                    matched_order.amount);
                
                match db.settle_order(matched_order).await {
                    Ok(_) => {
                        println!("Successfully settled order");
                    }
                    Err(e) => {
                        eprintln!("Error settling order: {}", e);
                    }
                }
            }
            Ok(None) => {
                // No message, continue
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Err(e) => {
                eprintln!("Error consuming message: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

