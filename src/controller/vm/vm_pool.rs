use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

#[derive(Clone)]
pub struct VmPool {
    pool: Arc<Mutex<HashMap<Uuid, u32>>>,
    ideal_vms: Arc<Mutex<Vec<u32>>>,
}

impl VmPool {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(HashMap::new())),
            ideal_vms: Arc::new(Mutex::new(Vec::new())),
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

    pub fn remove_from_pool(&self, project_id: Uuid) {
        let mut pool = self.pool.lock().unwrap();
        pool.remove(&project_id);
    }

    pub fn add_to_ideal_vms(&self, vm_id: u32) {
        let mut ideal_vms = self.ideal_vms.lock().unwrap();
        ideal_vms.push(vm_id);
    }

    pub fn get_from_ideal_vms(&self) -> Option<u32> {
        let mut ideal_vms = self.ideal_vms.lock().unwrap();
        ideal_vms.pop()
    }
}
