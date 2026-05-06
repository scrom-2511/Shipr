use redis::AsyncCommands;

use crate::{app_errors::AppError, controller::storage::redis::Redis};

#[derive(Clone)]
pub struct VmPool {
    redis: Redis,
}

impl VmPool {
    pub fn new(redis: Redis) -> Self {
        Self { redis }
    }

    pub async fn add_to_pool(&self, project_id: &str, vm_id: u32) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.set(project_id, vm_id).await?;

        Ok(())
    }

    pub async fn get_from_pool(&self, project_id: &str) -> Result<Option<usize>, AppError> {
        let mut conn = self.redis.get_conn().await?;

        let vm_id = conn.get(project_id).await?;

        Ok(vm_id)
    }

    pub async fn remove_from_pool(&self, project_id: &str) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.del(project_id).await?;

        Ok(())
    }

    pub async fn add_to_ideal_vms(&self, vm_id: u32) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.sadd("ideal_vms", vm_id).await?;

        Ok(())
    }

    pub async fn get_from_ideal_vms(&self) -> Result<Option<u32>, AppError> {
        let mut conn = self.redis.get_conn().await?;

        let vm_id = conn.spop("ideal_vms").await?;

        Ok(vm_id)
    }
}
