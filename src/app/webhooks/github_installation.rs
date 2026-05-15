use actix_web::{HttpResponse, web};

use crate::{
    app::{controllers::ApiResponse, db::DbPool},
    app_errors::AppError,
};

#[derive(PartialEq)]
enum GithubWebhookAction {
    Created,
    Deleted,
}

struct GithubAccount {
    login: String,
    id: i32,
}

struct GithubAppInstallation {
    id: i32,
    client_id: String,
    account: GithubAccount,
}

struct GithubRepository {
    id: i32,
    name: String,
    full_name: String,
}

struct GithubAppWebhookPayload {
    action: GithubWebhookAction,
    installation: GithubAppInstallation,
    repositories: Vec<GithubRepository>,
}

pub async fn github_webhook_installation(
    body: web::Json<GithubAppWebhookPayload>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();

    if body.action != GithubWebhookAction::Created {
        return Ok(HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "Installation not created".to_string(),
            data: None,
        }));
    }

    let owner = vec![body.installation.account.login];
    let installation_id = vec![body.installation.id];

    let query = r#"INSERT INTO github_app_installations (owner, installation_id) VALUES ($1, $2)"#;

    sqlx::query(query)
        .bind(owner)
        .bind(installation_id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Installation recorded successfully".to_string(),
        data: None,
    }))
}
