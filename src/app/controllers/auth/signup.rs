use crate::app::db::DbPool;
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct SignupRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponse {
    pub message: String,
}

pub async fn signup_controller(
    pool: web::Data<DbPool>,
    body: web::Json<SignupRequest>,
) -> Result<HttpResponse, AppError> {
    let signup = body.into_inner();

    signup
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let hashed_password =
        bcrypt::hash(&signup.password, 10).map_err(|_| AppError::PasswordHashFailed)?;

    let query = "INSERT INTO users (name, email, password) VALUES ($1, $2, $3)";

    let result = sqlx::query(query)
        .bind(&signup.name)
        .bind(&signup.email)
        .bind(&hashed_password)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(HttpResponse::Created().json(SignupResponse {
            message: "User created successfully".to_string(),
        })),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}
