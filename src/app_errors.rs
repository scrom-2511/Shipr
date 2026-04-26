use actix_web::{HttpResponse, ResponseError, http::StatusCode};

#[derive(Debug, thiserror::Error)]
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

    #[error("VM not ready")]
    VmNotReady,

    #[error("Unknown project type")]
    UnknownProjectType,

    #[error("ID allocation failed: {0}")]
    IdAllocationFailed(String),

    #[error("Failed to get id from pool: {0}")]
    FailedToGetIdFromPool(String),

    #[error("HTTP client build failed: {0}")]
    HttpClientBuildFailed(String),

    #[error("Invalid project id: {0}")]
    InvalidProjectId(String),

    #[error("VM provisioning failed: {0}")]
    VmProvisioningFailed(String),

    #[error("Request forwarding failed: {0}")]
    RequestForwardingFailed(String),

    #[error("Response read failed: {0}")]
    ResponseReadFailed(String),

    #[error("Method conversion failed: {0}")]
    MethodConversionFailed(String),

    #[error("No available VM")]
    NoAvailableVm,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InvalidGitUrl => StatusCode::BAD_REQUEST,
            AppError::UnknownProjectType => StatusCode::BAD_REQUEST,

            AppError::IdAllocationFailed(_)
            | AppError::FailedToGetIdFromPool(_)
            | AppError::StartingFirecrackerFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,

            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}
