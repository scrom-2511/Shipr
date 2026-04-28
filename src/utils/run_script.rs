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

pub fn run_script_vm(script: Vec<&str>) -> Result<(), AppError> {
    for cmd in script {
        Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir("/root")
            .output()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;
    }

    Ok(())
}
