use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
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
    pub dist_dir: String,
    pub home_dir: String,
    pub url: String,
    pub user_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
