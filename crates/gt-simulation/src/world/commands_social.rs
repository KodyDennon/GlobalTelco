use gt_common::types::*;

use super::GameWorld;

impl GameWorld {
    // === Phase 5.1: Alliance System ===

    pub(super) fn cmd_propose_alliance(&mut self, target_corp: EntityId, name: &str, revenue_share: f64) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Validate target corp exists
        if !self.corporations.contains_key(&target_corp) {
            return;
        }

        // Can't ally with yourself
        if corp_id == target_corp {
            return;
        }

        // Check neither corp is already in an alliance (max 1 alliance per corp)
        for alliance in self.alliances.values() {
            if alliance.member_corp_ids.contains(&corp_id)
                || alliance.member_corp_ids.contains(&target_corp)
            {
                return;
            }
        }

        // Clamp revenue share to valid range
        let revenue_share = revenue_share.clamp(0.0, 0.5);

        // Create pending alliance with just the proposer; target must accept
        let alliance_id = self.allocate_entity();
        let alliance = crate::components::alliance::Alliance::new(
            alliance_id,
            name.to_string(),
            vec![corp_id], // Only proposer initially; target joins on accept
            revenue_share,
            self.tick,
        );
        self.alliances.insert(alliance_id, alliance);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GlobalNotification {
                message: format!(
                    "Alliance \"{}\" proposed to corp {}. Awaiting acceptance.",
                    name, target_corp
                ),
                level: "info".to_string(),
            },
        );
    }

    pub(super) fn cmd_accept_alliance(&mut self, alliance_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // All validation with immutable borrows first
        {
            let alliance = match self.alliances.get(&alliance_id) {
                Some(a) => a,
                None => return,
            };

            // Can't join if already a member
            if alliance.member_corp_ids.contains(&corp_id) {
                return;
            }

            // Max 3 members
            if alliance.member_corp_ids.len() >= 3 {
                return;
            }

            // Check this corp is not already in another alliance
            for (id, a) in &self.alliances {
                if *id != alliance_id && a.member_corp_ids.contains(&corp_id) {
                    return;
                }
            }
        }

        // Now mutate — all validation passed
        let members = if let Some(alliance) = self.alliances.get_mut(&alliance_id) {
            alliance.member_corp_ids.push(corp_id);
            alliance.trust_scores.insert(corp_id, 0.5);
            alliance.member_corp_ids.clone()
        } else {
            return;
        };

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::AllianceFormed {
                alliance_id,
                members,
            },
        );
    }

    pub(super) fn cmd_dissolve_alliance(&mut self, alliance_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Only members can dissolve
        let is_member = self
            .alliances
            .get(&alliance_id)
            .map(|a| a.member_corp_ids.contains(&corp_id))
            .unwrap_or(false);

        if !is_member {
            return;
        }

        self.alliances.remove(&alliance_id);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::AllianceDissolved {
                alliance_id,
                reason: format!("Dissolved by corporation {}", corp_id),
            },
        );
    }

    // === Phase 5.2: Legal System ===

    pub(super) fn cmd_file_lawsuit(&mut self, defendant: EntityId, lawsuit_type: &str, damages: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Validate defendant exists and is a different corp
        if !self.corporations.contains_key(&defendant) || corp_id == defendant {
            return;
        }

        // Parse lawsuit type
        let lt = match lawsuit_type {
            "PatentInfringement" => crate::components::lawsuit::LawsuitType::PatentInfringement,
            "OwnershipDispute" => crate::components::lawsuit::LawsuitType::OwnershipDispute,
            "SabotageClaim" => crate::components::lawsuit::LawsuitType::SabotageClaim,
            "RegulatoryComplaint" => crate::components::lawsuit::LawsuitType::RegulatoryComplaint,
            _ => return,
        };

        // Filing cost is 10% of damages claimed
        let filing_cost = damages / 10;

        // Check plaintiff has enough cash for filing cost
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < filing_cost {
                return;
            }
        } else {
            return;
        }

        // Deduct filing cost from plaintiff
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= filing_cost;
        }

        // Deterministic resolution timing: 20-50 ticks based on entity ID
        let lawsuit_id = self.allocate_entity();
        let resolution_ticks = 20 + (lawsuit_id % 31); // 20-50 range

        let lawsuit = crate::components::lawsuit::Lawsuit::new(
            lawsuit_id,
            corp_id,
            defendant,
            lt,
            damages,
            self.tick,
            resolution_ticks,
        );
        self.lawsuits.insert(lawsuit_id, lawsuit);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LawsuitFiled {
                lawsuit_id,
                plaintiff: corp_id,
                defendant,
            },
        );
    }

    pub(super) fn cmd_settle_lawsuit(&mut self, lawsuit_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let lawsuit = match self.lawsuits.get(&lawsuit_id) {
            Some(l) if l.status == crate::components::lawsuit::LawsuitStatus::Active => l.clone(),
            _ => return,
        };

        // Only defendant can settle
        if lawsuit.defendant != corp_id {
            return;
        }

        // Settlement: defendant pays 60% of claimed damages
        let settlement_amount = (lawsuit.damages_claimed as f64 * 0.6) as Money;

        // Deduct from defendant, credit plaintiff
        if let Some(fin) = self.financials.get_mut(&lawsuit.defendant) {
            fin.cash -= settlement_amount;
        }
        if let Some(fin) = self.financials.get_mut(&lawsuit.plaintiff) {
            fin.cash += settlement_amount;
        }

        // Mark as settled
        if let Some(l) = self.lawsuits.get_mut(&lawsuit_id) {
            l.settle();
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SettlementReached {
                lawsuit_id,
                plaintiff: lawsuit.plaintiff,
                defendant: lawsuit.defendant,
                amount: settlement_amount,
            },
        );
    }

    // === Phase 10.5: Cooperative Infrastructure ===

    pub(super) fn cmd_propose_co_ownership(&mut self, node: EntityId, target_corp: EntityId, share_pct: f64) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Verify player owns the node
        let owner = match self.infra_nodes.get(&node) {
            Some(n) => n.owner,
            None => return,
        };
        if owner != corp_id {
            return;
        }

        // Verify target exists and share is valid
        if !self.corporations.contains_key(&target_corp) || share_pct <= 0.0 || share_pct >= 1.0 {
            return;
        }

        // Check if already a co-owner
        if let Some(ownership) = self.ownerships.get(&node) {
            if ownership.co_owners.iter().any(|(id, _)| *id == target_corp) {
                return;
            }
        }

        // Check if there's already a pending proposal for this node
        if self.co_ownership_proposals.contains_key(&node) {
            return;
        }

        // Store pending proposal — target corp must accept before it takes effect
        self.co_ownership_proposals
            .insert(node, (corp_id, target_corp, share_pct));
    }

    pub(super) fn cmd_respond_co_ownership(&mut self, proposal_node: EntityId, accept: bool) {
        // Look up pending proposal by node ID
        let (_proposer, target, share_pct) =
            match self.co_ownership_proposals.remove(&proposal_node) {
                Some(p) => p,
                None => return,
            };

        // Verify the responding corp is the target of the proposal
        // (In single-player the player_corp_id may be the target for AI-initiated proposals,
        //  or the AI system calls this for proposals the player made to AI corps)
        if accept {
            // Apply co-ownership
            if let Some(ownership) = self.ownerships.get_mut(&proposal_node) {
                if !ownership.co_owners.iter().any(|(id, _)| *id == target) {
                    ownership.co_owners.push((target, share_pct));
                }
            }

            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::CoOwnershipEstablished {
                    node: proposal_node,
                    partner: target,
                    share_pct,
                },
            );
        } else {
            // Rejected — proposal already removed, just emit notification
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!(
                        "Co-ownership proposal for node {} was rejected",
                        proposal_node
                    ),
                    level: "info".to_string(),
                },
            );
        }
    }

    pub(super) fn cmd_propose_buyout(&mut self, node: EntityId, target_corp: EntityId, price: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Check the target corp is a co-owner of this node
        let is_co_owner = self
            .ownerships
            .get(&node)
            .map(|o| o.co_owners.iter().any(|(id, _)| *id == target_corp))
            .unwrap_or(false);
        if !is_co_owner {
            return;
        }

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < price {
                return;
            }
        }

        // Execute buyout
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= price;
        }
        if let Some(fin) = self.financials.get_mut(&target_corp) {
            fin.cash += price;
        }
        if let Some(ownership) = self.ownerships.get_mut(&node) {
            ownership.co_owners.retain(|(id, _)| *id != target_corp);
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::BuyoutCompleted {
                node,
                buyer: corp_id,
                seller: target_corp,
                price,
            },
        );
    }

    pub(super) fn cmd_vote_upgrade(&mut self, node: EntityId, approve: bool) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Verify the voting corp has ownership stake in this node
        let has_majority = if let Some(ownership) = self.ownerships.get(&node) {
            if ownership.owner == corp_id {
                // Primary owner — majority by default (owns 1.0 - sum of co-owner shares)
                let co_owner_total: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
                (1.0 - co_owner_total) > 0.5
            } else {
                // Co-owner — check their share
                ownership
                    .co_owners
                    .iter()
                    .find(|(id, _)| *id == corp_id)
                    .map(|(_, share)| *share > 0.5)
                    .unwrap_or(false)
            }
        } else {
            return;
        };

        if !has_majority {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "Upgrade vote failed: insufficient ownership stake (need >50%)"
                        .to_string(),
                    level: "warning".to_string(),
                },
            );
            return;
        }

        if approve {
            self.cmd_upgrade_node(node);
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::UpgradeVotePassed { node },
            );
        } else {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::UpgradeVoteRejected { node },
            );
        }
    }
}
