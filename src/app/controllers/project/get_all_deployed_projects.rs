use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
struct DeployedProject {
    id: i32,
    name: String,
    url: String,
    last_deployment_time: chrono::DateTime<chrono::Utc>,
    repo_name: String,
}

#[derive(Debug, Serialize, FromRow)]
struct GetAllProjectsResponse {
    projects: Vec<DeployedProject>,
}

pub async fn get_all_projects_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let query = r#"
    SELECT
        p.id,
        p.name,
        p.url,
        p.last_deployment_time,
        gr.repo_name
    FROM projects p
    JOIN github_repos gr
    ON p.repo_id = gr.id
    WHERE p.user_id = $1
    ORDER BY p.created_at DESC
    "#;

    let projects: Vec<DeployedProject> = sqlx::query_as(query)
        .bind(user_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Projects fetched successfully".to_string(),
        data: Some(GetAllProjectsResponse { projects }),
    }))
}
