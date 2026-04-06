use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Lapin error: {0}")]
    LapinError(String),

    #[error("Channel creation error: {0}")]
    ChannelError(String),

    #[error("Queue declaration error: {0}")]
    QueueError(String),
}
