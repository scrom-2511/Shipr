use redis::{AsyncCommands, Client, aio::MultiplexedConnection};

use crate::app_errors::AppError;

#[derive(Clone)]
pub struct Redis {
    client: Client,
}

impl Redis {
    pub fn new() -> Self {
        let client = Client::open("redis://default:gQAAAAAAAYZ4AAIgcDE1YmJjNmZmN2NlZjI0OTM0YmFmNmU3MjRkZGNjMDgzOA@steady-jackal-99960.upstash.io:6379").unwrap();

        Self { client }
    }

    pub fn get_client(&self) -> Client {
        self.client.clone()
    }

    pub async fn get_conn(&self) -> Result<MultiplexedConnection, AppError> {
        let conn = self.client.get_multiplexed_async_connection().await?;

        Ok(conn)
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let _: () = conn.set(key, value).await?;

        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<String, AppError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let value = conn.get(key).await?;

        Ok(value)
    }
}
