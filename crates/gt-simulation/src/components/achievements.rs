use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementTracker {
    pub unlocked: HashSet<String>,
    pub progress: HashMap<String, f64>,
}

impl AchievementTracker {
    pub fn new() -> Self {
        Self {
            unlocked: HashSet::new(),
            progress: HashMap::new(),
        }
    }

    pub fn unlock(&mut self, id: &str) -> bool {
        self.unlocked.insert(id.to_string())
    }

    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }

    pub fn set_progress(&mut self, id: &str, value: f64) {
        self.progress.insert(id.to_string(), value);
    }
}

impl Default for AchievementTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub const ACHIEVEMENTS: &[(&str, &str, &str)] = &[
    ("first_node", "First Node", "Build your first infrastructure node"),
    ("first_profit", "First Profit", "Earn positive revenue for the first time"),
    ("ten_nodes", "Growing Network", "Own 10 infrastructure nodes"),
    ("hundred_nodes", "Network Empire", "Own 100 infrastructure nodes"),
    ("million_revenue", "Millionaire", "Earn $1M in total revenue"),
    ("billion_revenue", "Billionaire", "Earn $1B in total revenue"),
    ("all_regions", "Global Reach", "Have infrastructure in all regions"),
    ("first_merger", "Corporate Raider", "Complete your first acquisition"),
    ("monopoly_region", "Regional Monopoly", "Control >75% of a region's infrastructure"),
    ("aaa_rating", "AAA Rating", "Achieve AAA credit rating"),
    ("survive_bankruptcy", "Phoenix", "Survive a bankruptcy event"),
    ("debt_free", "Debt Free", "Have zero debt with positive cash flow"),
    ("research_complete", "Tech Pioneer", "Complete all research in a category"),
    ("global_backbone", "Backbone Builder", "Build a global backbone node"),
    ("ocean_cable", "Ocean Cable", "Build a submarine landing station"),
    ("first_contract", "Deal Maker", "Complete your first contract"),
    ("espionage_success", "Spy Master", "Successfully complete an espionage mission"),
    ("lobbyist", "Political Player", "Successfully lobby for a policy change"),
    ("co_owner", "Partnership", "Establish co-ownership of infrastructure"),
    // Satellite achievements
    ("first_satellite", "Space Pioneer", "Launch your first satellite into orbit"),
    ("constellation", "Constellation Operator", "Have 50 operational satellites"),
    ("mega_constellation", "Mega Constellation", "Have 500 operational satellites"),
    ("global_sat_coverage", "Global From Above", "Achieve satellite coverage in all regions"),
    ("kessler_survivor", "Kessler Survivor", "Operate satellites while a Kessler cascade is active"),
];
