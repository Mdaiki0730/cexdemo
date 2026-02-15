use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use shared::OrderMessage;
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

    pub async fn send_order(&self, order: OrderMessage) -> anyhow::Result<()> {
        let json = order.to_json()?;
        // Use pair as key to ensure orders for the same pair go to the same partition, guaranteeing order
        let key = order.pair.clone();
        let record = FutureRecord::to("orders")
            .key(&key)
            .payload(&json);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(_) => Ok(()),
            Err((e, _)) => Err(anyhow::anyhow!("Failed to send message: {}", e)),
        }
    }
}

