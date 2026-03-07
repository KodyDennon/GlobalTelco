use crate::components::AuctionStatus;
use crate::world::GameWorld;
use gt_common::types::EntityId;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only process auctions every 5 ticks
    if !tick.is_multiple_of(5) {
        return;
    }

    // Collect auction IDs that need processing
    let mut auctions_to_close: Vec<EntityId> = Vec::new();

    for (&auction_id, auction) in &world.auctions {
        if auction.status == AuctionStatus::Open && tick >= auction.end_tick {
            auctions_to_close.push(auction_id);
        }
    }
    auctions_to_close.sort_unstable();

    // Process expired auctions
    for auction_id in auctions_to_close {
        let auction = match world.auctions.get(&auction_id) {
            Some(a) => a.clone(),
            None => continue,
        };

        if let Some((_winner, winning_bid)) = auction.highest_bid() {
            let winner = _winner;

            // Check winner can still afford it
            let can_afford = world
                .financials
                .get(&winner)
                .map(|f| f.cash >= winning_bid)
                .unwrap_or(false);

            if can_afford {
                // Deduct payment
                if let Some(fin) = world.financials.get_mut(&winner) {
                    fin.cash -= winning_bid;
                }

                // Give proceeds to seller (or creditors)
                if let Some(fin) = world.financials.get_mut(&auction.seller) {
                    fin.cash += winning_bid;
                }

                // Transfer all assets to winner
                for &asset_id in &auction.assets {
                    // Transfer node ownership
                    if let Some(node) = world.infra_nodes.get_mut(&asset_id) {
                        let old_owner = node.owner;
                        node.owner = winner;

                        // Update corp_infra_nodes tracking
                        if let Some(old_nodes) = world.corp_infra_nodes.get_mut(&old_owner) {
                            old_nodes.retain(|&id| id != asset_id);
                        }
                        world
                            .corp_infra_nodes
                            .entry(winner)
                            .or_default()
                            .push(asset_id);
                    }
                    if let Some(own) = world.ownerships.get_mut(&asset_id) {
                        own.owner = winner;
                    }

                    world.event_queue.push(
                        tick,
                        gt_common::events::GameEvent::AuctionWon {
                            auction: auction_id,
                            asset: asset_id,
                            winner,
                            price: winning_bid / auction.assets.len().max(1) as i64,
                        },
                    );
                }

                // Also transfer edges owned by the seller
                for edge in world.infra_edges.values_mut() {
                    if edge.owner == auction.seller {
                        edge.owner = winner;
                    }
                }
            }
        } else {
            // No bids — cancel auction, assets remain with seller
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::AuctionCancelled {
                    auction: auction_id,
                },
            );
        }

        // Mark auction as closed
        if let Some(a) = world.auctions.get_mut(&auction_id) {
            a.status = AuctionStatus::Closed;
        }
    }

    // Clean up old closed auctions (older than 100 ticks)
    let to_remove: Vec<EntityId> = world
        .auctions
        .iter()
        .filter(|(_, a)| a.status != AuctionStatus::Open && tick > a.end_tick + 100)
        .map(|(&id, _)| id)
        .collect();
    for id in to_remove {
        world.auctions.shift_remove(&id);
    }
}
