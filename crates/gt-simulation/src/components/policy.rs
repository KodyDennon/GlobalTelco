use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    pub settings: HashMap<String, String>,
}

impl Policy {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.settings.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }
}
