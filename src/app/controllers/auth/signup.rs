use crate::app::{controllers::ApiResponse, db::DbPool};
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use sqlx::Error;
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

    println!("{:?}", signup);

    signup
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let hashed_password =
        bcrypt::hash(&signup.password, 10).map_err(|_| AppError::InternalServerError)?;

    let query = "INSERT INTO users (name, email, password) VALUES ($1, $2, $3)";

    let result = sqlx::query(query)
        .bind(&signup.name)
        .bind(&signup.email)
        .bind(&hashed_password)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(HttpResponse::Created().json(ApiResponse::<()> {
            success: true,
            message: "User created successfully".to_string(),
            data: None,
        })),
        Err(Error::Database(db_err)) => {
            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    "23505" => return Err(AppError::UserAlreadyExists(signup.email)),
                    _ => return Err(AppError::Database(db_err.to_string())),
                }
            }
            Err(AppError::Database(db_err.to_string()))
        }
        Err(_) => Err(AppError::Database("Something went wrong".to_string())),
    }
}
