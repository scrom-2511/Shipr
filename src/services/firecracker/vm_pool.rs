use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

#[derive(Clone)]
pub struct VmPool {
    pool: Arc<Mutex<HashMap<Uuid, u32>>>,
}

impl VmPool {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_to_pool(&self, project_id: Uuid, vm_id: u32) {
        let mut pool = self.pool.lock().unwrap();
        pool.insert(project_id, vm_id);
    }

    pub fn get_from_pool(&self, project_id: Uuid) -> Option<u32> {
        let pool = self.pool.lock().unwrap();
        pool.get(&project_id).copied()
    }
}
