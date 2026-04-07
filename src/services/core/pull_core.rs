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

    pub async fn pull(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let git_url = &deploy_details.url;

        Self::git_url_validator(git_url)?;

        let cwd = env::current_dir()
            .map_err(|_| AppError::InvalidGitUrl)?
            .join("pull");

        fs::create_dir_all(&cwd).map_err(|e| AppError::DirCreationFailed(e.to_string()))?;

        let output = Command::new("git")
            .arg("clone")
            .arg(git_url)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| AppError::GitCloneFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::GitCloneFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}
