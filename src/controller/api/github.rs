use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, crypto::CryptoProvider, encode};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

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

    pub async fn get_installation_id(&self, owner: &str, repo: &str) -> Result<u32, AppError> {
        let jwt_token = self.generate_jwt()?;

        println!("JWT Token: {}", jwt_token);

        let url = format!(
            "https://api.github.com/repos/{}/{}/installation",
            owner, repo
        );

        let res = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .header("User-Agent", "shipr-deployment")
            .send()
            .await
            .unwrap();

        let json = res.json::<serde_json::Value>().await.unwrap();

        let installation_id = json["id"].as_u64().unwrap() as u32;

        Ok(installation_id)
    }

    pub async fn get_installation_access_token(&self) -> Result<String, AppError> {
        let jwt_token = self.generate_jwt()?;

        let installation_id = self
            .get_installation_id("scrom-2511", "shipr_test_project")
            .await?;

        let url = format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        );

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .header("User-Agent", "shipr-deployment")
            .send()
            .await
            .unwrap();

        let json = res.json::<serde_json::Value>().await.unwrap();

        let access_token = json["token"].as_str().unwrap();

        Ok(access_token.to_string())
    }

    pub async fn get_defualt_branch(&self, owner: &str, repo: &str) -> Result<String, AppError> {
        let jwt_token = self.generate_jwt()?;

        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

        let res = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .header("User-Agent", "shipr-deployment")
            .send()
            .await
            .unwrap();

        let json = res.json::<serde_json::Value>().await.unwrap();

        let default_branch = json["default_branch"].as_str().unwrap();

        Ok(default_branch.to_string())
    }
}
