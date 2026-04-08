use std::fs;

use tokio::process::Command;

use crate::app_errors::AppError;

struct Firecracker;

impl Firecracker {
    pub fn new() -> Self {
        Self
    }
}
