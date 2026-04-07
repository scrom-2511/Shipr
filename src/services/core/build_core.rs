use std::{env, fs};

use tokio::process::Command;
use uuid::Uuid;

use crate::{app_errors::AppError, app_types::DeployDetails};

pub struct BuildCore;

impl BuildCore {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run_cmds(
        cmds: &Vec<String>,
        home_dir: &str,
        unique_folder_id: &Uuid,
    ) -> Result<(), AppError> {
        if cmds.len() == 0 {
            return Ok(());
        }

        let cwd = env::current_dir()
            .map_err(|e| AppError::CurrentWorkingDirUnavailable(e.to_string()))?
            .join("pull")
            .join(unique_folder_id.to_string())
            .join(home_dir);

        for cmd in cmds {
            let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();

            let initial_part = cmd_parts
                .get(0)
                .ok_or_else(|| AppError::CmdFailed("Empty command".into()))?;

            let output = Command::new(initial_part)
                .args(&cmd_parts[1..])
                .current_dir(&cwd)
                .output()
                .await
                .map_err(|e| AppError::CmdFailed(e.to_string()))?;

            if !output.status.success() {
                return Err(AppError::CmdFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn install(deploy_details: &DeployDetails) -> Result<(), AppError> {
        Self::run_cmds(
            &deploy_details.install_commands,
            &deploy_details.home_dir,
            &deploy_details.unique_id,
        )
        .await?;

        Ok(())
    }

    pub async fn build(deploy_details: &DeployDetails) -> Result<(), AppError> {
        Self::run_cmds(
            &deploy_details.build_commands,
            &deploy_details.home_dir,
            &deploy_details.unique_id,
        )
        .await?;

        let cwd = env::current_dir()
            .map_err(|e| AppError::CurrentWorkingDirUnavailable(e.to_string()))?;

        let rename_from = cwd
            .join("pull")
            .join(&deploy_details.unique_id.to_string())
            .join(&deploy_details.home_dir)
            .join(&deploy_details.dist_dir);

        let rename_to = cwd
            .join("build")
            .join(&deploy_details.unique_id.to_string());

        fs::rename(rename_from, rename_to);

        Ok(())
    }
}
