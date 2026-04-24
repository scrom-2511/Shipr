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
}
