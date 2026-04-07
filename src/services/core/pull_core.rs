use std::{env, fs, os::unix::process};

use tokio::process::Command;
use url::Url;

use crate::{app_errors::AppError, app_types::DeployDetails};

pub struct PullCore;

impl PullCore {
    pub fn new() -> Self {
        Self {}
    }

    fn git_url_validator(git_url: &str) -> Result<bool, AppError> {
        if let Ok(url) = Url::parse(git_url) {
            if url.host_str() == Some("github.com") && url.scheme() == "https" {
                return Ok(true);
            }
        }

        Err(AppError::InvalidGitUrl)
    }
}
