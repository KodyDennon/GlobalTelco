use gt_common::types::*;

use crate::components::*;

use super::GameWorld;

impl GameWorld {
    // === Phase 5.3: Patents & Licensing ===

    pub(super) fn cmd_file_patent(&mut self, tech_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let tech = match self.tech_research.get(&tech_id) {
            Some(t) if t.completed => t,
            _ => return,
        };
        if tech.researcher != Some(corp_id) {
            return;
        }
        if self.patents.values().any(|p| p.tech_id == tech_id) {
            return;
        }
        let patent_id = self.allocate_entity();
        let patent = Patent::new(tech_id, corp_id, self.tick);
        self.patents.insert(patent_id, patent);
        if let Some(t) = self.tech_research.get_mut(&tech_id) {
            t.patent_status = crate::components::tech_research::PatentStatus::Patented;
            t.patent_owner = Some(corp_id);
        }
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::PatentFiled {
                patent_id,
                tech_id,
                holder: corp_id,
            },
        );
    }

    pub(super) fn cmd_request_license(&mut self, patent_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let patent = match self.patents.get(&patent_id) {
            Some(p) => p.clone(),
            None => return,
        };
        if patent.holder_corp == corp_id {
            return;
        }
        if self.licenses.values().any(|l| {
            l.patent_id == patent_id && l.licensee_corp == corp_id && l.is_active(self.tick)
        }) {
            return;
        }
        let price = patent.license_price;
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < price {
                return;
            }
        } else {
            return;
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= price;
        }
        if let Some(fin) = self.financials.get_mut(&patent.holder_corp) {
            fin.cash += price;
        }
        let license_id = self.allocate_entity();
        // For Lease licenses, recalculate expires_tick from current tick + patent lease_duration
        let actual_license_type = match patent.license_type {
            LicenseType::Lease { .. } => LicenseType::Lease {
                expires_tick: self.tick + patent.lease_duration,
            },
            other => other,
        };
        let license =
            License::new(patent_id, corp_id, actual_license_type, price, self.tick);
        self.licenses.insert(license_id, license);
        if let Some(tech) = self.tech_research.get_mut(&patent.tech_id) {
            if !tech.licensed_to.contains(&corp_id) {
                tech.licensed_to.push(corp_id);
            }
        }
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LicenseGranted {
                license_id,
                patent_id,
                licensee: corp_id,
                price,
            },
        );
    }

    pub(super) fn cmd_set_license_price(
        &mut self,
        patent_id: EntityId,
        price: Money,
        license_type: &str,
        per_unit_price: Money,
        lease_duration: u64,
    ) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let patent = match self.patents.get(&patent_id) {
            Some(p) if p.holder_corp == corp_id => p,
            _ => return,
        };
        let duration = if lease_duration > 0 {
            lease_duration
        } else {
            patent.lease_duration
        };
        let lt = match license_type {
            "Permanent" => LicenseType::Permanent,
            "Royalty" => LicenseType::Royalty,
            "PerUnit" => LicenseType::PerUnit,
            "Lease" => LicenseType::Lease {
                expires_tick: self.tick + duration,
            },
            _ => patent.license_type,
        };
        if let Some(p) = self.patents.get_mut(&patent_id) {
            p.license_price = price;
            p.license_type = lt;
            p.per_unit_price = if per_unit_price > 0 {
                per_unit_price
            } else {
                p.per_unit_price
            };
            p.lease_duration = duration;
        }
    }

    pub(super) fn cmd_revoke_license(&mut self, license_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let (patent_id, licensee) = match self.licenses.get(&license_id) {
            Some(l) => (l.patent_id, l.licensee_corp),
            None => return,
        };
        match self.patents.get(&patent_id) {
            Some(p) if p.holder_corp == corp_id => {}
            _ => return,
        }
        self.licenses.shift_remove(&license_id);
        if let Some(patent) = self.patents.get(&patent_id) {
            let tech_id = patent.tech_id;
            if let Some(tech) = self.tech_research.get_mut(&tech_id) {
                tech.licensed_to.retain(|&id| id != licensee);
            }
        }
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LicenseRevoked {
                license_id,
                patent_id,
                licensee,
            },
        );
    }

    pub(super) fn cmd_start_independent_research(&mut self, tech_id: EntityId, premium: bool) {
        use crate::components::tech_research::IndependentTier;

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let tech = match self.tech_research.get(&tech_id) {
            Some(t) if t.completed => t,
            _ => return,
        };
        let is_patented_by_other = self
            .patents
            .values()
            .any(|p| p.tech_id == tech_id && p.holder_corp != corp_id);
        if !is_patented_by_other {
            return;
        }
        // Clone needed fields to release immutable borrow before mutable operations
        let tech_name = tech.name.clone();
        let tech_category = tech.category;
        let tech_total_cost = tech.total_cost;
        let tech_throughput_bonus = tech.throughput_bonus;
        let tech_cost_reduction = tech.cost_reduction;
        let tech_reliability_bonus = tech.reliability_bonus;
        let tech_prerequisites = tech.prerequisites.clone();
        let _ = tech;

        let already_researching = self.tech_research.values().any(|r| {
            r.name.starts_with("[Independent] ")
                && r.name.ends_with(&tech_name)
                && r.researcher == Some(corp_id)
                && !r.completed
        });
        if already_researching {
            return;
        }

        // Standard tier: 150% cost, normal stats, cannot patent
        // Premium tier: 200% cost, +10% bonus on all stats, can patent
        let (cost_multiplier, tier) = if premium {
            (2.0, IndependentTier::Premium)
        } else {
            (1.5, IndependentTier::Standard)
        };

        let independent_cost = (tech_total_cost as f64 * cost_multiplier) as Money;
        let research_id = self.allocate_entity();

        let tier_label = if premium { "Premium" } else { "Standard" };
        let mut independent = TechResearch::with_details(
            tech_category,
            format!("[Independent] {}", tech_name),
            format!(
                "{} independent research to bypass patent on {} ({}x cost)",
                tier_label, tech_name, cost_multiplier
            ),
            independent_cost,
            tech_throughput_bonus,
            tech_cost_reduction,
            tech_reliability_bonus,
            tech_prerequisites,
        );
        independent.researcher = Some(corp_id);
        independent.independent_tier = tier;
        self.tech_research.insert(research_id, independent);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::IndependentResearchStarted {
                corporation: corp_id,
                tech_id,
                cost_multiplier,
            },
        );
    }

    // === Phase 5.4: Government Grants ===

    pub(super) fn cmd_bid_for_grant(&mut self, grant_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let grant = match self.grants.get(&grant_id) {
            Some(g) if g.is_available() => g,
            _ => return,
        };
        if self.tick >= grant.deadline_tick {
            return;
        }
        let region = grant.region_id;
        if let Some(g) = self.grants.get_mut(&grant_id) {
            g.awarded_corp = Some(corp_id);
            g.status = GrantStatus::Awarded;
        }
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GrantAwarded {
                grant_id,
                corporation: corp_id,
                region,
            },
        );
    }

    pub(super) fn cmd_complete_grant(&mut self, grant_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };
        let grant = match self.grants.get(&grant_id) {
            Some(g) => g.clone(),
            None => return,
        };
        if grant.awarded_corp != Some(corp_id) {
            return;
        }
        if grant.progress < 1.0 {
            return;
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash += grant.reward_cash;
        }
        if grant.tax_break_pct > 0.0 {
            if let Some(region) = self.regions.get_mut(&grant.region_id) {
                region.tax_rate = (region.tax_rate * (1.0 - grant.tax_break_pct)).max(0.01);
            }
        }
        if let Some(g) = self.grants.get_mut(&grant_id) {
            g.status = GrantStatus::Completed;
        }
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GrantCompleted {
                grant_id,
                corporation: corp_id,
                reward: grant.reward_cash,
            },
        );
    }
}
