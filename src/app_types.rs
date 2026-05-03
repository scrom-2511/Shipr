use core::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct DeployDetails {
    pub url: String,
    pub branch: String,
    pub install_commands: Vec<String>,
    pub build_commands: Vec<String>,
    pub project_id: String,
    pub home_dir: String,
    pub dist_dir: String,
    pub presigned_upload_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct RunDetails {
    pub presigned_download_url: String,
    pub run_command: String,
    pub project_id: String,
}

pub enum JobType {
    Deploy,
    Run,
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

#[derive(Deserialize)]
pub struct InstallationEvent {
    pub action: String,
    pub installation: Installation,
    pub repositories: Repository,
}

#[derive(Deserialize)]
pub struct Installation {
    pub id: u64,
    pub account: Account,
}

#[derive(Deserialize)]
pub struct Account {
    pub login: String,
}

#[derive(Deserialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Deserialize)]
pub struct PushEvent {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub before: String,
    pub after: String,
    pub repository: Repository,
    pub pusher: Pusher,
    pub forced: bool,
    pub created: bool,
    pub deleted: bool,
    pub compare: String,
    pub commits: Vec<Commit>,
    pub head_commit: Commit,
}

#[derive(Deserialize)]
pub struct Pusher {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub timestamp: String,
    pub url: String,
    pub modified: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum EventType {
    Install(InstallationEvent),
    Push(PushEvent),
}
