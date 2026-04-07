use std::{env, fs, os::unix::process};

use tokio::process::Command;
use url::Url;

use crate::{app_errors::AppError, app_types::DeployDetails};

pub struct PullCore;

impl PullCore {
    pub fn new() -> Self {
        Self {}
    }
}
