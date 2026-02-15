mod matcher;
mod kafka_consumer;
mod kafka_producer;

use anyhow::Result;
use matcher::OrderMatcher;
use kafka_consumer::KafkaConsumer;
use kafka_producer::KafkaProducer;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    println!("Starting order matcher...");

    // Initialize matcher
    let matcher = Arc::new(Mutex::new(OrderMatcher::new()));

    // Initialize Kafka consumer
    let consumer = KafkaConsumer::new("orders")?;

    // Initialize Kafka producer for matched orders
    let producer = Arc::new(KafkaProducer::new().await?);

    // Clone for async task
    let matcher_clone = matcher.clone();
    let producer_clone = producer.clone();

    // Start consuming orders
    tokio::spawn(async move {
        loop {
            match consumer.consume_message().await {
                Ok(Some(order_msg)) => {
                    println!("Received order: {:?}", order_msg.order_id);
                    
                    let mut matcher_guard = matcher_clone.lock().await;
                    let matched_orders = matcher_guard.match_order(order_msg).await;
                    
                    // Send matched orders to Kafka
                    for matched in matched_orders {
                        if let Err(e) = producer_clone.send_matched_order(matched).await {
                            eprintln!("Failed to send matched order: {}", e);
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
    });

    // Keep main thread alive
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
}

