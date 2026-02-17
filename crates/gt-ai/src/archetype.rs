use gt_common::types::AIArchetype;

/// Weight configuration per AI archetype
#[derive(Debug, Clone)]
pub struct ArchetypeWeights {
    pub expansion: f64,
    pub debt_tolerance: f64,
    pub tech_focus: f64,
    pub risk_tolerance: f64,
    pub maintenance_priority: f64,
    pub contract_willingness: f64,
}

impl ArchetypeWeights {
    pub fn for_archetype(archetype: AIArchetype) -> Self {
        match archetype {
            AIArchetype::AggressiveExpander => Self {
                expansion: 0.9,
                debt_tolerance: 0.8,
                tech_focus: 0.3,
                risk_tolerance: 0.8,
                maintenance_priority: 0.3,
                contract_willingness: 0.5,
            },
            AIArchetype::DefensiveConsolidator => Self {
                expansion: 0.3,
                debt_tolerance: 0.2,
                tech_focus: 0.5,
                risk_tolerance: 0.2,
                maintenance_priority: 0.9,
                contract_willingness: 0.7,
            },
            AIArchetype::TechInnovator => Self {
                expansion: 0.5,
                debt_tolerance: 0.5,
                tech_focus: 0.9,
                risk_tolerance: 0.5,
                maintenance_priority: 0.6,
                contract_willingness: 0.4,
            },
            AIArchetype::BudgetOperator => Self {
                expansion: 0.4,
                debt_tolerance: 0.1,
                tech_focus: 0.3,
                risk_tolerance: 0.1,
                maintenance_priority: 0.7,
                contract_willingness: 0.8,
            },
        }
    }

    /// How many ticks of cash reserves the AI wants to maintain
    pub fn desired_cash_reserve_ticks(&self) -> f64 {
        // Risk-averse AIs want more cash buffer
        30.0 + (1.0 - self.risk_tolerance) * 60.0
    }

    /// Maximum debt-to-cash ratio the AI will tolerate
    pub fn max_debt_ratio(&self) -> f64 {
        self.debt_tolerance * 3.0
    }
}
