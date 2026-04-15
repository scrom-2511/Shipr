use futures::lock::Mutex;
use std::{collections::HashSet, sync::Arc};

use crate::app_errors::AppError;

const MAX_IDS: usize = 64;

pub struct UniqueIdAllocator {
    current_ids: Arc<Mutex<HashSet<usize>>>,
}

impl UniqueIdAllocator {
    pub fn new() -> Self {
        Self {
            current_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn allocate_id(&self) -> Result<usize, AppError> {
        let mut ids = self.current_ids.lock().await;

        for id in 0..MAX_IDS {
            if !ids.contains(&id) {
                ids.insert(id);
                return Ok(id);
            }
        }

        Err(AppError::StartingFirecrackerFailed(
            "No available subnet IDs".to_string(),
        ))
    }

    pub async fn release_id(&self, id: usize) -> Result<(), AppError> {
        let mut ids = self.current_ids.lock().await;
        ids.remove(&id);
        Ok(())
    }
}
