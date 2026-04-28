use lapin::{Connection, ConnectionProperties};

use crate::app_errors::AppError;

pub struct Lapin {
    connection: Connection,
}

impl Lapin {
    pub async fn new() -> Result<Self, AppError> {
        let conn = Connection::connect(
            "amqp://guest:guest@localhost:5672/",
            ConnectionProperties::default(),
        )
        .await
        .map_err(|e| AppError::LapinError(e.to_string()))?;

        Ok(Self { connection: conn })
    }

    pub async fn get_connection(&self) -> &Connection {
        &self.connection
    }
}
