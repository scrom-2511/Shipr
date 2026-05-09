use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, config::Builder, presigning::PresigningConfig};

use crate::app_errors::AppError;

#[derive(Clone)]
pub struct S3Service {
    client: Client,
    bucket: String,
}

impl S3Service {
    pub async fn new() -> Self {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url("https://francisco-unscholarlike-punctually.ngrok-free.dev/")
            .region("us-east-1")
            .load()
            .await;

        let s3_config = Builder::from(&config).force_path_style(true).build();

        let client = Client::from_conf(s3_config);

        Self {
            client,
            bucket: String::from("shipr"),
        }
    }

    pub async fn get_presigned_upload_url(&self, key: &str) -> Result<String, AppError> {
        let key = format!("{}.zip", key);

        let presigned_req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(PresigningConfig::expires_in(Duration::from_mins(10))?)
            .await
            .map_err(aws_sdk_s3::Error::from)?;

        Ok(presigned_req.uri().to_string())
    }

    pub async fn get_presigned_download_url(&self, key: &str) -> Result<String, AppError> {
        let key = format!("{}.zip", key);

        let presigned_req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(PresigningConfig::expires_in(Duration::from_mins(10))?)
            .await
            .map_err(aws_sdk_s3::Error::from)?;

        Ok(presigned_req.uri().to_string())
    }
}
