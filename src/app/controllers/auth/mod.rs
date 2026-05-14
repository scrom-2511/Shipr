pub mod github_signup;
pub mod signin;
pub mod signup;

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;

use crate::app_errors::AppError;

const JWT_SECRET: &[u8] = b"shipr_jwt_secret_key_2024";

#[derive(Serialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub iat: u64,
    pub exp: u64,
}

pub fn generate_token(user_id: i32, email: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now,
        exp: now + (24 * 60 * 60),
    };

    let header = Header::new(Algorithm::HS256);
    let token = encode(&header, &claims, &EncodingKey::from_secret(JWT_SECRET))?;
    Ok(token)
}
