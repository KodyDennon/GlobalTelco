use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryConditions {
    pub domination_score: f64,
    pub tech_score: f64,
    pub wealth_score: f64,
    pub infrastructure_score: f64,
    pub total_score: f64,
    pub victory_type: Option<String>,
}

impl VictoryConditions {
    pub fn new() -> Self {
        Self {
            domination_score: 0.0,
            tech_score: 0.0,
            wealth_score: 0.0,
            infrastructure_score: 0.0,
            total_score: 0.0,
            victory_type: None,
        }
    }

    pub fn update_total(&mut self) {
        self.total_score = self.domination_score * 0.3
            + self.tech_score * 0.2
            + self.wealth_score * 0.25
            + self.infrastructure_score * 0.25;
    }
}

impl Default for VictoryConditions {
    fn default() -> Self {
        Self::new()
    }
}
