use std::fs;

use tokio::process::Command;

use crate::app_errors::AppError;

struct Firecracker;

impl Firecracker {
    pub fn new() -> Self {
        Self
    }

    pub async fn start() -> Result<(), AppError> {
        let API_SOCKET = "/tmp/firecracker.socket";

        fs::remove_file(API_SOCKET);

        let firecracker_run_cmd = format!(
            "sudo ./firecracker-main --api-sock {} --enable-pci",
            API_SOCKET
        );

        let firecracker_run_cmd_parts: Vec<&str> = firecracker_run_cmd.split_whitespace().collect();

        let output = Command::new(&firecracker_run_cmd_parts[0])
            .args(&firecracker_run_cmd_parts[1..])
            .output()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::CmdFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}
