use gt_common::types::{AIArchetype, AIStrategy, Money};

/// Financial snapshot for strategy decision-making
#[derive(Debug, Clone)]
pub struct FinancialSnapshot {
    pub cash: Money,
    pub debt: Money,
    pub revenue_per_tick: Money,
    pub cost_per_tick: Money,
    pub infrastructure_count: usize,
}

impl FinancialSnapshot {
    pub fn cash_ratio(&self) -> f64 {
        self.cash as f64 / (self.cost_per_tick as f64 * 90.0).max(1.0)
    }

    pub fn profit(&self) -> Money {
        self.revenue_per_tick - self.cost_per_tick
    }

    pub fn is_debt_heavy(&self) -> bool {
        self.debt > self.cash * 2
    }

    pub fn is_profitable(&self) -> bool {
        self.revenue_per_tick > self.cost_per_tick
    }
}

/// Selects AI strategy based on archetype and financial state
pub struct StrategySelector;

impl StrategySelector {
    /// Select the best strategy given current conditions
    pub fn select(archetype: AIArchetype, snapshot: &FinancialSnapshot) -> AIStrategy {
        let cash_ratio = snapshot.cash_ratio();
        let profit = snapshot.profit();
        let debt_heavy = snapshot.is_debt_heavy();

        // Universal survival check
        if cash_ratio < 1.0 || (profit < 0 && snapshot.cash < snapshot.cost_per_tick * 30) {
            return AIStrategy::Survive;
        }

        match archetype {
            AIArchetype::AggressiveExpander => {
                if cash_ratio > 3.0 {
                    AIStrategy::Expand
                } else if profit > 0 {
                    AIStrategy::Compete
                } else {
                    AIStrategy::Consolidate
                }
            }
            AIArchetype::DefensiveConsolidator => {
                if debt_heavy {
                    AIStrategy::Survive
                } else {
                    AIStrategy::Consolidate
                }
            }
            AIArchetype::TechInnovator => {
                if cash_ratio > 4.0 {
                    AIStrategy::Expand
                } else {
                    AIStrategy::Consolidate
                }
            }
            AIArchetype::BudgetOperator => {
                if profit > 0 && cash_ratio > 6.0 && !debt_heavy {
                    AIStrategy::Expand
                } else {
                    AIStrategy::Consolidate
                }
            }
        }
    }

    /// Check if AI should attempt to take a loan
    pub fn should_take_loan(archetype: AIArchetype, snapshot: &FinancialSnapshot) -> bool {
        let weights = super::archetype::ArchetypeWeights::for_archetype(archetype);
        let debt_ratio = snapshot.debt as f64 / snapshot.cash.max(1) as f64;
        debt_ratio < weights.max_debt_ratio() && snapshot.cash < snapshot.cost_per_tick * 30
    }

    /// Check if AI should invest in research
    pub fn should_research(archetype: AIArchetype, snapshot: &FinancialSnapshot) -> bool {
        let weights = super::archetype::ArchetypeWeights::for_archetype(archetype);
        weights.tech_focus > 0.5 && snapshot.cash > 5_000_000
    }
}
