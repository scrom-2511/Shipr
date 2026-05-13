use crate::app::db::DbPool;
use crate::app::models::Project;
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GetAllProjectsResponse {
    pub projects: Vec<Project>,
}

pub async fn get_all_projects(pool: web::Data<DbPool>) -> Result<HttpResponse, AppError> {
    let query = "SELECT * FROM projects ORDER BY created_at DESC";

    let projects = sqlx::query_as::<_, Project>(query)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(GetAllProjectsResponse { projects }))
}
