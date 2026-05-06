use redis::AsyncCommands;

use crate::{app_errors::AppError, controller::storage::redis::Redis};

const MAX_IDS: usize = 64;

#[derive(Clone)]
pub struct IdAllocator {
    redis: Redis,
}

impl IdAllocator {
    pub fn new(redis: Redis) -> Self {
        Self { redis }
    }

    pub async fn get_current_ids(&self) -> Result<Vec<usize>, AppError> {
        let mut conn = self.redis.get_conn().await?;

        let current_ids = conn.smembers("current_ids").await?;

        Ok(current_ids)
    }

    pub async fn add_to_current_ids(&self, id: usize) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.sadd("current_ids", id).await?;

        Ok(())
    }

    pub async fn remove_from_current_ids(&self, id: usize) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.srem("current_ids", id).await?;

        Ok(())
    }

    pub async fn allocate_id(&self) -> Result<usize, AppError> {
        let ids = self.get_current_ids().await?;

        for id in 0..MAX_IDS {
            if !ids.contains(&id) {
                self.add_to_current_ids(id).await?;
                return Ok(id);
            }
        }

        Err(AppError::StartingFirecrackerFailed(
            "No available subnet IDs".to_string(),
        ))
    }

    pub async fn release_id(&self, id: usize) -> Result<(), AppError> {
        self.remove_from_current_ids(id).await?;

        Ok(())
    }
}
