use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::{
    app::{
        controllers::{ApiResponse, auth::generate_token},
        middlewares::AuthMiddleware,
    },
    app_errors::AppError,
};

const JWT_SECRET: &str = "shipr_jwt_secret_key_2026";

#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    user_id: i32,
    exp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateResponse {
    state: String,
}

pub async fn get_state(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let state = generate_token(user_id)?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "State generated successfully".to_string(),
        data: Some(StateResponse { state }),
    }))
}
