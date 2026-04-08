use tokio::process::Command;

use crate::app_errors::AppError;

pub struct Firecracker;

impl Firecracker {
    pub fn new() -> Self {
        Self
    }

    async fn run_cmds(cmds: &Vec<String>) -> Result<(), AppError> {
        if cmds.len() == 0 {
            return Ok(());
        }

        for cmd in cmds {
            let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();

            print!("{:?}", cmd_parts);

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

    pub async fn start() -> Result<(), AppError> {
        let API_SOCKET = "/tmp/firecracker.socket";

        let cleanup_cmd = format!("sudo rm -f {}", API_SOCKET);

        let firecracker_path = "/home/scrom/firecracker-main";

        Self::run_cmds(&vec![cleanup_cmd]).await?;

        Command::new("sudo")
            .current_dir("/home/scrom")
            .arg(firecracker_path)
            .arg("--api-sock")
            .arg(API_SOCKET)
            .arg("--enable-pci")
            .spawn()
            .map_err(|e| AppError::CmdFailed(e.to_string()))?;

        Ok(())
    }
}
