use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
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
    access_token: Option<String>,
    owner: String,
    repo: String,
}

impl Github {
    pub fn new(app_id: u64, owner: String, repo: String) -> Self {
        Self {
            app_id,
            client: Client::new(),
            access_token: None,
            owner,
            repo,
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

    async fn app_get(&self, url: &str) -> Result<reqwest::Response, AppError> {
        let jwt = self.generate_jwt()?;

        let res = self
            .client
            .get(url)
            .bearer_auth(jwt)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
    }

    async fn ensure_access_token(&mut self) -> Result<(), AppError> {
        if self.access_token.is_none() {
            let token = self.get_installation_access_token().await?;
            self.access_token = Some(token);
        }
        Ok(())
    }

    async fn api_get(&mut self, url: &str) -> Result<reqwest::Response, AppError> {
        self.ensure_access_token().await?;

        let token = self.access_token.as_ref().unwrap();

        let res = self
            .client
            .get(url)
            .bearer_auth(token)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
    }

    pub async fn get_installation_id(&self) -> Result<u32, AppError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/installation",
            self.owner, self.repo
        );

        let res = self.app_get(&url).await?;
        let json = res.json::<serde_json::Value>().await?;

        let id = json["id"].as_u64().unwrap() as u32;

        Ok(id)
    }

    pub async fn get_installation_access_token(&self) -> Result<String, AppError> {
        let installation_id = self.get_installation_id().await?;

        let url = format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        );

        let res = self.app_get(&url).await?;
        let json = res.json::<serde_json::Value>().await?;

        let token = json["token"].as_str().unwrap();

        Ok(token.to_string())
    }

    async fn get_default_branch(&mut self) -> Result<String, AppError> {
        let url = format!("https://api.github.com/repos/{}/{}", self.owner, self.repo);

        let res = self.api_get(&url).await?;

        let json = res.json::<serde_json::Value>().await?;

        let branch = json["default_branch"].as_str().unwrap();

        Ok(branch.to_string())
    }

    pub async fn get_commit_sha(&mut self) -> Result<String, AppError> {
        let branch = self.get_default_branch().await?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/commits/{}",
            self.owner, self.repo, branch
        );

        let res = self.api_get(&url).await?;
        let json = res.json::<serde_json::Value>().await?;

        let sha = json["sha"].as_str().unwrap();

        Ok(sha.to_string())
    }
}
