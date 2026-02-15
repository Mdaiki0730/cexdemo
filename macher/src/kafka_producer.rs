use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use shared::MatchedOrder;
use std::time::Duration;

pub struct KafkaProducer {
    producer: FutureProducer,
}

impl KafkaProducer {
    pub async fn new() -> anyhow::Result<Self> {
        let kafka_bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .map_err(|_| anyhow::anyhow!("KAFKA_BOOTSTRAP_SERVERS environment variable is required"))?;
        
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create()?;

        Ok(Self { producer })
    }

    pub async fn send_matched_order(&self, matched: MatchedOrder) -> anyhow::Result<()> {
        let json = matched.to_json()?;
        let key = format!("{}-{}", matched.buy_order_id, matched.sell_order_id);
        let record = FutureRecord::to("matched-orders")
            .key(&key)
            .payload(&json);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(_) => {
                println!("Sent matched order: buy={}, sell={}, amount={}", 
                    matched.buy_order_id, matched.sell_order_id, matched.amount);
                Ok(())
            }
            Err((e, _)) => Err(anyhow::anyhow!("Failed to send matched order: {}", e)),
        }
    }
}

