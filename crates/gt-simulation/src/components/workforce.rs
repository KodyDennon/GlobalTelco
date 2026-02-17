use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workforce {
    pub employee_count: u32,
    pub skill_level: f64,
    pub morale: f64,
    pub salary_per_tick: gt_common::types::Money,
}
