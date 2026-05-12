use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{Client, Method};
use serde::Serialize;

use crate::app_errors::AppError;

#[derive(Serialize)]
struct Claims {
    iat: u64,
    exp: u64,
    iss: u64,
}

pub struct Github {
    app_id: u64,
    client: Client,
    owner: String,
    repo: String,
}

impl Github {
    pub fn new(app_id: u64, owner: &str, repo: &str) -> Self {
        Self {
            app_id,
            client: Client::new(),
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }

    fn generate_jwt(&self) -> Result<String, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            iat: now,
            exp: now + 600,
            iss: self.app_id,
        };

        let key_str = fs::read_to_string("shipr-deployment.pem")?;
        let key = EncodingKey::from_rsa_pem(key_str.as_bytes())?;

        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

        Ok(token)
    }

    async fn app_req(&self, method: Method, url: &str) -> Result<reqwest::Response, AppError> {
        let jwt = self.generate_jwt()?;

        let res = self
            .client
            .request(method, url)
            .bearer_auth(jwt)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
    }

    async fn get_installation_id(&self) -> Result<u32, AppError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/installation",
            self.owner, self.repo
        );

        println!("URL: {}", url);

        let res = self.app_req(Method::GET, &url).await?;
        let json = res.json::<serde_json::Value>().await?;

        let id = json["id"].as_u64().unwrap() as u32;

        println!("ID: {}", id);

        Ok(id)
    }

    pub async fn get_installation_access_token(&self) -> Result<String, AppError> {
        let installation_id = self.get_installation_id().await?;

        let url = format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        );

        println!("URL: {}", url);

        let res = self.app_req(Method::POST, &url).await?;
        let json = res.json::<serde_json::Value>().await?;

        println!("JSON: {}", json);

        let token = json["token"].as_str().unwrap();

        Ok(token.to_string())
    }
}
