use gt_common::types::*;

use crate::components::*;

use super::GameWorld;

impl GameWorld {
    pub(super) fn cmd_take_loan(&mut self, corporation: EntityId, amount: Money) {
        if amount <= 0 {
            return;
        }

        // Credit rating determines interest rate
        let interest_rate = if let Some(corp) = self.corporations.get(&corporation) {
            match corp.credit_rating {
                CreditRating::AAA => 0.03,
                CreditRating::AA => 0.04,
                CreditRating::A => 0.05,
                CreditRating::BBB => 0.07,
                CreditRating::BB => 0.10,
                CreditRating::B => 0.15,
                CreditRating::CCC => 0.25,
                CreditRating::D => return, // Can't borrow
            }
        } else {
            return;
        };

        let duration = 365; // ~1 year in ticks
        let debt = DebtInstrument::new(corporation, amount, interest_rate, duration);
        let payment = debt.payment_per_tick;

        let loan_id = self.allocate_entity();
        self.debt_instruments.insert(loan_id, debt);

        if let Some(fin) = self.financials.get_mut(&corporation) {
            fin.cash += amount;
            fin.debt += amount;
            fin.cost_per_tick += payment;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LoanTaken {
                corporation,
                amount,
            },
        );
    }

    pub(super) fn cmd_repay_loan(&mut self, loan_id: EntityId, amount: Money) {
        if let Some(debt) = self.debt_instruments.get_mut(&loan_id) {
            let holder = debt.holder;
            let repay = amount.min(debt.principal);
            debt.principal -= repay;

            if let Some(fin) = self.financials.get_mut(&holder) {
                fin.cash -= repay;
                fin.debt = (fin.debt - repay).max(0);
                if debt.is_paid_off() {
                    fin.cost_per_tick = (fin.cost_per_tick - debt.payment_per_tick).max(0);
                }
            }

            if debt.is_paid_off() {
                self.debt_instruments.shift_remove(&loan_id);
            }
        }
    }

    // === Phase 10.1: Bankruptcy & Auctions ===

    pub(super) fn cmd_declare_bankruptcy(&mut self, entity: EntityId) {
        if !self.corporations.contains_key(&entity) {
            return;
        }

        // Gather all assets owned by the bankrupt corporation
        let assets: Vec<EntityId> = self
            .corp_infra_nodes
            .get(&entity)
            .cloned()
            .unwrap_or_default();

        if assets.is_empty() {
            // Nothing to auction, just emit event
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::BankruptcyDeclared {
                    corporation: entity,
                },
            );
            return;
        }

        // Create auction for all assets
        let auction_id = self.allocate_entity();
        let auction = Auction::new(entity, assets.clone(), self.tick, 50);
        self.auctions.insert(auction_id, auction);

        // Zero out the corporation's finances
        if let Some(fin) = self.financials.get_mut(&entity) {
            fin.cash = 0;
            fin.revenue_per_tick = 0;
            fin.cost_per_tick = 0;
            fin.debt = 0;
        }

        // Remove all debt instruments
        let debts_to_remove: Vec<EntityId> = self
            .debt_instruments
            .iter()
            .filter(|(_, d)| d.holder == entity)
            .map(|(&id, _)| id)
            .collect();
        for id in debts_to_remove {
            self.debt_instruments.shift_remove(&id);
        }


        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::BankruptcyDeclared {
                corporation: entity,
            },
        );
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::AuctionStarted {
                auction: auction_id,
                seller: entity,
                asset_count: assets.len() as u32,
            },
        );
    }

    pub(super) fn cmd_request_bailout(&mut self, entity: EntityId) {
        // Bailout: get a high-interest emergency loan
        let cost_per_tick = self
            .financials
            .get(&entity)
            .map(|f| f.cost_per_tick)
            .unwrap_or(0);
        let bailout_amount = cost_per_tick * 180; // 6 months of costs
        if bailout_amount <= 0 {
            return;
        }

        let interest_rate = 0.30; // 30% — punitive
        let debt = DebtInstrument::new(entity, bailout_amount, interest_rate, 365);
        let payment = debt.payment_per_tick;
        let loan_id = self.allocate_entity();
        self.debt_instruments.insert(loan_id, debt);

        if let Some(fin) = self.financials.get_mut(&entity) {
            fin.cash += bailout_amount;
            fin.debt += bailout_amount;
            fin.cost_per_tick += payment;
        }

        // Downgrade credit rating
        if let Some(corp) = self.corporations.get_mut(&entity) {
            corp.credit_rating = CreditRating::CCC;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::BailoutTaken {
                corporation: entity,
                amount: bailout_amount,
                interest_rate,
            },
        );
    }

    pub(super) fn cmd_accept_bailout(&mut self, entity: EntityId) {
        // Same as request — this is the confirmation path
        self.cmd_request_bailout(entity);
    }

    pub(super) fn cmd_place_bid(&mut self, auction_id: EntityId, amount: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Check player has funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < amount {
                return;
            }
        }

        if let Some(auction) = self.auctions.get_mut(&auction_id) {
            if auction.status != AuctionStatus::Open {
                return;
            }
            auction.place_bid(corp_id, amount);
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::AuctionBidPlaced {
                    auction: auction_id,
                    bidder: corp_id,
                    amount,
                },
            );
        }
    }

    // === Stock Market ===

    pub(super) fn cmd_buy_shares(&mut self, target_corp: EntityId, count: u32) {
        let buyer_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        if count == 0 { return; }

        // Verify target is public
        let sm = match self.stock_market.get_mut(&target_corp) {
            Some(sm) if sm.public => sm,
            _ => return,
        };

        let price = sm.share_price;
        let total_cost = price * count as i64;

        // Check funds
        if let Some(fin) = self.financials.get(&buyer_id) {
            if fin.cash < total_cost {
                return;
            }
        }

        // Execute transaction
        if let Some(fin) = self.financials.get_mut(&buyer_id) {
            fin.cash -= total_cost;
        }

        let sm = self.stock_market.get_mut(&target_corp).unwrap();
        *sm.shareholders.entry(buyer_id).or_insert(0) += count;
        
        // Increase price slightly on buy demand
        sm.share_price = (sm.share_price as f64 * 1.01) as i64;

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GlobalNotification {
                message: format!("Purchased {} shares of {} for ${}", count, target_corp, total_cost),
                level: "info".to_string(),
            },
        );
    }

    pub(super) fn cmd_sell_shares(&mut self, target_corp: EntityId, count: u32) {
        let seller_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        if count == 0 { return; }

        let sm = match self.stock_market.get_mut(&target_corp) {
            Some(sm) if sm.public => sm,
            _ => return,
        };

        // Check ownership
        let owned = sm.shareholders.get(&seller_id).copied().unwrap_or(0);
        if owned < count {
            return;
        }

        let price = sm.share_price;
        let total_sale = price * count as i64;

        // Execute transaction
        *sm.shareholders.entry(seller_id).or_insert(0) -= count;
        if sm.shareholders.get(&seller_id).copied().unwrap_or(0) == 0 {
            sm.shareholders.shift_remove(&seller_id);
        }

        // Decrease price slightly on sell pressure
        sm.share_price = (sm.share_price as f64 * 0.99).max(1.0) as i64;

        if let Some(fin) = self.financials.get_mut(&seller_id) {
            fin.cash += total_sale;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GlobalNotification {
                message: format!("Sold {} shares of {} for ${}", count, target_corp, total_sale),
                level: "info".to_string(),
            },
        );
    }

    // === Phase 10.2: Mergers & Acquisitions ===

    pub(super) fn cmd_propose_acquisition(&mut self, target: EntityId, offer: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        if !self.corporations.contains_key(&target) || target == corp_id {
            return;
        }

        // Check player has funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < offer {
                return;
            }
        }

        let proposal_id = self.allocate_entity();
        let proposal = AcquisitionProposal::new(corp_id, target, offer, self.tick);
        self.acquisition_proposals.insert(proposal_id, proposal);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::AcquisitionProposed {
                acquirer: corp_id,
                target,
                offer,
            },
        );
    }

    pub(super) fn cmd_respond_to_acquisition(&mut self, proposal_id: EntityId, accept: bool) {
        let proposal = match self.acquisition_proposals.get_mut(&proposal_id) {
            Some(p) => p,
            None => return,
        };

        if proposal.status != AcquisitionStatus::Pending {
            return;
        }

        if accept {
            proposal.status = AcquisitionStatus::Accepted;
            let acquirer = proposal.acquirer;
            let target = proposal.target;
            let offer = proposal.offer;

            // Transfer payment
            if let Some(fin) = self.financials.get_mut(&acquirer) {
                fin.cash -= offer;
            }

            // Transfer all assets from target to acquirer
            self.transfer_corporation_assets(target, acquirer);

            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::AcquisitionAccepted { acquirer, target },
            );
        } else {
            proposal.status = AcquisitionStatus::Rejected;
            let acquirer = proposal.acquirer;
            let target = proposal.target;
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::AcquisitionRejected { acquirer, target },
            );
        }
    }
}
