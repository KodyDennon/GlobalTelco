use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workforce {
    pub employee_count: u32,
    pub skill_level: f64,
    pub morale: f64,
    pub salary_per_tick: gt_common::types::Money,
    /// Number of dedicated maintenance crews. Speeds up active repairs.
    /// Speed multiplier: `1.0 + 0.1 * crew_count`, capped at 2.0.
    #[serde(default)]
    pub maintenance_crew_count: u32,
}
