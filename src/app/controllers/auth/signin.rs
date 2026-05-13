use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    app::{controllers::auth::generate_token, db::DbPool, models::User},
    app_errors::AppError,
};

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct SigninRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SigninResponse {
    pub message: String,
    pub token: String,
}

pub async fn signin_controller(
    pool: web::Data<DbPool>,
    body: web::Json<SigninRequest>,
) -> Result<HttpResponse, AppError> {
    let signin = body.into_inner();

    signin
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let query = "SELECT id, name, email, password, created_at FROM users WHERE email = $1";

    let user: User = sqlx::query_as(query)
        .bind(&signin.email)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_| AppError::UserNotFound)?;

    let is_valid = bcrypt::verify(&signin.password, &user.password)
        .map_err(|_| AppError::PasswordHashFailed)?;

    if !is_valid {
        return Err(AppError::InvalidCredentials);
    }

    let token = generate_token(user.id, &user.email)?;

    Ok(HttpResponse::Ok().json(SigninResponse {
        message: "Login successful".to_string(),
        token,
    }))
}
