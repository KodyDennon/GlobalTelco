//! FTTH (Fiber To The Home) coverage system.
//!
//! Validates the access network hierarchy for each NAP (NetworkAccessPoint).
//! A NAP is marked as "active_ftth" when it has a valid chain back to a CO:
//!   CO <--FeederFiber--> FDH <--DistributionFiber--> NAP
//!
//! Active NAPs auto-cover nearby buildings (cells with city population) within
//! their coverage radius. This coverage feeds into the revenue system where
//! per-building subscriber revenue is calculated.

use std::collections::{HashMap, HashSet};

use gt_common::types::{EdgeType, EntityId, NodeType};

use crate::world::GameWorld;

/// Run the FTTH system: validate NAP chains and mark active NAPs.
///
/// Called each tick after coverage and before revenue in the system order.
pub fn run(world: &mut GameWorld) {
    // Step 1: Build adjacency structures for chain validation.
    // We need to trace: NAP <-DistributionFiber-> FDH <-FeederFiber-> CO

    // Collect all operational (non-under-construction) nodes by type
    let mut co_nodes: HashSet<EntityId> = HashSet::new();
    let mut fdh_nodes: HashSet<EntityId> = HashSet::new();
    let mut nap_node_ids: Vec<EntityId> = Vec::new();

    for (&id, node) in &world.infra_nodes {
        // Skip nodes still under construction
        if world.constructions.contains_key(&id) {
            continue;
        }
        match node.node_type {
            // Any Central Office type qualifies as a CO for the FTTH chain
            NodeType::CentralOffice => {
                co_nodes.insert(id);
            }
            NodeType::FiberDistributionHub => {
                fdh_nodes.insert(id);
            }
            NodeType::NetworkAccessPoint => {
                nap_node_ids.push(id);
            }
            _ => {}
        }
    }

    // Sort NAP IDs for deterministic processing
    nap_node_ids.sort_unstable();

    // Build edge adjacency maps filtered by edge type.
    // distribution_fiber_adj: node_id -> set of connected node_ids via DistributionFiber
    // feeder_fiber_adj: node_id -> set of connected node_ids via FeederFiber
    let mut distribution_fiber_adj: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    let mut feeder_fiber_adj: HashMap<EntityId, Vec<EntityId>> = HashMap::new();

    for edge in world.infra_edges.values() {
        match edge.edge_type {
            EdgeType::DistributionFiber => {
                distribution_fiber_adj
                    .entry(edge.source)
                    .or_default()
                    .push(edge.target);
                distribution_fiber_adj
                    .entry(edge.target)
                    .or_default()
                    .push(edge.source);
            }
            EdgeType::FeederFiber => {
                feeder_fiber_adj
                    .entry(edge.source)
                    .or_default()
                    .push(edge.target);
                feeder_fiber_adj
                    .entry(edge.target)
                    .or_default()
                    .push(edge.source);
            }
            _ => {}
        }
    }

    // Step 2: For each FDH, check if it connects to any CO via FeederFiber.
    // An FDH is "backhaul-valid" if any of its FeederFiber neighbors is a CO.
    let mut valid_fdhs: HashSet<EntityId> = HashSet::new();
    for &fdh_id in &fdh_nodes {
        if let Some(neighbors) = feeder_fiber_adj.get(&fdh_id) {
            for &neighbor_id in neighbors {
                if co_nodes.contains(&neighbor_id) {
                    valid_fdhs.insert(fdh_id);
                    break;
                }
            }
        }
    }

    // Step 3: For each NAP, check if it connects to a valid FDH via DistributionFiber.
    // A NAP is "active_ftth" if any of its DistributionFiber neighbors is a valid FDH.
    let mut active_naps: HashSet<EntityId> = HashSet::new();
    for &nap_id in &nap_node_ids {
        if let Some(neighbors) = distribution_fiber_adj.get(&nap_id) {
            for &neighbor_id in neighbors {
                if valid_fdhs.contains(&neighbor_id) {
                    active_naps.insert(nap_id);
                    break;
                }
            }
        }
    }

    // Step 4: Update the active_ftth flag on all NAP nodes.
    // Also clear the flag on any non-NAP nodes that might have it set from a previous tick.
    for (&id, node) in world.infra_nodes.iter_mut() {
        if node.node_type == NodeType::NetworkAccessPoint {
            node.active_ftth = active_naps.contains(&id);
        } else {
            // Ensure non-NAP nodes never have the flag set
            node.active_ftth = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::InfraEdge;
    use crate::components::InfraNode;

    /// Helper: create a minimal GameWorld for testing.
    fn make_test_world() -> GameWorld {
        let config = gt_common::types::WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            starting_era: gt_common::types::Era::Modern,
            difficulty: gt_common::types::DifficultyPreset::Normal,
            ai_corporations: 0,
            use_real_earth: false,
            corp_name: Some("TestCo".to_string()),
            continent_count: 2,
            ocean_percentage: 0.5,
            terrain_roughness: 0.5,
            climate_variation: 0.5,
            city_density: 0.5,
        };
        // Build a bare-bones world — we only need infra_nodes, infra_edges, constructions
        let mut world = GameWorld::new(config);
        // Clear generated data so we control the test precisely
        world.infra_nodes.clear();
        world.infra_edges.clear();
        world.constructions.clear();
        world
    }

    #[test]
    fn test_valid_ftth_chain_activates_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        // Create CO -> FeederFiber -> FDH -> DistributionFiber -> NAP
        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // FeederFiber: CO <-> FDH
        let feeder_edge_id = 100;
        world.infra_edges.insert(
            feeder_edge_id,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );

        // DistributionFiber: FDH <-> NAP
        let dist_edge_id = 101;
        world.infra_edges.insert(
            dist_edge_id,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with valid CO->FDH->NAP chain should be active_ftth"
        );
    }

    #[test]
    fn test_missing_co_does_not_activate_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        // FDH -> DistributionFiber -> NAP, but no CO connected
        let fdh_id = 20;
        let nap_id = 30;

        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        let dist_edge_id = 101;
        world.infra_edges.insert(
            dist_edge_id,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP without CO in chain should NOT be active_ftth"
        );
    }

    #[test]
    fn test_wrong_edge_type_does_not_activate_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // Use wrong edge type: FiberLocal instead of FeederFiber
        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FiberLocal, co_id, fdh_id, 1.0, corp),
        );

        // Correct distribution fiber
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with wrong edge type (FiberLocal instead of FeederFiber) should NOT be active_ftth"
        );
    }

    #[test]
    fn test_under_construction_co_does_not_validate() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // CO is under construction
        world.constructions.insert(
            co_id,
            crate::components::Construction::new(0, 10),
        );

        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with under-construction CO should NOT be active_ftth"
        );
    }

    #[test]
    fn test_nap_flag_clears_when_chain_breaks() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        // First run: should activate
        run(&mut world);
        assert!(world.infra_nodes.get(&nap_id).unwrap().active_ftth);

        // Remove the feeder fiber edge, breaking the chain
        world.infra_edges.remove(&100);

        // Second run: should deactivate
        run(&mut world);
        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP should lose active_ftth when chain breaks"
        );
    }
}
