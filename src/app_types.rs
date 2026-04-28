use core::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct DeployDetails {
    pub url: String,
    pub branch: String,
    pub install_commands: Vec<String>,
    pub build_commands: Vec<String>,
    pub unique_id: Uuid,
    pub home_dir: String,
    pub dist_dir: String,
    pub presigned_url: String,
}

#[derive(Clone)]
pub enum ProjectType {
    Html,
    Rust,
    React,
    Node,
    Unknown,
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProjectType::Html => "html",
            ProjectType::Rust => "rust",
            ProjectType::React => "react",
            ProjectType::Node => "node",
            ProjectType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}
