use futures::StreamExt;
use lapin::{
    Channel, Connection, Queue,
    options::{BasicAckOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, LongString, ShortString},
};

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, DeployReq},
    controller::queue::lapin::Lapin,
};

pub struct PullQueue {
    channel: Channel,
    queue: Queue,
}

impl PullQueue {
    pub async fn new(lapin_conn: Lapin) -> Result<Self, AppError> {
        let connection = lapin_conn.get_connection().await;

        let channel = connection
            .create_channel()
            .await
            .map_err(|e| AppError::ChannelError(e.to_string()))?;

        let mut queue_args = FieldTable::default();
        queue_args.insert(
            ShortString::from("x-queue-type"),
            AMQPValue::LongString(LongString::from("quorum")),
        );

        let queue = channel
            .queue_declare(
                ShortString::from("pull_queue"),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                queue_args,
            )
            .await
            .map_err(|e| AppError::QueueError(e.to_string()))?;

        Ok(Self { channel, queue })
    }

    pub async fn publish(&self, deploy_details: &DeployReq) -> Result<(), AppError> {
        let message = serde_json::to_string(deploy_details)
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        self.channel
            .basic_publish(
                ShortString::from(""),
                ShortString::from("pull_queue"),
                Default::default(),
                message.as_bytes(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        println!("Message published");

        Ok(())
    }

    pub async fn consume(&self) -> Result<DeployReq, AppError> {
        let mut consumer = self
            .channel
            .basic_consume(
                ShortString::from("pull_queue"),
                ShortString::from("pull_queue"),
                Default::default(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.map_err(|e| AppError::LapinError(e.to_string()))?;

            let data = serde_json::from_slice::<DeployReq>(&delivery.data)
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            delivery
                .ack(BasicAckOptions::default())
                .await
                .map_err(|e| AppError::LapinError(e.to_string()))?;

            return Ok(data);
        }

        Err(AppError::QueueError("No message received".to_string()))
    }
}
