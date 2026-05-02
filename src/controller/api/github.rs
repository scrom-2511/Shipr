use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, crypto::CryptoProvider, encode};
use reqwest::Client;
use serde::Serialize;

use crate::app_errors::AppError;

#[derive(Serialize)]
struct Claims {
    iat: usize,
    exp: usize,
    iss: String,
}

pub struct Github {
    app_id: String,
    client: Client,
}

impl Github {
    pub fn new(app_id: String) -> Self {
        let client = Client::new();
        Self { app_id, client }
    }

    fn generate_jwt(&self) -> Result<String, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = Claims {
            iat: now - 60,
            exp: now + (10 * 60),
            iss: self.app_id.to_string(),
        };

        let key_str = fs::read_to_string("shipr-deployment.pem")?;

        let key = EncodingKey::from_rsa_pem(key_str.as_bytes()).unwrap();

        let token = encode(&Header::new(Algorithm::RS256), &claims, &key).unwrap();

        Ok(token)
    }

    pub async fn get_installation_access_token(
        &self,
        installation_id: &u64,
    ) -> Result<String, AppError> {
        let jwt_token = self.generate_jwt()?;

        let url = format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        );

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .send()
            .await
            .unwrap();

        let json = res.json::<serde_json::Value>().await.unwrap();

        println!("{:?}", json);

        Ok(String::from(""))
    }
}
