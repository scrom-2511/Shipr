use crate::app_errors::AppError;
use std::process::Command;

pub fn run_script(script: Vec<&str>) -> Result<(), AppError> {
    for cmd in script {
        Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir("/home/scrom")
            .output()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}
