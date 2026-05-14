use crate::app::db::DbPool;
use crate::app::models::Project;
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GetAllProjectsResponse {
    pub projects: Vec<Project>,
}

#[derive(serde::Deserialize)]
pub struct GetProjectsQuery {
    pub user_id: i32,
}

pub async fn get_all_projects(
    pool: web::Data<DbPool>,
    query: web::Query<GetProjectsQuery>,
) -> Result<HttpResponse, AppError> {
    let query_sql = "SELECT * FROM projects WHERE user_id = $1 ORDER BY created_at DESC";

    let projects = sqlx::query_as::<_, Project>(query_sql)
        .bind(query.user_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(GetAllProjectsResponse { projects }))
}
