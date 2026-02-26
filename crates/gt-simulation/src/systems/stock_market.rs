use crate::components::StockMarket;
use crate::world::GameWorld;
use gt_common::types::EntityId;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 10 ticks for performance
    if !tick.is_multiple_of(10) {
        return;
    }

    let mut corp_ids: Vec<EntityId> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    // First pass: check for auto-IPO on private corporations
    for &corp_id in &corp_ids {
        let is_public = world
            .stock_market
            .get(&corp_id)
            .map(|sm| sm.public)
            .unwrap_or(false);

        if !is_public {
            // Auto-IPO: corporation needs >= 50 nodes and cash >= 1,000,000
            let node_count = world
                .corp_infra_nodes
                .get(&corp_id)
                .map(|n| n.len())
                .unwrap_or(0);
            let cash = world
                .financials
                .get(&corp_id)
                .map(|f| f.cash)
                .unwrap_or(0);

            if node_count >= 50 && cash >= 1_000_000 {
                let sm = world
                    .stock_market
                    .entry(corp_id)
                    .or_insert_with(StockMarket::new);
                sm.public = true;
                sm.ipo_tick = Some(tick);
                // Set initial share price based on cash
                sm.share_price = (cash / sm.total_shares as i64).max(1);
            }
        }
    }

    // Second pass: update share prices and dividends for public corporations
    for &corp_id in &corp_ids {
        let sm = match world.stock_market.get(&corp_id) {
            Some(sm) if sm.public => sm.clone(),
            _ => continue,
        };

        let fin = match world.financials.get(&corp_id) {
            Some(f) => f.clone(),
            None => continue,
        };

        let node_count = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|n| n.len())
            .unwrap_or(0) as i64;

        let reputation = world
            .corporations
            .get(&corp_id)
            .map(|c| c.reputation)
            .unwrap_or(50.0);

        let total_shares = sm.total_shares as i64;
        let net_profit = fin.revenue_per_tick - fin.cost_per_tick;

        // Formula: base_price + (cash / total_shares) * 0.1 + (net_profit * 10) / total_shares
        let base_price = (node_count * 10) + (reputation as i64);
        let cash_component = (fin.cash / total_shares) / 10; // * 0.1
        let profit_component = (net_profit * 10) / total_shares;
        let new_price = (base_price + cash_component + profit_component).max(1);

        let prev_price = sm.share_price;

        // Calculate dividends: if profitable and cash reserves are healthy
        let maintenance_cost = fin.cost_per_tick;
        let new_dividends = if net_profit > 0 && fin.cash > maintenance_cost * 20 {
            // Pay 10% of net_profit as dividends, divided by total shares
            (net_profit / 10) / total_shares
        } else {
            0
        };

        // Update shareholder satisfaction based on price trend and dividends
        let price_trend = if prev_price > 0 {
            (new_price - prev_price) as f64 / prev_price as f64
        } else {
            0.0
        };

        let prev_satisfaction = sm.shareholder_satisfaction;
        // Satisfaction moves toward a target based on price trend and dividends
        let dividend_bonus = if new_dividends > 0 { 0.1 } else { 0.0 };
        let target_satisfaction = (0.5 + price_trend + dividend_bonus).clamp(0.0, 1.0);
        // Smooth toward target (20% per update)
        let new_satisfaction =
            (prev_satisfaction + (target_satisfaction - prev_satisfaction) * 0.2).clamp(0.0, 1.0);

        // Expire completed board votes
        let mut updated_votes: Vec<crate::components::stock_market::BoardVote> = sm
            .board_votes
            .iter()
            .filter(|v| v.deadline_tick > tick)
            .cloned()
            .collect();
        updated_votes.truncate(10); // Limit to 10 active votes

        // Apply updates
        if let Some(sm) = world.stock_market.get_mut(&corp_id) {
            sm.share_price = new_price;
            sm.dividends_per_share = new_dividends;
            sm.shareholder_satisfaction = new_satisfaction;
            sm.board_votes = updated_votes;
        }
    }
}
