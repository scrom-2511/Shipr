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
}
