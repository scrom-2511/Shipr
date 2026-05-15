use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::{
    app::{controllers::ApiResponse, middlewares::AuthMiddleware},
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

    let header = Header::new(Algorithm::HS256);

    let now = chrono::Utc::now().timestamp() as u64;

    let claims = &StateClaims {
        user_id,
        exp: now + (24 * 60 * 60),
    };

    let state = jsonwebtoken::encode(
        &header,
        claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .map_err(|_| AppError::InternalServerError);

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "State generated successfully".to_string(),
        data: Some(StateResponse { state: state? }),
    }))
}
