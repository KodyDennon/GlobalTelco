use gt_common::types::*;

use crate::components::*;

use super::GameWorld;

impl GameWorld {
    /// Serialize the entire game world to a JSON string for saving.
    pub fn save_game(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Save failed: {}", e))
    }

    /// Deserialize a game world from a JSON string.
    pub fn load_game(data: &str) -> Result<Self, String> {
        let mut world: Self = serde_json::from_str(data).map_err(|e| format!("Load failed: {}", e))?;
        // dirty_flags is #[serde(skip)] so it defaults to 0 after deser.
        // Set all bits so every conditional system runs on the first tick after load.
        world.dirty_flags = u64::MAX;
        Ok(world)
    }

    /// Serialize to binary format (bincode + zstd compression).
    /// Format: [version: u8] [crc32: 4 bytes LE] [zstd-compressed bincode data]
    #[cfg(feature = "native-compression")]
    pub fn save_game_binary(&self) -> Result<Vec<u8>, String> {
        let bincode_data =
            bincode::serialize(self).map_err(|e| format!("Bincode serialize failed: {}", e))?;
        let compressed = zstd::encode_all(bincode_data.as_slice(), 3)
            .map_err(|e| format!("Zstd compress failed: {}", e))?;
        let checksum = crc32fast::hash(&compressed);
        let mut result = Vec::with_capacity(1 + 4 + compressed.len());
        result.push(2u8); // version byte (v2 = with CRC32)
        result.extend_from_slice(&checksum.to_le_bytes());
        result.extend_from_slice(&compressed);
        Ok(result)
    }

    /// Deserialize from binary format (bincode + zstd).
    /// Supports v1 (no checksum) and v2 (with CRC32 checksum).
    #[cfg(feature = "native-compression")]
    pub fn load_game_binary(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("Empty save data".to_string());
        }
        let version = data[0];
        let payload = match version {
            1 => {
                // Legacy v1: no checksum, payload starts at byte 1
                &data[1..]
            }
            2 => {
                // v2: [version: 1] [crc32: 4] [payload: rest]
                if data.len() < 6 {
                    return Err("Save data too short for v2 format".to_string());
                }
                let stored_crc = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                let payload = &data[5..];
                let computed_crc = crc32fast::hash(payload);
                if stored_crc != computed_crc {
                    return Err(format!(
                        "Save file corrupted: CRC32 mismatch (expected {:08x}, got {:08x})",
                        stored_crc, computed_crc
                    ));
                }
                payload
            }
            _ => return Err(format!("Unsupported save version: {}", version)),
        };
        let decompressed =
            zstd::decode_all(payload).map_err(|e| format!("Zstd decompress failed: {}", e))?;
        let mut world: Self = bincode::deserialize(&decompressed)
            .map_err(|e| format!("Bincode deserialize failed: {}", e))?;
        // dirty_flags is #[serde(skip)] so defaults to 0 after deser.
        // Set all bits so every conditional system runs on the first tick after load.
        world.dirty_flags = u64::MAX;
        Ok(world)
    }

    /// Apply a batch of delta operations to the world state.
    /// Used by multiplayer clients to incrementally update WASM state
    /// from CommandBroadcast messages without a full snapshot reload.
    pub fn apply_delta(&mut self, ops: &[gt_common::protocol::DeltaOp]) {
        use gt_common::protocol::DeltaOp;
        for op in ops {
            match op {
                DeltaOp::NodeCreated {
                    entity_id,
                    owner,
                    node_type,
                    network_level: _,
                    lon,
                    lat,
                    under_construction,
                } => {
                    // Find nearest cell for terrain lookup
                    let (cell_index, _) = self.find_nearest_cell(*lon, *lat).unwrap_or((0, 0.0));
                    let terrain = self
                        .get_cell_terrain(cell_index)
                        .unwrap_or(TerrainType::Rural);
                    let node = InfraNode::new_on_terrain(*node_type, cell_index, *owner, terrain);
                    self.infra_nodes.insert(*entity_id, node);
                    let region_id = self.cell_to_region.get(&cell_index).copied();
                    self.positions.insert(
                        *entity_id,
                        Position {
                            x: *lon,
                            y: *lat,
                            region_id,
                        },
                    );
                    self.spatial_index.insert(crate::world::SpatialNode {
                        id: *entity_id,
                        pos: [*lon, *lat],
                    });
                    self.ownerships.insert(*entity_id, Ownership::sole(*owner));
                    self.healths.insert(*entity_id, Health::new());
                    self.capacities.insert(*entity_id, Capacity::new(0.0));
                    if *under_construction {
                        self.constructions
                            .insert(*entity_id, Construction::new(self.tick, 10));
                    }
                    self.corp_infra_nodes
                        .entry(*owner)
                        .or_default()
                        .push(*entity_id);
                    // Ensure next_entity_id stays ahead
                    if *entity_id >= self.next_entity_id {
                        self.next_entity_id = *entity_id + 1;
                    }
                }
                DeltaOp::EdgeCreated {
                    entity_id,
                    owner,
                    edge_type,
                    from_node,
                    to_node,
                } => {
                    let from_pos = self.positions.get(from_node);
                    let to_pos = self.positions.get(to_node);
                    let length_km = match (from_pos, to_pos) {
                        (Some(a), Some(b)) => {
                            let dlat = (a.y - b.y).to_radians();
                            let dlon = (a.x - b.x).to_radians();
                            let lat1 = a.y.to_radians();
                            let lat2 = b.y.to_radians();
                            let a_val = (dlat / 2.0).sin().powi(2)
                                + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
                            6371.0 * 2.0 * a_val.sqrt().asin()
                        }
                        _ => 100.0,
                    };
                    let edge = InfraEdge::new(*edge_type, *from_node, *to_node, length_km, *owner);
                    self.infra_edges.insert(*entity_id, edge);
                    self.network
                        .add_edge_with_id(*from_node, *to_node, *entity_id);
                    if *entity_id >= self.next_entity_id {
                        self.next_entity_id = *entity_id + 1;
                    }
                }
                DeltaOp::NodeUpgraded { entity_id, .. } => {
                    if let Some(node) = self.infra_nodes.get_mut(entity_id) {
                        node.max_throughput *= 1.5;
                        node.reliability = (node.reliability + 0.05).min(1.0);
                    }
                    if let Some(cap) = self.capacities.get_mut(entity_id) {
                        cap.max_throughput *= 1.5;
                    }
                }
                DeltaOp::NodeRemoved { entity_id } => {
                    if let Some(pos) = self.positions.get(entity_id) {
                        self.spatial_index.remove(&crate::world::SpatialNode {
                            id: *entity_id,
                            pos: [pos.x, pos.y],
                        });
                    }
                    if let Some(node) = self.infra_nodes.remove(entity_id) {
                        self.network.remove_node(*entity_id);
                        if let Some(nodes) = self.corp_infra_nodes.get_mut(&node.owner) {
                            nodes.retain(|&id| id != *entity_id);
                        }
                        self.positions.remove(entity_id);
                        self.healths.remove(entity_id);
                        self.capacities.remove(entity_id);
                        self.ownerships.remove(entity_id);
                        self.constructions.remove(entity_id);
                    }
                }
                DeltaOp::EdgeRemoved { entity_id } => {
                    self.infra_edges.remove(entity_id);
                }
                DeltaOp::ConstructionCompleted { entity_id } => {
                    self.constructions.remove(entity_id);
                }
                DeltaOp::SatelliteLaunched {
                    entity_id,
                    owner: _,
                    orbit_type: _,
                    lon: _,
                    lat: _,
                    altitude_km: _,
                } => {
                    if let Some(sat) = self.satellites.get_mut(entity_id) {
                        sat.status = gt_common::types::SatelliteStatus::Operational;
                        sat.launched_tick = self.tick;
                    }
                }
                DeltaOp::SatelliteRemoved { entity_id } => {
                    if let Some(sat) = self.satellites.get_mut(entity_id) {
                        sat.status = gt_common::types::SatelliteStatus::Dead;
                    }
                }
            }
        }
    }
}
