use tokio::process::Command;

use crate::{app_errors::AppError, app_types::DeployDetails};

pub struct BuildCore;

impl BuildCore {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run_cmds(cmds: &Vec<String>) -> Result<(), AppError> {
        if cmds.len() == 0 {
            return Ok(());
        }

        for cmd in cmds {
            let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();

            let initial_part = cmd_parts
                .get(0)
                .ok_or_else(|| AppError::CmdFailed("Empty command".into()))?;

            let output = Command::new(initial_part)
                .args(&cmd_parts[1..])
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
}
