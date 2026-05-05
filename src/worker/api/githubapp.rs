use reqwest::{Client, Response};

use crate::app_errors::AppError;

pub struct GithubApp {
    client: Client,
    access_token: String,
    owner: String,
    repo: String,
}

impl GithubApp {
    pub fn new(access_token: String, owner: String, repo: String) -> Self {
        let client = Client::new();
        Self {
            client,
            access_token,
            owner,
            repo,
        }
    }

    async fn api_get(&mut self, url: &str) -> Result<Response, AppError> {
        let res = self
            .client
            .get(url)
            .bearer_auth(&self.access_token)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
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

    pub async fn get_tarball_url(&mut self) -> Result<String, AppError> {
        let sha = self.get_commit_sha().await?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/tarball/{}",
            self.owner, self.repo, sha
        );

        Ok(url)
    }
}
