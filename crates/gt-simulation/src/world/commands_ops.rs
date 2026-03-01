use gt_common::types::*;

use crate::components::covert_ops::MissionType;
use crate::components::*;

use super::GameWorld;

impl GameWorld {
    // === Phase 10.3: Espionage & Sabotage ===

    pub(super) fn cmd_launch_espionage(&mut self, target: EntityId, region: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Base cost depends on target's security level
        let target_security = self
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 100_000 + target_security as Money * 100_000;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let attacker_security = self
            .covert_ops
            .get(&corp_id)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let success_chance = 0.8 - target_security as f64 * 0.1 + attacker_security as f64 * 0.05;

        let mission = Mission {
            mission_type: MissionType::Espionage,
            target,
            region,
            start_tick: self.tick,
            duration: 20,
            cost,
            success_chance: success_chance.clamp(0.1, 0.95),
            completed: false,
        };

        self.covert_ops
            .entry(corp_id)
            .or_default()
            .active_missions
            .push(mission);
    }

    pub(super) fn cmd_launch_sabotage(&mut self, target: EntityId, node: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // If no specific node given, pick a random node owned by target
        let actual_node = if node == 0 {
            self.corp_infra_nodes
                .get(&target)
                .and_then(|nodes| {
                    if nodes.is_empty() {
                        None
                    } else {
                        // Use tick as a simple deterministic "random" index
                        let idx = self.tick as usize % nodes.len();
                        nodes.get(idx).copied()
                    }
                })
                .unwrap_or(0)
        } else {
            node
        };

        // Must have a valid target node
        if actual_node == 0 {
            return;
        }

        let target_security = self
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 200_000 + target_security as Money * 200_000;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let attacker_security = self
            .covert_ops
            .get(&corp_id)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let detection_chance = 0.2 - attacker_security as f64 * 0.05 + target_security as f64 * 0.1;

        let region = self
            .positions
            .get(&actual_node)
            .and_then(|p| p.region_id)
            .unwrap_or(0);

        let mission = Mission {
            mission_type: MissionType::Sabotage,
            target,
            region,
            start_tick: self.tick,
            duration: 15,
            cost,
            success_chance: (1.0 - detection_chance).clamp(0.1, 0.95),
            completed: false,
        };

        self.covert_ops
            .entry(corp_id)
            .or_default()
            .active_missions
            .push(mission);
    }

    pub(super) fn cmd_upgrade_security(&mut self, level: u32) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let cost = level as Money * 500_000;
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let ops = self
            .covert_ops
            .entry(corp_id)
            .or_default();
        ops.security_level = level;

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SecurityUpgraded {
                corporation: corp_id,
                level,
            },
        );
    }

    // === Phase 10.4: Lobbying ===

    pub(super) fn cmd_start_lobbying(&mut self, region: EntityId, policy: &str, budget: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        if !self.regions.contains_key(&region) {
            return;
        }

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < budget {
                return;
            }
        }

        let lobby_policy = match policy {
            "ReduceTax" => LobbyPolicy::ReduceTax,
            "RelaxZoning" => LobbyPolicy::RelaxZoning,
            "FastTrackPermits" => LobbyPolicy::FastTrackPermits,
            "IncreasedCompetitorBurden" => LobbyPolicy::IncreasedCompetitorBurden,
            "SubsidyRequest" => LobbyPolicy::SubsidyRequest,
            _ => return,
        };

        let campaign = LobbyingCampaign::new(corp_id, region, lobby_policy, budget, self.tick);
        let campaign_id = self.allocate_entity();
        self.lobbying_campaigns.insert(campaign_id, campaign);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LobbyingStarted {
                corporation: corp_id,
                region,
                policy: policy.to_string(),
            },
        );
    }

    pub(super) fn cmd_propose_contract(&mut self, from: EntityId, to: EntityId, terms: &str) {
        // Parse structured terms: "type:Transit,bandwidth:1000,price:5000,duration:100"
        let mut bandwidth: f64 = 1000.0;
        let mut price: Money = 500;
        let mut duration: Tick = 180;
        let mut contract_type = ContractType::Transit;

        for part in terms.split(',') {
            let kv: Vec<&str> = part.splitn(2, ':').collect();
            if kv.len() == 2 {
                let key = kv[0].trim();
                let val = kv[1].trim();
                match key {
                    "bandwidth" => {
                        if let Ok(v) = val.parse::<f64>() {
                            bandwidth = v.clamp(100.0, 100_000.0);
                        }
                    }
                    "price" => {
                        if let Ok(v) = val.parse::<i64>() {
                            price = v.clamp(100, 10_000_000);
                        }
                    }
                    "duration" => {
                        if let Ok(v) = val.parse::<u64>() {
                            duration = v.clamp(10, 1000);
                        }
                    }
                    "type" => {
                        contract_type = match val {
                            "Peering" => ContractType::Peering,
                            "SLA" => ContractType::SLA,
                            _ => ContractType::Transit,
                        };
                    }
                    _ => {}
                }
            }
        }

        // Penalty scales with contract value: 10% of total contract value
        let total_value = price * duration as i64;
        let penalty = (total_value / 10).max(1000);

        let contract = Contract::new_proposal(
            contract_type,
            from,
            to,
            bandwidth,
            price,
            self.tick,
            duration,
            penalty,
        );
        let contract_id = self.allocate_entity();
        self.contracts.insert(contract_id, contract);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ContractProposed {
                entity: contract_id,
                from,
                to,
            },
        );
    }

    pub(super) fn cmd_start_research(&mut self, corporation: EntityId, tech: &str) {
        let category = match tech {
            "optical" | "OpticalNetworks" => ResearchCategory::OpticalNetworks,
            "wireless" | "Wireless5G" => ResearchCategory::Wireless5G,
            "satellite" | "Satellite" => ResearchCategory::Satellite,
            "datacenter" | "DataCenter" => ResearchCategory::DataCenter,
            "resilience" | "NetworkResilience" => ResearchCategory::NetworkResilience,
            "efficiency" | "OperationalEfficiency" => ResearchCategory::OperationalEfficiency,
            _ => return,
        };

        // Check if already researching
        let already_researching = self
            .tech_research
            .values()
            .any(|r| r.researcher == Some(corporation) && !r.completed);
        if already_researching {
            return;
        }

        let research_id = self.allocate_entity();
        let mut research = TechResearch::new(category, tech);
        research.researcher = Some(corporation);

        self.tech_research.insert(research_id, research);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ResearchStarted {
                corporation,
                tech: tech.to_string(),
            },
        );
    }
}
