use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DeployReq {
    pub url: String,
    pub install: Vec<String>,
    pub build: Vec<String>,
    pub branch: String,
    pub dist_dir: String,
    pub home_dir: String,
}

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
    pub owner: String,
    pub repo: String,
    pub access_token: String,
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

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct InstallationEvent {
    pub action: String,
    pub installation: Installation,
    pub repositories: Vec<Repository>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Installation {
    pub id: u64,
}

#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Deserialize)]
pub struct PushEvent {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub after: String,
    pub repository: Repository,
    pub installation: Installation,
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
