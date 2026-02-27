use crate::systems::utilization::{build_transit_permissions, lookup_permission};
use crate::world::GameWorld;
use gt_common::types::TransitPermission;

pub fn run(world: &mut GameWorld) {
    if !world.network.has_dirty_nodes() {
        return;
    }

    // Build transit permission cache for ownership-aware routing
    let transit_perms = build_transit_permissions(world);

    // Build node ownership map: node_id → owner_corp_id
    let node_owners: std::collections::HashMap<u64, u64> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&id, node)| (id, node.owner))
        .collect();

    // Build edge weight function: weight = latency * (1 / bandwidth) — lower is better.
    // Also factor in edge health: damaged edges have much higher weight.
    // NEW: Add ownership penalty when crossing corporate boundaries without a contract.
    let edge_weights: std::collections::HashMap<(u64, u64), f64> = world
        .infra_edges
        .values()
        .map(|edge| {
            let effective_bw = edge.effective_bandwidth();
            let base_weight = if effective_bw <= 0.0 {
                f64::MAX // Edge is effectively destroyed
            } else {
                edge.latency_ms * (1.0 / effective_bw)
            };

            // Apply ownership penalty for cross-corp edges
            let weight = if base_weight >= f64::MAX {
                f64::MAX
            } else {
                let owner_a = node_owners.get(&edge.source).copied().unwrap_or(0);
                let owner_b = node_owners.get(&edge.target).copied().unwrap_or(0);

                if owner_a == 0 || owner_b == 0 || owner_a == owner_b {
                    // Same corp or unknown owner — no penalty
                    base_weight
                } else {
                    let (perm, _) = lookup_permission(&transit_perms, owner_a, owner_b);
                    match perm {
                        TransitPermission::OwnNetwork | TransitPermission::CoOwned => base_weight,
                        TransitPermission::PeeringContract
                        | TransitPermission::TransitContract { .. } => {
                            // Small penalty to prefer own network but allow cross-corp
                            base_weight * 1.1
                        }
                        TransitPermission::Alliance { .. } => {
                            // Moderate penalty — alliance is less reliable than contract
                            base_weight * 1.5
                        }
                        TransitPermission::Blocked => {
                            // No agreement — traffic cannot cross this boundary
                            f64::MAX
                        }
                    }
                }
            };

            let key = if edge.source < edge.target {
                (edge.source, edge.target)
            } else {
                (edge.target, edge.source)
            };
            (key, weight)
        })
        .collect();

    let weight_fn = |a: u64, b: u64| -> f64 {
        let key = if a < b { (a, b) } else { (b, a) };
        edge_weights.get(&key).copied().unwrap_or(f64::MAX)
    };

    world.network.recompute_dirty(&weight_fn);
}
