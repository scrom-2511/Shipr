use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Serialize;
use sqlx::FromRow;

use crate::{
    app::{controllers::ApiResponse, db::DbPool, middlewares::AuthMiddleware},
    app_errors::AppError,
};

#[derive(Debug, Serialize, FromRow)]
struct GithubRepo {
    id: i32,
    repo_name: String,
}

#[derive(Debug, Serialize)]
struct GetGithubAppInstalledReposResponse {
    repos: Vec<GithubRepo>,
}

pub async fn get_github_app_installed_repos_controller(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let query = r#"SELECT id, repo_name FROM github_repos WHERE user_id = $1"#;

    let repos: Vec<GithubRepo> = sqlx::query_as(query)
        .bind(user_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Repos fetched successfully".to_string(),
        data: Some(GetGithubAppInstalledReposResponse { repos }),
    }))
}
