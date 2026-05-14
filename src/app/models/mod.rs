use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, prelude::Type};

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "project_status", rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Deploying,
    Inactive,
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
    pub install_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,
    pub branch: String,
    pub dist_dir: String,
    pub home_dir: String,
    pub url: String,
    pub user_id: i32,
    pub commit_hash: String,
    pub last_deployment_time: chrono::DateTime<chrono::Utc>,
    pub status: ProjectStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// {
//   id: "1",
//   name: "my-landing-page",
//   url: "my-landing-page.shipr.dev",
//   repo: "scrom/landing-page",
//   branch: "main",
//   home_dir: "/",
//   dist_dir: "/dist",
//   install_cmds: ["npm install"],
//   build_cmds: ["npm run build"],
//   run_cmds: ["npm start"],
//   status: "active",
//   lastDeployed: "2 hours ago",
//   lastDeploymentTime: "2026-05-14T10:30:00Z",
//   commitHash: "a1b2c3d4e5f6",
// }
