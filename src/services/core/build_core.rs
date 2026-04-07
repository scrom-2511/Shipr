use tokio::process::Command;

use crate::{app_errors::AppError, app_types::DeployDetails};

pub struct BuildCore;

impl BuildCore {
    pub fn new() -> Self {
        Self {}
    }
}
