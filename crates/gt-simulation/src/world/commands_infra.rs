use gt_common::types::*;

use crate::components::*;

use super::GameWorld;

impl GameWorld {
    pub(super) fn cmd_build_node(
        &mut self,
        node_type: NodeType,
        lon: f64,
        lat: f64,
    ) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::{CommandResult, DeltaOp};

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot build: No corporation assigned.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("No corporation assigned");
            }
        };

        // Find nearest grid cell for terrain/region lookup
        let (cell_index, _dist) = match self.find_nearest_cell(lon, lat) {
            Some(result) => result,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot build: No valid location found.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("No valid location found");
            }
        };

        // Get terrain from nearest cell's parcel
        let terrain = match self.get_cell_terrain(cell_index) {
            Some(t) => t,
            None => TerrainType::Rural, // fallback if no parcel data
        };

        // Terrain constraints for node types
        // Categorize: subsea/coastal-only, land-only, water-tolerant, ocean-floor
        match node_type {
            // Landing stations require coastal terrain specifically
            NodeType::SubmarineLanding | NodeType::SubseaLandingStation => {
                if !matches!(terrain, TerrainType::Coastal) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Landing stations require coastal terrain".to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Landing stations require coastal terrain");
                }
            }
            // Cable huts can be placed on coastal or shallow ocean
            NodeType::CableHut => {
                if !matches!(terrain, TerrainType::Coastal | TerrainType::OceanShallow) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: format!(
                                "Cannot build {}: requires coastal or shallow ocean terrain.",
                                node_type.display_name()
                            ),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Requires coastal or shallow ocean terrain");
                }
            }
            // Underwater data center requires deep ocean
            NodeType::UnderwaterDataCenter => {
                if !matches!(terrain, TerrainType::OceanDeep | TerrainType::OceanShallow | TerrainType::OceanTrench) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Underwater data center requires ocean terrain."
                                .to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Underwater data center requires ocean terrain");
                }
            }
            // Satellite/drone nodes — any non-deep-ocean terrain
            NodeType::SatelliteGround
            | NodeType::SatelliteGroundStation
            | NodeType::LEO_SatelliteGateway
            | NodeType::MeshDroneRelay => {
                if matches!(terrain, TerrainType::OceanDeep | TerrainType::OceanTrench) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: format!(
                                "Cannot build {}: cannot be placed on deep ocean.",
                                node_type.display_name()
                            ),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Cannot be placed on deep ocean");
                }
            }
            // All other nodes require solid ground (no ocean)
            _ => {
                if matches!(terrain, TerrainType::OceanDeep | TerrainType::OceanShallow | TerrainType::OceanTrench) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: format!(
                                "Cannot build {}: Requires solid ground.",
                                node_type.display_name()
                            ),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Requires solid ground");
                }
            }
        }

        let node = InfraNode::new_on_terrain(node_type, cell_index, corp_id, terrain);
        let cost = node.construction_cost;

        // Check funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "Insufficient funds: {:?} costs ${}, you have ${}.",
                            node_type, cost, fin.cash
                        ),
                        level: "warning".to_string(),
                    },
                );
                return CommandResult::fail("Insufficient funds");
            }
        } else {
            return CommandResult::fail("Corporation financials not found");
        }

        // Deduct cost
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        // Charge PerUnit license fees for any patented tech the builder is licensed to use
        self.charge_per_unit_fees(corp_id);

        // Create construction (takes time)
        let difficulty = gt_common::config::DifficultyConfig::from_preset(self.config.difficulty);
        let base_duration = match node_type {
            // Original 8 (unchanged)
            NodeType::CellTower | NodeType::WirelessRelay => 10,
            NodeType::CentralOffice => 20,
            NodeType::ExchangePoint => 30,
            NodeType::DataCenter | NodeType::BackboneRouter => 50,
            NodeType::SatelliteGround => 40,
            NodeType::SubmarineLanding => 60,
            // Era 1: Telegraph — fast
            NodeType::TelegraphOffice => 5,
            NodeType::TelegraphRelay => 3,
            NodeType::CableHut => 8,
            // Era 2: Telephone
            NodeType::ManualExchange => 10,
            NodeType::AutomaticExchange => 15,
            NodeType::TelephonePole => 2,
            NodeType::LongDistanceRelay => 12,
            // Era 3: Early Digital
            NodeType::DigitalSwitch => 20,
            NodeType::MicrowaveTower => 15,
            NodeType::CoaxHub => 10,
            NodeType::EarlyDataCenter => 40,
            NodeType::SatelliteGroundStation => 35,
            // Era 4: Internet
            NodeType::FiberPOP => 25,
            NodeType::InternetExchangePoint => 35,
            NodeType::SubseaLandingStation => 70,
            NodeType::ColocationFacility => 45,
            NodeType::ISPGateway => 15,
            // Era 5: Modern
            NodeType::MacroCell => 12,
            NodeType::SmallCell => 5,
            NodeType::EdgeDataCenter => 30,
            NodeType::HyperscaleDataCenter => 120,
            NodeType::CloudOnRamp => 25,
            NodeType::ContentDeliveryNode => 20,
            NodeType::FiberSplicePoint => 2,
            NodeType::DWDM_Terminal => 35,
            NodeType::FiberDistributionHub => 5,
            NodeType::NetworkAccessPoint => 3,
            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 80,
            NodeType::QuantumRepeater => 60,
            NodeType::MeshDroneRelay => 5,
            NodeType::UnderwaterDataCenter => 150,
            NodeType::NeuromorphicEdgeNode => 40,
            NodeType::TerahertzRelay => 10,
            // Satellite infrastructure
            NodeType::LEO_Satellite | NodeType::MEO_Satellite
            | NodeType::GEO_Satellite | NodeType::HEO_Satellite => 0, // Manufactured, not built
            NodeType::LEO_GroundStation => 60,
            NodeType::MEO_GroundStation => 50,
            NodeType::SatelliteFactory => 80,
            NodeType::TerminalFactory => 40,
            NodeType::SatelliteWarehouse => 20,
            NodeType::LaunchPad => 100,
            NodeType::Building => 0,
        };
        let duration = (base_duration as f64 * difficulty.construction_time_multiplier) as Tick;

        let node_id = self.allocate_entity();
        let network_level = node.network_level;
        let maintenance = node.maintenance_cost;
        self.infra_nodes.insert(node_id, node);
        self.constructions
            .insert(node_id, Construction::new(self.tick, duration));
        self.ownerships.insert(node_id, Ownership::sole(corp_id));
        self.healths.insert(node_id, Health::new());
        self.capacities.insert(node_id, Capacity::new(0.0)); // 0 until construction completes

        // Position at exact clicked coordinates (free placement)
        let region_id = self.cell_to_region.get(&cell_index).copied();
        self.positions.insert(
            node_id,
            Position {
                x: lon,
                y: lat,
                region_id,
            },
        );

        self.corp_infra_nodes
            .entry(corp_id)
            .or_default()
            .push(node_id);

        // Update maintenance costs
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cost_per_tick += maintenance;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ConstructionStarted {
                entity: node_id,
                tick: self.tick,
            },
        );

        CommandResult::ok_with_entity(node_id).with_op(DeltaOp::NodeCreated {
            entity_id: node_id,
            owner: corp_id,
            node_type,
            network_level,
            lon,
            lat,
            under_construction: true,
        })
    }

    pub(super) fn cmd_build_edge(
        &mut self,
        edge_type: EdgeType,
        from_node: EntityId,
        to_node: EntityId,
        waypoints: Vec<(f64, f64)>,
        deployment: Option<String>,
    ) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::{CommandResult, DeltaOp};

        // Get corp from either node
        let corp_id = match self.infra_nodes.get(&from_node) {
            Some(n) => n.owner,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot connect: Source node not found.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("Source node not found");
            }
        };

        // Verify target node exists and belongs to the same corp (or is a neutral Building)
        match self.infra_nodes.get(&to_node) {
            Some(n) if n.owner == corp_id || n.node_type == NodeType::Building => {}
            Some(_) => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot connect: Target node belongs to a different corporation."
                            .to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("Target node belongs to a different corporation");
            }
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot connect: Target node not found.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("Target node not found");
            }
        }

        // Enforce tier compatibility: edge type must be valid for the node tiers
        let from_type = self.infra_nodes.get(&from_node).map(|n| n.node_type);
        let to_type = self.infra_nodes.get(&to_node).map(|n| n.node_type);
        if let (Some(ft), Some(tt)) = (from_type, to_type) {
            if !edge_type.can_connect(ft, tt) {
                // Build a suggestion of which edge types CAN connect these two node types
                let all_types = [
                    EdgeType::Copper,
                    EdgeType::FiberLocal,
                    EdgeType::Microwave,
                    EdgeType::FiberRegional,
                    EdgeType::FiberNational,
                    EdgeType::Satellite,
                    EdgeType::Submarine,
                ];
                let suggestions: Vec<&str> = all_types
                    .iter()
                    .filter(|et| et.can_connect(ft, tt))
                    .map(|et| et.display_name())
                    .collect();
                let hint = if suggestions.is_empty() {
                    format!(
                        "{} and {} are too far apart in tier — build intermediate nodes.",
                        ft.display_name(),
                        tt.display_name()
                    )
                } else {
                    format!("Try: {}", suggestions.join(", "))
                };
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "{} cannot connect {} ({}) to {} ({}). {}",
                            edge_type.display_name(),
                            ft.display_name(),
                            ft.tier().display_name(),
                            tt.display_name(),
                            tt.tier().display_name(),
                            hint
                        ),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("Edge type incompatible with node tiers");
            }
        }

        // Calculate distance between nodes
        let from_pos = self.positions.get(&from_node);
        let to_pos = self.positions.get(&to_node);
        let length_km = match (from_pos, to_pos) {
            (Some(a), Some(b)) => {
                let dlat = (a.y - b.y).to_radians();
                let dlon = (a.x - b.x).to_radians();
                let lat1 = a.y.to_radians();
                let lat2 = b.y.to_radians();
                let a_val = (dlat / 2.0).sin().powi(2)
                    + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
                let c = 2.0 * a_val.sqrt().asin();
                6371.0 * c // Earth radius in km
            }
            _ => 100.0, // Default
        };

        // Enforce max distance based on edge type, scaled to grid resolution.
        let max_distance_km = self.cell_spacing_km * edge_type.distance_multiplier();
        if length_km > max_distance_km {
            // Suggest a longer-range edge type that can still connect these tiers
            let from_ft = self.infra_nodes.get(&from_node).map(|n| n.node_type);
            let to_ft = self.infra_nodes.get(&to_node).map(|n| n.node_type);
            let all_types = [
                EdgeType::Copper,
                EdgeType::FiberLocal,
                EdgeType::Microwave,
                EdgeType::FiberRegional,
                EdgeType::FiberNational,
                EdgeType::Satellite,
                EdgeType::Submarine,
            ];
            let suggestion = if let (Some(ft), Some(tt)) = (from_ft, to_ft) {
                all_types
                    .iter()
                    .filter(|et| {
                        et.can_connect(ft, tt)
                            && self.cell_spacing_km * et.distance_multiplier() >= length_km
                    })
                    .map(|et| {
                        format!(
                            "{} ({:.0}km range)",
                            et.display_name(),
                            self.cell_spacing_km * et.distance_multiplier()
                        )
                    })
                    .next()
                    .map(|s| format!(" Try: {}", s))
                    .unwrap_or_else(|| {
                        " Build intermediate relay nodes to bridge the gap.".to_string()
                    })
            } else {
                String::new()
            };
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!(
                        "Too far for {}: {:.0}km distance, {:.0}km max range.{}",
                        edge_type.display_name(),
                        length_km,
                        max_distance_km,
                        suggestion
                    ),
                    level: "warning".to_string(),
                },
            );
            return CommandResult::fail("Distance exceeds max range for edge type");
        }

        // Enforce terrain constraints: check source/target terrain
        // Build cell_index → terrain lookup (O(N) instead of O(N²) scanning)
        let cell_terrain: std::collections::HashMap<usize, TerrainType> = self
            .land_parcels
            .values()
            .map(|p| (p.cell_index, p.terrain))
            .collect();
        let from_terrain = self
            .infra_nodes
            .get(&from_node)
            .and_then(|n| cell_terrain.get(&n.cell_index).copied());
        let to_terrain = self
            .infra_nodes
            .get(&to_node)
            .and_then(|n| cell_terrain.get(&n.cell_index).copied());

        match edge_type {
            // Subsea cables require at least one endpoint on ocean/coastal
            EdgeType::Submarine | EdgeType::SubseaTelegraphCable | EdgeType::SubseaFiberCable => {
                let is_water = |t: Option<TerrainType>| {
                    matches!(
                        t,
                        Some(
                            TerrainType::OceanShallow
                                | TerrainType::OceanDeep
                                | TerrainType::OceanTrench
                                | TerrainType::Coastal
                        )
                    )
                };
                if !is_water(from_terrain) && !is_water(to_terrain) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: format!(
                                "Cannot build: {} requires at least one endpoint on water.",
                                edge_type.display_name()
                            ),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Subsea edges require water");
                }
            }
            // Land-based cables/wires can't span deep ocean
            EdgeType::Copper
            | EdgeType::FiberLocal
            | EdgeType::FiberRegional
            | EdgeType::FiberNational
            | EdgeType::TelegraphWire
            | EdgeType::CopperTrunkLine
            | EdgeType::LongDistanceCopper
            | EdgeType::CoaxialCable
            | EdgeType::FiberMetro
            | EdgeType::FiberLongHaul
            | EdgeType::DWDM_Backbone
            | EdgeType::FeederFiber
            | EdgeType::DistributionFiber
            | EdgeType::DropCable
            | EdgeType::QuantumFiberLink => {
                let is_deep_ocean =
                    |t: Option<TerrainType>| matches!(t, Some(TerrainType::OceanDeep | TerrainType::OceanTrench));
                if is_deep_ocean(from_terrain) || is_deep_ocean(to_terrain) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Wired cables cannot be placed in deep ocean."
                                .to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return CommandResult::fail("Wired cables cannot be placed in deep ocean");
                }
            }
            // Microwave, Satellite, Terahertz, Laser have no terrain restriction
            _ => {}
        }

        // Cable ship constraint: submarine cable construction requires available cable ships
        if matches!(
            edge_type,
            EdgeType::Submarine | EdgeType::SubseaTelegraphCable | EdgeType::SubseaFiberCable
        ) {
            let cable_ship_count = self.cable_ships.get(&corp_id).copied().unwrap_or(0);
            if cable_ship_count == 0 {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "You need to purchase a cable ship first".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("You need to purchase a cable ship first");
            }

            // Count active submarine constructions for this corporation
            let active_submarine_constructions = self
                .active_submarine_builds
                .values()
                .filter(|&&owner| owner == corp_id)
                .count() as u32;

            if active_submarine_constructions >= cable_ship_count {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "No available cable ships".to_string(),
                        level: "warning".to_string(),
                    },
                );
                return CommandResult::fail("No available cable ships");
            }
        }

        // DropCable constraint: must connect FROM an active FTTH NAP to a target.
        // The source (or target) must be a NetworkAccessPoint with active_ftth == true.
        if edge_type == EdgeType::DropCable {
            let from_node_data = self.infra_nodes.get(&from_node);
            let to_node_data = self.infra_nodes.get(&to_node);

            // At least one endpoint must be a NAP
            let from_is_nap = from_node_data
                .map(|n| n.node_type == NodeType::NetworkAccessPoint)
                .unwrap_or(false);
            let to_is_nap = to_node_data
                .map(|n| n.node_type == NodeType::NetworkAccessPoint)
                .unwrap_or(false);

            if !from_is_nap && !to_is_nap {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Drop Cable must connect from a Network Access Point.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("Drop Cable requires a NAP endpoint");
            }

            // The NAP endpoint must have an active FTTH chain
            let nap_is_active = if from_is_nap {
                from_node_data.map(|n| n.active_ftth).unwrap_or(false)
            } else {
                to_node_data.map(|n| n.active_ftth).unwrap_or(false)
            };

            if !nap_is_active {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Drop Cable requires an active FTTH chain (CO -> FDH -> NAP). Connect the NAP to a Fiber Distribution Hub first.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return CommandResult::fail("NAP must have active FTTH chain for Drop Cable");
            }
        }

        // Network contiguity: new edge must connect to existing corp network
        // (exempt if corp has fewer than 2 nodes — first edge always allowed)
        let corp_nodes = self
            .corp_infra_nodes
            .get(&corp_id)
            .map(|v| v.len())
            .unwrap_or(0);
        if corp_nodes > 1 {
            let connected = self.network.connected_nodes(from_node);
            if !connected.contains(&from_node) && !connected.contains(&to_node) {
                // Neither node is in an existing network component — allow it
                // (this can happen with isolated node pairs)
            }
            // Note: we allow connecting two separate network components owned by same corp
        }

        let mut edge = InfraEdge::new(edge_type, from_node, to_node, length_km, corp_id);
        if !waypoints.is_empty() {
            edge.waypoints = waypoints;
        }
        if let Some(ref d) = deployment {
            match d.as_str() {
                "Underground" => {
                    edge.deployment = crate::components::infra_edge::DeploymentMethod::Underground
                }
                _ => edge.deployment = crate::components::infra_edge::DeploymentMethod::Aerial,
            }
        }
        let cost = edge.construction_cost;
        let maintenance = edge.maintenance_cost;

        // Check funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "Insufficient funds: {} costs ${}, you have ${}.",
                            edge_type.display_name(),
                            cost,
                            fin.cash
                        ),
                        level: "warning".to_string(),
                    },
                );
                return CommandResult::fail("Insufficient funds");
            }
        } else {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "Internal Error: Corporation financials not found.".to_string(),
                    level: "error".to_string(),
                },
            );
            return CommandResult::fail("Corporation financials not found");
        }

        // Deduct cost
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let edge_id = self.allocate_entity();
        self.infra_edges.insert(edge_id, edge);
        self.network.add_edge_with_id(from_node, to_node, edge_id);

        // Submarine cables get a construction phase (cable ship must lay the cable).
        // Build time scales with cable length: base 80 ticks + 2 ticks per 100 km.
        if matches!(
            edge_type,
            EdgeType::Submarine | EdgeType::SubseaTelegraphCable | EdgeType::SubseaFiberCable
        ) {
            let build_ticks = 80 + (length_km / 100.0 * 2.0) as u64;
            self.constructions
                .insert(edge_id, Construction::new(self.tick, build_ticks));
            // Track in active_submarine_builds: edge_id → corp_id
            self.active_submarine_builds.insert(edge_id, corp_id);
        }

        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cost_per_tick += maintenance;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::EdgeBuilt {
                entity: edge_id,
                from: from_node,
                to: to_node,
            },
        );

        CommandResult::ok_with_entity(edge_id).with_op(DeltaOp::EdgeCreated {
            entity_id: edge_id,
            owner: corp_id,
            edge_type,
            from_node,
            to_node,
        })
    }

    pub(super) fn cmd_update_edge_waypoints(
        &mut self,
        edge_id: EntityId,
        waypoints: Vec<(f64, f64)>,
        deployment: Option<String>,
    ) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::CommandResult;

        // Verify the edge exists
        let edge = match self.infra_edges.get(&edge_id) {
            Some(e) => e,
            None => return CommandResult::fail("Edge not found"),
        };

        // Verify ownership: the player's corporation must own this edge
        let edge_owner = edge.owner;
        if let Some(player_corp) = self.player_corp_id() {
            if edge_owner != player_corp {
                return CommandResult::fail("You do not own this edge");
            }
        }

        // Update the edge
        if let Some(edge) = self.infra_edges.get_mut(&edge_id) {
            edge.waypoints = waypoints;
            if let Some(ref d) = deployment {
                match d.as_str() {
                    "Underground" => {
                        edge.deployment =
                            crate::components::infra_edge::DeploymentMethod::Underground
                    }
                    _ => edge.deployment = crate::components::infra_edge::DeploymentMethod::Aerial,
                }
            }
        }

        CommandResult::ok()
    }

    pub(super) fn cmd_upgrade_node(&mut self, entity: EntityId) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::{CommandResult, DeltaOp};

        let _node_owner = match self.infra_nodes.get(&entity) {
            Some(n) => n.owner,
            None => return CommandResult::fail("Node not found"),
        };

        let player_corp = match self.player_corp_id {
            Some(id) => id,
            None => return CommandResult::fail("Not authenticated"),
        };

        // 1. Check Ownership & Co-ownership
        let ownership = match self.ownerships.get(&entity) {
            Some(o) => o,
            None => return CommandResult::fail("Ownership data missing"),
        };

        // Must have some stake to propose upgrade
        let has_stake = ownership.owner == player_corp || ownership.co_owners.iter().any(|(id, _)| *id == player_corp);
        if !has_stake {
            return CommandResult::fail("You do not have an ownership stake in this node");
        }

        // 2. Co-ownership Voting logic
        if !ownership.co_owners.is_empty() {
            // Proposed for co-owned node: start a vote
            if self.pending_upgrade_votes.contains_key(&entity) {
                return CommandResult::fail("Upgrade vote already in progress for this node");
            }

            let mut votes = std::collections::HashMap::new();
            votes.insert(player_corp, true); // Proposer automatically votes YES

            self.pending_upgrade_votes.insert(entity, (player_corp, votes, self.tick));

            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!("Upgrade proposed for co-owned node {}. Voting started.", entity),
                    level: "info".to_string(),
                },
            );

            return CommandResult::ok();
        }

        // 3. Sole ownership: execute immediate upgrade
        let upgrade_cost = match self.infra_nodes.get(&entity) {
            Some(n) => n.construction_cost / 2,
            None => return CommandResult::fail("Node not found"),
        };

        if let Some(fin) = self.financials.get(&player_corp) {
            if fin.cash < upgrade_cost {
                return CommandResult::fail("Insufficient funds");
            }
        }

        if let Some(fin) = self.financials.get_mut(&player_corp) {
            fin.cash -= upgrade_cost;
        }

        if let Some(node) = self.infra_nodes.get_mut(&entity) {
            node.max_throughput *= 1.5;
            node.reliability = (node.reliability + 0.05).min(1.0);
        }
        if let Some(cap) = self.capacities.get_mut(&entity) {
            cap.max_throughput *= 1.5;
        }
        if let Some(health) = self.healths.get_mut(&entity) {
            health.condition = 1.0;
        }

        let node_type = self.infra_nodes.get(&entity).map(|n| n.node_type).unwrap();
        CommandResult::ok_with_entity(entity).with_op(DeltaOp::NodeUpgraded {
            entity_id: entity,
            node_type,
        })
    }

    pub(super) fn cmd_decommission_node(&mut self, entity: EntityId) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::{CommandResult, DeltaOp};

        if let Some(node) = self.infra_nodes.remove(&entity) {
            let corp_id = node.owner;
            // Remove from network
            self.network.remove_node(entity);
            // Remove associated edges — collect delta ops for removed edges
            let edges_to_remove: Vec<EntityId> = self
                .infra_edges
                .iter()
                .filter(|(_, e)| e.source == entity || e.target == entity)
                .map(|(&id, _)| id)
                .collect();
            let mut result = CommandResult::ok_with_entity(entity);
            for eid in &edges_to_remove {
                if let Some(edge) = self.infra_edges.remove(eid) {
                    if let Some(fin) = self.financials.get_mut(&corp_id) {
                        fin.cost_per_tick = (fin.cost_per_tick - edge.maintenance_cost).max(0);
                    }
                    result.ops.push(DeltaOp::EdgeRemoved { entity_id: *eid });
                }
            }
            // Reduce maintenance
            if let Some(fin) = self.financials.get_mut(&corp_id) {
                fin.cost_per_tick = (fin.cost_per_tick - node.maintenance_cost).max(0);
            }
            // Remove from corp tracking
            if let Some(nodes) = self.corp_infra_nodes.get_mut(&corp_id) {
                nodes.retain(|&id| id != entity);
            }
            // Cleanup other components
            self.positions.remove(&entity);
            self.healths.remove(&entity);
            self.capacities.remove(&entity);
            self.ownerships.remove(&entity);
            self.constructions.remove(&entity);

            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::NodeDestroyed { entity },
            );

            // Recover 20% of construction cost
            let salvage = node.construction_cost / 5;
            if let Some(fin) = self.financials.get_mut(&corp_id) {
                fin.cash += salvage;
            }

            result.ops.push(DeltaOp::NodeRemoved { entity_id: entity });
            result
        } else {
            CommandResult::fail("Node not found")
        }
    }

    pub(super) fn cmd_repair_node(&mut self, entity: EntityId, emergency: bool) {
        let (corp_id, base_cost) = match self.infra_nodes.get(&entity) {
            Some(n) => (n.owner, n.construction_cost),
            None => return,
        };

        // Don't allow repair while already repairing
        if self
            .infra_nodes
            .get(&entity)
            .map(|n| n.repairing)
            .unwrap_or(false)
        {
            return;
        }

        let current_health = match self.healths.get(&entity) {
            Some(h) => h.condition,
            None => return,
        };

        if current_health >= 0.95 {
            return; // Already healthy
        }

        // Cost = (1.0 - health) x construction_cost x rate_multiplier
        // Standard: 0.3, Emergency: 0.8
        let damage = 1.0 - current_health;
        let rate_multiplier = if emergency { 0.8 } else { 0.3 };
        let cost = (base_cost as f64 * damage * rate_multiplier) as Money;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }

        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        // Duration: standard = 10 ticks, emergency = 2 ticks
        let duration = if emergency { 2u32 } else { 10u32 };
        let health_per_tick = damage / duration as f64;

        if let Some(node) = self.infra_nodes.get_mut(&entity) {
            node.repairing = true;
            node.repair_ticks_left = duration;
            node.repair_health_per_tick = health_per_tick;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::RepairStarted { entity, cost },
        );
    }

    pub(super) fn cmd_repair_edge(&mut self, edge_id: EntityId, emergency: bool) {
        use crate::components::infra_edge::DeploymentMethod;

        let (corp_id, base_cost, health, deployment, edge_type, already_repairing) =
            match self.infra_edges.get(&edge_id) {
                Some(e) => (
                    e.owner,
                    e.construction_cost,
                    e.health,
                    e.deployment,
                    e.edge_type,
                    e.repairing,
                ),
                None => return,
            };

        // Don't allow repair while already repairing
        if already_repairing {
            return;
        }

        if health >= 0.95 {
            return; // Already healthy
        }

        let damage = 1.0 - health;
        let rate_multiplier = if emergency { 0.8 } else { 0.3 };

        // Type-based cost multiplier: Aerial 0.7, Underground 1.5, Submarine 5.0
        let is_submarine = matches!(
            edge_type,
            gt_common::types::EdgeType::Submarine
                | gt_common::types::EdgeType::SubseaTelegraphCable
                | gt_common::types::EdgeType::SubseaFiberCable
        );
        let type_multiplier = if is_submarine {
            5.0
        } else {
            match deployment {
                DeploymentMethod::Aerial => 0.7,
                DeploymentMethod::Underground => 1.5,
            }
        };

        let cost = (base_cost as f64 * damage * rate_multiplier * type_multiplier) as Money;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }

        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        // Duration: standard = 10 ticks, emergency = 2 ticks
        let duration = if emergency { 2u32 } else { 10u32 };
        let health_per_tick = damage / duration as f64;

        if let Some(edge) = self.infra_edges.get_mut(&edge_id) {
            edge.repairing = true;
            edge.repair_ticks_left = duration;
            edge.repair_health_per_tick = health_per_tick;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::RepairStarted {
                entity: edge_id,
                cost,
            },
        );
    }

    pub(super) fn cmd_create_subsidiary(&mut self, parent: EntityId, name: &str) {
        // Parent must be a corporation
        if !self.corporations.contains_key(&parent) {
            return;
        }

        // Cost to establish a subsidiary
        let establishment_cost: Money = 1_000_000;
        if let Some(fin) = self.financials.get(&parent) {
            if fin.cash < establishment_cost {
                return;
            }
        } else {
            return;
        }

        // Deduct cost
        if let Some(fin) = self.financials.get_mut(&parent) {
            fin.cash -= establishment_cost;
        }

        // Create subsidiary entity
        let sub_id = self.allocate_entity();
        let mut sub_corp = Corporation::new(name, false);
        sub_corp.credit_rating = self
            .corporations
            .get(&parent)
            .map(|c| c.credit_rating)
            .unwrap_or(CreditRating::BBB);

        self.corporations.insert(sub_id, sub_corp);
        self.financials.insert(
            sub_id,
            Financial {
                cash: 500_000, // Seed capital
                revenue_per_tick: 0,
                cost_per_tick: 0,
                debt: 0,
            },
        );
        self.workforces.insert(sub_id, Workforce::default());
        self.policies.insert(sub_id, Policy::default());

        // Register as subsidiary of parent
        if let Some(parent_corp) = self.corporations.get_mut(&parent) {
            parent_corp.subsidiaries.push(sub_id);
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SubsidiaryCreated {
                parent,
                subsidiary: sub_id,
                name: name.to_string(),
            },
        );
    }

    pub(super) fn cmd_purchase_insurance(&mut self, node: EntityId) {
        let (owner, premium) = match self.infra_nodes.get(&node) {
            Some(n) => {
                if n.insured {
                    return; // Already insured
                }
                (n.owner, n.insurance_premium)
            }
            None => return,
        };

        // Check funds for premium
        if let Some(fin) = self.financials.get(&owner) {
            if fin.cash < premium {
                return;
            }
        } else {
            return;
        }

        // Deduct first premium payment and mark as insured
        if let Some(fin) = self.financials.get_mut(&owner) {
            fin.cash -= premium;
            fin.cost_per_tick += premium; // Ongoing premium
        }
        if let Some(n) = self.infra_nodes.get_mut(&node) {
            n.insured = true;
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::InsurancePurchased {
                entity: node,
                premium,
            },
        );
    }

    /// Purchase a cable ship for submarine cable construction.
    /// Cost: $50,000,000. Required for building SubmarineCable edges.
    pub(super) fn cmd_purchase_cable_ship(&mut self) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let ship_cost: Money = 50_000_000;

        // Check funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < ship_cost {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "Insufficient funds for cable ship. Need ${}, have ${}",
                            ship_cost, fin.cash
                        ),
                        level: "warning".to_string(),
                    },
                );
                return;
            }
        } else {
            return;
        }

        // Deduct cost
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= ship_cost;
        }

        // Increment ship count
        let count = self.cable_ships.entry(corp_id).or_insert(0);
        *count += 1;

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::CableShipPurchased { corp: corp_id },
        );

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::GlobalNotification {
                message: format!("Cable ship purchased! You now own {} ship(s).", count),
                level: "info".to_string(),
            },
        );
    }
}
