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

        self.alliances.shift_remove(&alliance_id);
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
            match self.co_ownership_proposals.shift_remove(&proposal_node) {
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

        // 1. Regulatory Check: Anti-Monopoly Threshold
        if let Some(node_comp) = self.infra_nodes.get(&node) {
            if let Some(&region_id) = self.cell_to_region.get(&node_comp.cell_index) {
                let market_share = self.calculate_regional_market_share(region_id, corp_id);
                
                // Block if > 50% market share (Section 4 of Docs)
                if market_share > 0.5 {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Regulatory Block: Your market share in this region exceeds 50%. Acquisitions are restricted.".to_string(),
                            level: "warning".to_string(),
                        },
                    );
                    return;
                }
            }
        }

        // 2. Existing validation: check the target corp is a co-owner
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

        // 3. Execute buyout
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

    /// Calculate corporation's market share in a region (based on node count)
    fn calculate_regional_market_share(&self, region_id: EntityId, corp_id: EntityId) -> f64 {
        let mut total_nodes = 0;
        let mut corp_nodes = 0;

        for node in self.infra_nodes.values() {
            if let Some(&r_id) = self.cell_to_region.get(&node.cell_index) {
                if r_id == region_id {
                    total_nodes += 1;
                    if node.owner == corp_id {
                        corp_nodes += 1;
                    }
                }
            }
        }

        if total_nodes == 0 { 0.0 } else { corp_nodes as f64 / total_nodes as f64 }
    }

    pub(super) fn cmd_vote_upgrade(&mut self, node: EntityId, approve: bool) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Check if there is a pending vote for this node
        let (_proposer, votes, _start_tick) = match self.pending_upgrade_votes.get_mut(&node) {
            Some(v) => v,
            None => return,
        };

        // Register the vote
        votes.insert(corp_id, approve);

        // Check if we have reached a resolution
        let mut total_share_voted = 0.0;
        let mut approval_share = 0.0;

        if let Some(ownership) = self.ownerships.get(&node) {
            let co_owner_total: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
            let primary_share = (1.0 - co_owner_total).max(0.0);

            // Add primary owner's vote
            if let Some(&vote) = votes.get(&ownership.owner) {
                total_share_voted += primary_share;
                if vote { approval_share += primary_share; }
            }

            // Add co-owners' votes
            for &(co_id, share) in &ownership.co_owners {
                if let Some(&vote) = votes.get(&co_id) {
                    total_share_voted += share;
                    if vote { approval_share += share; }
                }
            }
        }

        // Majority (>50% of total ownership) required to approve
        if approval_share > 0.5 {
            // PASS: Execute upgrade
            self.pending_upgrade_votes.shift_remove(&node);
            self.execute_shared_upgrade(node);
            
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::UpgradeVotePassed { node },
            );
        } else if (total_share_voted - approval_share) >= 0.5 {
            // FAIL: Rejected by majority
            self.pending_upgrade_votes.shift_remove(&node);
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::UpgradeVoteRejected { node },
            );
        }
    }

    /// Internal: Execute node upgrade and split costs among all owners.
    fn execute_shared_upgrade(&mut self, node_id: EntityId) {
        let node_data = match self.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => return,
        };
        let upgrade_cost = node_data.construction_cost / 2;

        // Split cost among all owners
        if let Some(ownership) = self.ownerships.get(&node_id) {
            let co_owner_total: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
            let primary_share = (1.0 - co_owner_total).max(0.0);

            // Debit primary owner
            if let Some(fin) = self.financials.get_mut(&ownership.owner) {
                fin.cash -= (upgrade_cost as f64 * primary_share) as i64;
            }

            // Debit co-owners
            for &(co_id, share) in &ownership.co_owners {
                if let Some(fin) = self.financials.get_mut(&co_id) {
                    fin.cash -= (upgrade_cost as f64 * share) as i64;
                }
            }
        }

        // Apply physical upgrade
        if let Some(node) = self.infra_nodes.get_mut(&node_id) {
            node.max_throughput *= 1.5;
            node.reliability = (node.reliability + 0.05).min(1.0);
        }
        if let Some(cap) = self.capacities.get_mut(&node_id) {
            cap.max_throughput *= 1.5;
        }
        if let Some(health) = self.healths.get_mut(&node_id) {
            health.condition = 1.0;
        }

        let _node_type = self.infra_nodes.get(&node_id).map(|n| n.node_type).unwrap();
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::InfrastructureDamaged { // Re-using damaged event for condition reset? 
                // Better use NodeBuilt or similar if no NodeUpgraded exists in GameEvent
                entity: node_id,
                damage: -1.0, // Negative damage = repair/upgrade
            }
        );
    }
}
