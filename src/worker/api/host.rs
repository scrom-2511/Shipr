use std::{
    ops::Deref,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::sleep};

use reqwest::Client;
use serde_json::json;

use crate::{app_errors::AppError, app_types::JobType};

pub struct Host {
    client: Client,
    last_heartbeat: Arc<Mutex<Instant>>,
}

impl Host {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn heartbeat(&self) {
        let mut hb = self.last_heartbeat.lock().await;
        *hb = Instant::now();
    }

    pub fn start_watchdog(&self, project_id: String, job_type: JobType) {
        let last_heartbeat = self.last_heartbeat.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;

                let last = *last_heartbeat.lock().await;

                if last.elapsed() > Duration::from_secs(3600) {
                    let _ = client
                        .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/kill-vm")
                        .json(&json!({
                            "project_id": project_id,
                            "job_type": job_type,
                        }))
                        .send()
                        .await;

                    break;
                }
            }
        });
    }

    pub async fn kill_vm(&self, project_id: String, job_type: JobType) -> Result<(), AppError> {
        self.client
            .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/kill-vm")
            .json(&json!({
                "project_id": project_id,
                "job_type": job_type,
            }))
            .send()
            .await?;
        Ok(())
    }
}
