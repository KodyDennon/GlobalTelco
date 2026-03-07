//! AI Satellite Operations.
//!
//! Handles satellite-specific AI behavior for the SatellitePioneer archetype
//! and opportunistic satellite building by other archetypes. Manages the full
//! satellite vertical: research, manufacturing, launch, ground stations, and
//! terminal distribution.
//!
//! Build order:
//!   1. Start satellite research (LEO Constellation)
//!   2. Build SatelliteFactory + TerminalFactory
//!   3. Build LEO_GroundStation connected to existing POP/backbone
//!   4. Build LaunchPad
//!   5. Order satellites → manufacture → launch constellation
//!   6. Build warehouses → distribute terminals → earn revenue
//!   7. Expand: more ground stations, more constellations, MEO/GEO

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

use super::helpers;

/// Run satellite AI logic for a corporation.
/// Called from the main AI orchestrator for SatellitePioneer corps,
/// and optionally for other archetypes with satellite ambitions.
pub fn run_satellite_ai(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // Only SatellitePioneer runs full satellite logic
    // Other archetypes might get a limited version later
    if ai.archetype != AIArchetype::SatellitePioneer {
        return;
    }

    // Phase 1: Ensure satellite research is underway
    ensure_satellite_research(world, corp_id, fin);

    // Phase 2: Build satellite infrastructure (factory, launch pad, ground station)
    build_satellite_infrastructure(world, corp_id, fin, tick);

    // Phase 3: Order satellites and schedule launches
    manage_satellite_production(world, corp_id, fin, tick);

    // Phase 4: Build terminal distribution chain
    manage_terminal_distribution(world, corp_id, fin, tick);

    // Phase 5: Service satellites that need it
    manage_satellite_servicing(world, corp_id, fin, tick);
}

// ─── Phase 1: Research ──────────────────────────────────────────────────────

fn ensure_satellite_research(world: &mut GameWorld, corp_id: EntityId, fin: &Financial) {
    // Check if we already have satellite research in progress or completed
    let has_satellite_research = world
        .tech_research
        .values()
        .any(|r| {
            r.category == ResearchCategory::Satellite
                && (r.researcher == Some(corp_id) || r.completed)
        });

    if has_satellite_research {
        return;
    }

    // Need at least 3M to start research
    if fin.cash < 3_000_000 {
        return;
    }

    let research_id = world.allocate_entity();
    let mut research = TechResearch::new(ResearchCategory::Satellite, "LEO Constellation Design");
    research.researcher = Some(corp_id);
    world.tech_research.insert(research_id, research);
}

// ─── Phase 2: Infrastructure ────────────────────────────────────────────────

fn build_satellite_infrastructure(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    tick: Tick,
) {
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let has_factory = corp_nodes.iter().any(|&nid| {
        world
            .infra_nodes
            .get(&nid)
            .map(|n| n.node_type == NodeType::SatelliteFactory)
            .unwrap_or(false)
    });
    let has_terminal_factory = corp_nodes.iter().any(|&nid| {
        world
            .infra_nodes
            .get(&nid)
            .map(|n| n.node_type == NodeType::TerminalFactory)
            .unwrap_or(false)
    });
    let has_ground_station = corp_nodes.iter().any(|&nid| {
        world
            .infra_nodes
            .get(&nid)
            .map(|n| {
                n.node_type == NodeType::LEO_GroundStation
                    || n.node_type == NodeType::MEO_GroundStation
            })
            .unwrap_or(false)
    });
    let has_launch_pad = corp_nodes.iter().any(|&nid| {
        world
            .infra_nodes
            .get(&nid)
            .map(|n| n.node_type == NodeType::LaunchPad)
            .unwrap_or(false)
    });

    // Need a backbone node first (standard AI building handles this)
    let has_backbone = helpers::has_node_at_or_above_tier(world, corp_id, NetworkTier::Core);
    if !has_backbone {
        return;
    }

    // Build in order: factory → ground station → launch pad → terminal factory
    if !has_factory && fin.cash > 150_000_000 {
        build_satellite_node(world, corp_id, NodeType::SatelliteFactory, &corp_nodes, tick);
    } else if !has_ground_station && fin.cash > 80_000_000 {
        // Ground station needs to be near an existing POP/backbone for backhaul
        build_ground_station(world, corp_id, &corp_nodes, tick);
    } else if !has_launch_pad && fin.cash > 300_000_000 {
        build_satellite_node(world, corp_id, NodeType::LaunchPad, &corp_nodes, tick);
    } else if !has_terminal_factory && fin.cash > 40_000_000 {
        build_satellite_node(world, corp_id, NodeType::TerminalFactory, &corp_nodes, tick);
    }
}

/// Build a satellite infrastructure node near the best city.
fn build_satellite_node(
    world: &mut GameWorld,
    corp_id: EntityId,
    node_type: NodeType,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Find a suitable city cell for placement
    let target_cell = find_best_infra_cell(world, corp_id);
    let Some(cell_index) = target_cell else {
        return;
    };

    if let Some(new_id) = helpers::build_node(world, corp_id, node_type, cell_index, tick) {
        // Connect to nearest existing node
        if !corp_nodes.is_empty() {
            if let Some(nearest) = helpers::find_nearest_node(world, corp_nodes, cell_index) {
                helpers::build_edge(world, corp_id, nearest, new_id, tick);
            }
        }
    }
}

/// Build a ground station near an existing POP/backbone for backhaul.
fn build_ground_station(
    world: &mut GameWorld,
    corp_id: EntityId,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Find a backbone/core node to place ground station near
    let backbone_cells: Vec<usize> = corp_nodes
        .iter()
        .filter_map(|&nid| {
            let node = world.infra_nodes.get(&nid)?;
            if (node.node_type.tier() as u8) >= (NetworkTier::Core as u8) {
                Some(node.cell_index)
            } else {
                None
            }
        })
        .collect();

    if backbone_cells.is_empty() {
        return;
    }

    // Use the first backbone node's cell (or a nearby cell)
    let cell_index = backbone_cells[0];

    if let Some(new_id) =
        helpers::build_node(world, corp_id, NodeType::LEO_GroundStation, cell_index, tick)
    {
        // Must connect to the backbone POP via fiber
        if let Some(nearest) =
            helpers::find_nearest_node_at_or_above_tier(world, corp_nodes, cell_index, NetworkTier::Core)
        {
            helpers::build_edge(world, corp_id, nearest, new_id, tick);
        }
    }
}

// ─── Phase 3: Satellite Production + Launch ─────────────────────────────────

fn manage_satellite_production(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    _tick: Tick,
) {
    // Check if we have the infrastructure to produce and launch
    let factory_id = find_corp_node_of_type(world, corp_id, NodeType::SatelliteFactory);
    let launch_pad_id = find_corp_node_of_type(world, corp_id, NodeType::LaunchPad);

    // Need both factory and launch pad, both completed construction
    let factory_ready = factory_id
        .map(|id| !world.constructions.contains_key(&id))
        .unwrap_or(false);
    let pad_ready = launch_pad_id
        .map(|id| !world.constructions.contains_key(&id))
        .unwrap_or(false);

    if !factory_ready || !pad_ready {
        return;
    }

    let factory_id = factory_id.unwrap();
    let launch_pad_id = launch_pad_id.unwrap();

    // Check if we have a constellation to build for
    let has_constellation = world
        .constellations
        .values()
        .any(|c| c.owner == corp_id);

    if !has_constellation && fin.cash > 50_000_000 {
        // Create a starter LEO constellation
        create_starter_constellation(world, corp_id);
    }

    // Find constellation needing satellites
    let constellation_data: Option<(EntityId, u32, u32)> = world
        .constellations
        .iter()
        .find(|(_, c)| c.owner == corp_id && c.operational_count < c.num_planes * c.sats_per_plane)
        .map(|(&id, c)| (id, c.num_planes * c.sats_per_plane, c.operational_count));

    let Some((constellation_id, target_count, current_count)) = constellation_data else {
        return;
    };

    // Check if factory already has orders queued
    let factory_has_queue = world
        .satellite_factories
        .get(&factory_id)
        .map(|f| !f.queue.is_empty())
        .unwrap_or(false);

    if !factory_has_queue && current_count < target_count && fin.cash > 20_000_000 {
        // Order a batch of satellites (up to 10 at a time)
        let batch_size = (target_count - current_count).min(10);
        if let Some(factory) = world.satellite_factories.get_mut(&factory_id) {
            factory.queue.push((constellation_id, batch_size));
        }
    }

    // Check for satellites awaiting launch
    let awaiting_launch: Vec<EntityId> = world
        .satellites
        .iter()
        .filter(|(_, sat)| {
            sat.status == SatelliteStatus::AwaitingLaunch && sat.constellation_id == constellation_id
        })
        .filter(|(sat_id, _)| {
            world
                .ownerships
                .get(*sat_id)
                .map(|o| o.owner == corp_id)
                .unwrap_or(false)
        })
        .map(|(id, _)| *id)
        .collect();

    if awaiting_launch.is_empty() {
        return;
    }


    // Check launch pad cooldown
    let pad_ready_to_launch = world
        .launch_pads
        .get(&launch_pad_id)
        .map(|lp| lp.cooldown_remaining == 0 && lp.launch_queue.is_empty())
        .unwrap_or(false);

    if !pad_ready_to_launch {
        return;
    }

    // Schedule a launch with up to 20 satellites
    let launch_batch: Vec<EntityId> = awaiting_launch.into_iter().take(20).collect();
    let rocket_type = if launch_batch.len() > 10 {
        "Heavy"
    } else if launch_batch.len() > 5 {
        "Medium"
    } else {
        "Small"
    };

    if let Some(lp) = world.launch_pads.get_mut(&launch_pad_id) {
        lp.launch_queue
            .push((rocket_type.to_string(), launch_batch));
    }
}

fn create_starter_constellation(world: &mut GameWorld, corp_id: EntityId) {
    let constellation_id = world.allocate_entity();
    let constellation = Constellation {
        name: format!("SatNet-{}", corp_id),
        owner: corp_id,
        orbit_type: OrbitType::LEO,
        target_altitude_km: 550.0,
        target_inclination_deg: 53.0,
        num_planes: 6,
        sats_per_plane: 22,
        satellite_ids: Vec::new(),
        operational_count: 0,
    };
    world.constellations.insert(constellation_id, constellation);
}

// ─── Phase 4: Terminal Distribution ─────────────────────────────────────────

fn manage_terminal_distribution(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    tick: Tick,
) {
    // Check for terminal factory
    let terminal_factory_id = find_corp_node_of_type(world, corp_id, NodeType::TerminalFactory);
    let factory_ready = terminal_factory_id
        .map(|id| !world.constructions.contains_key(&id))
        .unwrap_or(false);

    if !factory_ready {
        return;
    }

    // Check if we have warehouses
    let warehouse_count = world
        .warehouses
        .values()
        .filter(|w| w.owner == corp_id)
        .count();

    // Build warehouses in underserved regions with satellite coverage
    if warehouse_count == 0 && fin.cash > 10_000_000 {
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();
        build_satellite_node(world, corp_id, NodeType::SatelliteWarehouse, &corp_nodes, tick);
    }
}

// ─── Phase 5: Satellite Servicing ───────────────────────────────────────────

fn manage_satellite_servicing(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    tick: Tick,
) {
    // Find satellites that need servicing (low fuel or decaying)
    let needs_service: Vec<(EntityId, ServiceType)> = world
        .satellites
        .iter()
        .filter(|(sat_id, _)| {
            world
                .ownerships
                .get(*sat_id)
                .map(|o| o.owner == corp_id)
                .unwrap_or(false)
        })
        .filter_map(|(sat_id, sat)| {
            if sat.status == SatelliteStatus::Operational && sat.fuel_remaining < sat.fuel_capacity * 0.2 {
                Some((*sat_id, ServiceType::Refuel))
            } else if sat.status == SatelliteStatus::Decaying {
                Some((*sat_id, ServiceType::Refuel))
            } else {
                // Check health
                let low_health = world
                    .healths
                    .get(sat_id)
                    .map(|h| h.condition < 0.3)
                    .unwrap_or(false);
                if low_health {
                    Some((*sat_id, ServiceType::Repair))
                } else {
                    None
                }
            }
        })
        .collect();

    // Already have missions in progress?
    let active_missions = world
        .service_missions
        .iter()
        .filter(|m| {
            world
                .ownerships
                .get(&m.satellite_id)
                .map(|o| o.owner == corp_id)
                .unwrap_or(false)
        })
        .count();

    // Limit concurrent service missions
    if active_missions >= 3 {
        return;
    }

    for (sat_id, service_type) in needs_service.into_iter().take(1) {
        // Don't service if already being serviced
        let already_servicing = world
            .service_missions
            .iter()
            .any(|m| m.satellite_id == sat_id);
        if already_servicing {
            continue;
        }

        let cost = match service_type {
            ServiceType::Refuel => 500_000,
            ServiceType::Repair => 1_000_000,
        };

        if fin.cash < cost * 2 {
            continue;
        }

        world.service_missions.push(ServiceMission {
            satellite_id: sat_id,
            service_type,
            ticks_remaining: 10,
            cost,
        });

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::SatelliteServiced {
                satellite_id: sat_id,
                service_type: format!("{:?}", service_type),
                cost,
            },
        );
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Find a corp's node of a specific type.
fn find_corp_node_of_type(
    world: &GameWorld,
    corp_id: EntityId,
    node_type: NodeType,
) -> Option<EntityId> {
    world
        .corp_infra_nodes
        .get(&corp_id)?
        .iter()
        .find(|&&nid| {
            world
                .infra_nodes
                .get(&nid)
                .map(|n| n.node_type == node_type)
                .unwrap_or(false)
        })
        .copied()
}

/// Find the best cell for placing satellite infrastructure.
/// Prefers cells near the corp's existing backbone, in high-population regions.
fn find_best_infra_cell(world: &GameWorld, corp_id: EntityId) -> Option<usize> {
    // Use the cell of the corp's most central backbone node
    let corp_nodes = world.corp_infra_nodes.get(&corp_id)?;

    // Try to find a core/backbone node first
    for &nid in corp_nodes {
        if let Some(node) = world.infra_nodes.get(&nid) {
            if (node.node_type.tier() as u8) >= (NetworkTier::Core as u8)
                && !world.constructions.contains_key(&nid)
            {
                return Some(node.cell_index);
            }
        }
    }

    // Fallback: any operational node
    for &nid in corp_nodes {
        if let Some(node) = world.infra_nodes.get(&nid) {
            if !world.constructions.contains_key(&nid) {
                return Some(node.cell_index);
            }
        }
    }

    None
}
