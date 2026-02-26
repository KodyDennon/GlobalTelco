use gt_common::types::{AIArchetype, AIStrategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiState {
    pub archetype: AIArchetype,
    pub strategy: AIStrategy,
    pub aggression: f64,
    pub risk_tolerance: f64,
    /// When true, this is a proxy for a disconnected player — policy-only execution, no strategic changes
    pub proxy_mode: bool,
    /// Consecutive ticks the corporation has been in deep negative cash (< -500_000).
    /// Used by the finance system to trigger bankruptcy liquidation.
    #[serde(default)]
    pub bankruptcy_ticks: u32,
}

impl AiState {
    pub fn new(archetype: AIArchetype) -> Self {
        let (aggression, risk_tolerance) = match archetype {
            AIArchetype::AggressiveExpander => (0.9, 0.8),
            AIArchetype::DefensiveConsolidator => (0.3, 0.2),
            AIArchetype::TechInnovator => (0.5, 0.6),
            AIArchetype::BudgetOperator => (0.4, 0.3),
        };
        Self {
            archetype,
            strategy: AIStrategy::Expand,
            aggression,
            risk_tolerance,
            proxy_mode: false,
            bankruptcy_ticks: 0,
        }
    }
}
