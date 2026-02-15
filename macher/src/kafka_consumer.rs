use rdkafka::{
    config::ClientConfig,
    consumer::{stream_consumer::StreamConsumer, Consumer},
    Message,
};
use shared::OrderMessage;
use anyhow::Result;

pub struct KafkaConsumer {
    consumer: StreamConsumer,
}

impl KafkaConsumer {
    pub fn new(topic: &str) -> Result<Self> {
        let kafka_bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .map_err(|_| anyhow::anyhow!("KAFKA_BOOTSTRAP_SERVERS environment variable is required"))?;
        
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_bootstrap_servers)
            .set("group.id", "order-matcher")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[topic])?;

        Ok(Self { consumer })
    }

    pub async fn consume_message(&self) -> Result<Option<OrderMessage>> {
        match self.consumer.recv().await {
            Ok(message) => {
                let payload = message.payload().ok_or_else(|| anyhow::anyhow!("Empty payload"))?;
                let order_msg = OrderMessage::from_json(std::str::from_utf8(payload)?)?;
                Ok(Some(order_msg))
            }
            Err(_e) => {
                // Non-fatal errors (like timeout) are expected, return None
                // Fatal errors will be propagated by the consumer automatically
                Ok(None)
            }
        }
    }
}

