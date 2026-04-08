use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Lapin error: {0}")]
    LapinError(String),

    #[error("Channel creation error: {0}")]
    ChannelError(String),

    #[error("Queue declaration error: {0}")]
    QueueError(String),

    #[error("Invalid git url")]
    InvalidGitUrl,

    #[error("Gitclone failed: {0}")]
    GitCloneFailed(String),

    #[error("Error creating dir: {0}")]
    DirCreationFailed(String),

    #[error("Running command failed: {0}")]
    CmdFailed(String),

    #[error("Failed getting the current working directory: {0}")]
    CurrentWorkingDirUnavailable(String),

    #[error("Failed to start firecracker: {0}")]
    StartingFirecrackerFailed(String),
}
