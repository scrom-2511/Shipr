use futures::stream::StreamExt;
use lapin::{
    Channel, Connection, Queue,
    options::BasicAckOptions,
    types::{AMQPValue, FieldTable, LongString, ShortString},
};

use crate::{app_errors::AppError, app_types::DeployDetails, services::lapin::lapin::Lapin};

struct BuildQueue<'a> {
    connection: &'a Connection,
    channel: Channel,
    queue: Queue,
}

impl<'a> BuildQueue<'a> {
    pub async fn new(lapin_conn: &'a Lapin) -> Result<Self, AppError> {
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
                ShortString::from("build_queue"),
                Default::default(),
                queue_args,
            )
            .await
            .map_err(|e| AppError::QueueError(e.to_string()))?;

        Ok(Self {
            connection,
            channel,
            queue,
        })
    }

    pub async fn publish(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let message = serde_json::to_string(deploy_details)
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        self.channel
            .basic_publish(
                ShortString::from("build_queue"),
                ShortString::from("build_queue"),
                Default::default(),
                message.as_bytes(),
                Default::default(),
            )
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        Ok(())
    }
}
