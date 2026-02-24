use std::collections::HashMap;

use gt_common::commands::Command;
use gt_common::types::*;
use serde::{Deserialize, Serialize};

use crate::components::covert_ops::MissionType;
use crate::components::*;
use crate::events::EventQueue;
use crate::systems;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameWorld {
    config: WorldConfig,
    tick: Tick,
    #[serde(default)]
    speed: GameSpeed,
    next_entity_id: EntityId,
    player_corp_id: Option<EntityId>,

    // Original component storage
    pub positions: HashMap<EntityId, Position>,
    pub ownerships: HashMap<EntityId, Ownership>,
    pub financials: HashMap<EntityId, Financial>,
    pub capacities: HashMap<EntityId, Capacity>,
    pub healths: HashMap<EntityId, Health>,
    pub constructions: HashMap<EntityId, Construction>,
    pub populations: HashMap<EntityId, Population>,
    pub demands: HashMap<EntityId, Demand>,
    pub workforces: HashMap<EntityId, Workforce>,
    pub ai_states: HashMap<EntityId, AiState>,
    pub policies: HashMap<EntityId, Policy>,
    pub corporations: HashMap<EntityId, Corporation>,

    // World generation component storage
    pub land_parcels: HashMap<EntityId, LandParcelComponent>,
    pub regions: HashMap<EntityId, RegionComponent>,
    pub cities: HashMap<EntityId, CityComponent>,

    // Infrastructure component storage
    pub infra_nodes: HashMap<EntityId, InfraNode>,
    pub infra_edges: HashMap<EntityId, InfraEdge>,
    pub contracts: HashMap<EntityId, Contract>,
    pub debt_instruments: HashMap<EntityId, DebtInstrument>,
    pub tech_research: HashMap<EntityId, TechResearch>,

    // Advanced gameplay (Phase 10)
    pub auctions: HashMap<EntityId, Auction>,
    pub acquisition_proposals: HashMap<EntityId, AcquisitionProposal>,
    pub covert_ops: HashMap<EntityId, CovertOps>,
    pub lobbying_campaigns: HashMap<EntityId, LobbyingCampaign>,
    pub achievements: HashMap<EntityId, AchievementTracker>,
    pub victory_state: Option<VictoryConditions>,

    // Intel levels: (spy_corp, target_corp) → intel level (0..3)
    // 0 = infra positions only (default), 1 = basic financials (ranges),
    // 2 = detailed financials (exact), 3 = operational data (utilization, health, throughput)
    #[serde(with = "gt_common::serde_helpers::entity_pair_map")]
    pub intel_levels: HashMap<(EntityId, EntityId), u8>,

    // Mappings for fast lookup
    pub cell_to_parcel: HashMap<usize, EntityId>,
    pub cell_to_region: HashMap<usize, EntityId>,
    pub cell_to_city: HashMap<usize, EntityId>,
    pub corp_infra_nodes: HashMap<EntityId, Vec<EntityId>>,

    // World grid reference (used by systems for spatial queries)
    pub grid_cell_count: usize,
    pub grid_cell_positions: Vec<(f64, f64)>, // (lat, lon) for each cell
    pub grid_cell_neighbors: Vec<Vec<usize>>, // adjacency list per cell
    /// Average distance between neighboring grid cells in km.
    /// Used by coverage system to scale coverage radii to the grid resolution.
    pub cell_spacing_km: f64,

    // Per-cell coverage data (recalculated each tick by coverage system)
    #[serde(skip)]
    pub cell_coverage: HashMap<usize, crate::systems::coverage::CellCoverage>,

    // Traffic flow data (computed by utilization system)
    pub traffic_matrix: gt_common::types::TrafficMatrix,

    // Network graph state
    pub network: gt_infrastructure::NetworkGraph,

    // RNG for deterministic simulation
    rng_seed: u64,
    tick_rng_counter: u64,

    pub event_queue: EventQueue,
}

impl GameWorld {
    pub fn new(config: WorldConfig) -> Self {
        let rng_seed = config.seed;

        let mut world = Self {
            config: config.clone(),
            tick: 0,
            speed: GameSpeed::Normal,
            next_entity_id: 1,
            player_corp_id: None,
            positions: HashMap::new(),
            ownerships: HashMap::new(),
            financials: HashMap::new(),
            capacities: HashMap::new(),
            healths: HashMap::new(),
            constructions: HashMap::new(),
            populations: HashMap::new(),
            demands: HashMap::new(),
            workforces: HashMap::new(),
            ai_states: HashMap::new(),
            policies: HashMap::new(),
            corporations: HashMap::new(),
            land_parcels: HashMap::new(),
            regions: HashMap::new(),
            cities: HashMap::new(),
            infra_nodes: HashMap::new(),
            infra_edges: HashMap::new(),
            contracts: HashMap::new(),
            debt_instruments: HashMap::new(),
            tech_research: HashMap::new(),
            auctions: HashMap::new(),
            acquisition_proposals: HashMap::new(),
            covert_ops: HashMap::new(),
            lobbying_campaigns: HashMap::new(),
            achievements: HashMap::new(),
            victory_state: None,
            intel_levels: HashMap::new(),
            cell_to_parcel: HashMap::new(),
            cell_to_region: HashMap::new(),
            cell_to_city: HashMap::new(),
            corp_infra_nodes: HashMap::new(),
            grid_cell_count: 0,
            grid_cell_positions: Vec::new(),
            grid_cell_neighbors: Vec::new(),
            cell_spacing_km: 100.0,
            cell_coverage: HashMap::new(),
            traffic_matrix: gt_common::types::TrafficMatrix::default(),
            network: gt_infrastructure::NetworkGraph::new(),
            rng_seed,
            tick_rng_counter: 0,
            event_queue: EventQueue::new(),
        };

        world.generate_world();
        world.create_corporations();
        world.seed_tech_tree();

        world
    }

    fn generate_world(&mut self) {
        let generator = gt_world::WorldGenerator::new(self.config.clone());
        let gen_world = generator.generate();

        // Store grid info for spatial queries
        self.grid_cell_count = gen_world.grid.cell_count();
        self.grid_cell_positions = gen_world
            .grid
            .cells
            .iter()
            .map(|c| (c.lat, c.lon))
            .collect();
        self.grid_cell_neighbors = gen_world
            .grid
            .cells
            .iter()
            .map(|c| c.neighbors.clone())
            .collect();

        // Calculate average cell spacing in km for coverage radius scaling
        // On a sphere: avg_spacing ≈ sqrt(4πR² / N) where R = 6371 km
        if self.grid_cell_count > 0 {
            let surface_area = 4.0 * std::f64::consts::PI * 6371.0_f64.powi(2);
            let area_per_cell = surface_area / self.grid_cell_count as f64;
            self.cell_spacing_km = area_per_cell.sqrt();
        }

        // Create land parcel entities
        for parcel in &gen_world.parcels {
            let id = self.allocate_entity();
            let zoning = match parcel.zoning {
                gt_world::parcels::ZoningType::Residential => ZoningType::Residential,
                gt_world::parcels::ZoningType::Commercial => ZoningType::Commercial,
                gt_world::parcels::ZoningType::Industrial => ZoningType::Industrial,
                gt_world::parcels::ZoningType::Mixed => ZoningType::Mixed,
                gt_world::parcels::ZoningType::Protected => ZoningType::Protected,
                gt_world::parcels::ZoningType::Unzoned => ZoningType::Unzoned,
            };
            self.land_parcels.insert(
                id,
                LandParcelComponent {
                    cell_index: parcel.cell_index,
                    terrain: parcel.terrain,
                    elevation: parcel.elevation,
                    zoning,
                    cost_modifier: parcel.cost_modifier,
                    disaster_risk: parcel.disaster_risk,
                    owner: None,
                },
            );
            self.cell_to_parcel.insert(parcel.cell_index, id);
            // Position from lat/lon
            if let Some(&(lat, lon)) = self.grid_cell_positions.get(parcel.cell_index) {
                self.positions.insert(id, Position::new(lon, lat));
            }
        }

        // Create region entities - map old temp IDs to new real IDs
        let mut region_id_map: HashMap<EntityId, EntityId> = HashMap::new();
        for region in &gen_world.regions {
            let id = self.allocate_entity();
            region_id_map.insert(region.id, id);

            self.regions.insert(
                id,
                RegionComponent {
                    name: region.name.clone(),
                    cells: region.cells.clone(),
                    center_lat: region.center_lat,
                    center_lon: region.center_lon,
                    gdp: region.gdp,
                    population: region.population,
                    development: region.development,
                    regulatory_strictness: 0.5,
                    tax_rate: 0.2,
                    disaster_risk: 0.1,
                    city_ids: Vec::new(),
                    boundary_polygon: region.boundary_polygon.clone(),
                },
            );
            self.positions.insert(
                id,
                Position {
                    x: region.center_lon,
                    y: region.center_lat,
                    region_id: Some(id),
                },
            );
            self.populations
                .insert(id, Population::new(region.population));
            self.demands.insert(
                id,
                Demand {
                    base_demand: (region.population as f64).sqrt() * region.development * 2.0,
                    current_demand: 0.0,
                    satisfaction: 0.0,
                },
            );

            // Map cells to this region
            for &cell_idx in &region.cells {
                self.cell_to_region.insert(cell_idx, id);
            }
        }

        // Create city entities with multi-cell expansion
        // First pass: collect region cells for neighbor lookup
        let region_cell_sets: HashMap<EntityId, Vec<usize>> = self
            .regions
            .iter()
            .map(|(&id, r)| (id, r.cells.clone()))
            .collect();

        // Build neighbor lookup from grid (we stored positions, reconstruct neighbors)
        // Use a simple spatial approach: cells within ~1.5x average spacing are neighbors
        let avg_spacing_deg = if self.grid_cell_count > 0 {
            360.0 / (self.grid_cell_count as f64).sqrt()
        } else {
            5.0
        };
        let neighbor_threshold = avg_spacing_deg * 1.8; // ~1.8x spacing catches all neighbors

        for city in &gen_world.cities {
            let id = self.allocate_entity();
            let real_region_id = region_id_map.get(&city.region_id).copied().unwrap_or(0);

            let _econ = gen_world.economics.cities.iter().find(|e| {
                gen_world.cities.get(e.city_index).map(|c| c.cell_index) == Some(city.cell_index)
            });
            // Use sqrt(population) scaling for demand — matches the formula in the demand system tick.
            // Keeps demand in the same magnitude as node capacity (hundreds to thousands).
            let telecom_demand = (city.population as f64).sqrt() * city.development * 2.0;

            // Expand city to multiple cells based on population
            // Small cities: 1-3 cells, medium: 3-7, large: 7-15
            let cell_budget = if city.population > 500_000 {
                ((city.population as f64).log10() * 3.0) as usize
            } else if city.population > 100_000 {
                ((city.population as f64).log10() * 2.0) as usize
            } else {
                1
            };
            let cell_budget = cell_budget.max(1).min(15);

            let center_pos = self.grid_cell_positions.get(city.cell_index);
            let region_cells = region_cell_sets.get(&real_region_id);

            let mut city_cells = vec![city.cell_index];

            if cell_budget > 1 {
                if let (Some(&(clat, clon)), Some(rcells)) = (center_pos, region_cells) {
                    // Find nearby cells in the same region, sorted by distance to center
                    let mut candidates: Vec<(usize, f64)> = rcells
                        .iter()
                        .filter(|&&ci| {
                            ci != city.cell_index && !self.cell_to_city.contains_key(&ci)
                        })
                        .filter_map(|&ci| {
                            let (lat, lon) = self.grid_cell_positions.get(ci)?;
                            let dlat = lat - clat;
                            let dlon = lon - clon;
                            let dist = (dlat * dlat + dlon * dlon).sqrt();
                            if dist < neighbor_threshold * (cell_budget as f64).sqrt() {
                                Some((ci, dist))
                            } else {
                                None
                            }
                        })
                        .collect();
                    candidates
                        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                    for (ci, _) in candidates.into_iter().take(cell_budget - 1) {
                        city_cells.push(ci);
                    }
                }
            }

            // Map all city cells
            for &ci in &city_cells {
                self.cell_to_city.insert(ci, id);
            }

            self.cities.insert(
                id,
                CityComponent {
                    name: city.name.clone(),
                    region_id: real_region_id,
                    cell_index: city.cell_index,
                    cells: city_cells,
                    population: city.population,
                    growth_rate: city.growth_rate,
                    development: city.development,
                    telecom_demand,
                    infrastructure_satisfaction: 0.0,
                    jobs_available: 0,
                    employment_rate: 0.5 + city.development * 0.3,
                    birth_rate: 0.012 + city.development * 0.003,
                    death_rate: 0.008,
                    migration_pressure: 0.0,
                },
            );
            self.positions.insert(
                id,
                Position {
                    x: self
                        .grid_cell_positions
                        .get(city.cell_index)
                        .map(|p| p.1)
                        .unwrap_or(0.0),
                    y: self
                        .grid_cell_positions
                        .get(city.cell_index)
                        .map(|p| p.0)
                        .unwrap_or(0.0),
                    region_id: Some(real_region_id),
                },
            );
            self.populations.insert(
                id,
                Population {
                    count: city.population,
                    growth_rate: city.growth_rate,
                    income_level: city.development,
                },
            );
            self.demands.insert(
                id,
                Demand {
                    base_demand: telecom_demand,
                    current_demand: telecom_demand,
                    satisfaction: 0.0,
                },
            );

            // Add city to its region's city list
            if let Some(region) = self.regions.get_mut(&real_region_id) {
                region.city_ids.push(id);
            }
        }
    }

    fn create_corporations(&mut self) {
        let era_config = gt_common::config::EraConfig::from_era(self.config.starting_era);
        let difficulty = gt_common::config::DifficultyConfig::from_preset(self.config.difficulty);
        let starting_capital =
            (era_config.starting_capital as f64 * difficulty.starting_capital_multiplier) as Money;

        // Create player corporation
        let player_id = self.allocate_entity();
        self.player_corp_id = Some(player_id);
        let player_name = self
            .config
            .corp_name
            .clone()
            .unwrap_or_else(|| "Player Corp".to_string());
        self.corporations
            .insert(player_id, Corporation::new(&player_name, true));
        self.financials.insert(
            player_id,
            Financial {
                cash: starting_capital,
                revenue_per_tick: 0,
                cost_per_tick: 0,
                debt: 0,
            },
        );
        self.policies.insert(player_id, Policy::default());
        self.workforces.insert(
            player_id,
            Workforce {
                employee_count: 10,
                skill_level: 0.5,
                morale: 0.8,
                salary_per_tick: 1000,
            },
        );

        // Create AI corporations
        let archetypes = [
            AIArchetype::AggressiveExpander,
            AIArchetype::DefensiveConsolidator,
            AIArchetype::TechInnovator,
            AIArchetype::BudgetOperator,
        ];
        let ai_names = [
            "TelcoMax Global",
            "NetGuard Systems",
            "FutureCom Labs",
            "ValueNet Inc",
            "MegaLink Corp",
            "SecureNet Holdings",
            "InnoWave Tech",
            "BudgetTel Solutions",
        ];

        for i in 0..self.config.ai_corporations as usize {
            let ai_id = self.allocate_entity();
            let archetype = archetypes[i % archetypes.len()];
            let name = ai_names.get(i).unwrap_or(&"AI Corp");
            let ai_capital =
                (starting_capital as f64 * (0.5 + difficulty.ai_aggressiveness * 0.5)) as Money;

            self.corporations
                .insert(ai_id, Corporation::new(*name, false));
            self.financials.insert(
                ai_id,
                Financial {
                    cash: ai_capital,
                    revenue_per_tick: 0,
                    cost_per_tick: 0,
                    debt: 0,
                },
            );
            self.ai_states.insert(ai_id, AiState::new(archetype));
            self.policies.insert(ai_id, Policy::default());
            self.workforces.insert(
                ai_id,
                Workforce {
                    employee_count: 15,
                    skill_level: 0.5 + archetype_skill_bonus(archetype),
                    morale: 0.7,
                    salary_per_tick: 1200,
                },
            );

            // Give AI some starting infrastructure in random regions
            self.seed_ai_infrastructure(ai_id, i);
        }
    }

    fn seed_ai_infrastructure(&mut self, corp_id: EntityId, corp_index: usize) {
        // Pick a region for this AI to start in (distribute across regions)
        let mut region_ids: Vec<EntityId> = self.regions.keys().copied().collect();
        region_ids.sort_unstable();
        if region_ids.is_empty() {
            return;
        }

        let start_region_id = region_ids[corp_index % region_ids.len()];
        let region = match self.regions.get(&start_region_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let mut placed_node_ids: Vec<(EntityId, usize)> = Vec::new(); // (node_id, cell_index)

        // Place a central office in the first city of their region
        if let Some(&city_id) = region.city_ids.first() {
            if let Some(city) = self.cities.get(&city_id) {
                // Find a suitable neighbor: must be land, and ideally no city
                let city_center_cell = city.cell_index;
                let cell_index = self
                    .grid_cell_neighbors
                    .get(city_center_cell)
                    .and_then(|neighbors| {
                        neighbors
                            .iter()
                            .find(|&&n_idx| {
                                let is_not_city = !self.cell_to_city.contains_key(&n_idx);
                                let is_land = self
                                    .cell_to_parcel
                                    .get(&n_idx)
                                    .and_then(|p_id| self.land_parcels.get(p_id))
                                    .map(|p| {
                                        !matches!(
                                            p.terrain,
                                            TerrainType::OceanDeep | TerrainType::OceanShallow
                                        )
                                    })
                                    .unwrap_or(false);

                                is_not_city && is_land
                            })
                            .or_else(|| neighbors.first())
                    })
                    .copied()
                    .unwrap_or(city_center_cell);

                let node_id = self.allocate_entity();
                let node = InfraNode::new(NodeType::CentralOffice, cell_index, corp_id);
                let maintenance = node.maintenance_cost;

                self.infra_nodes.insert(node_id, node);
                self.healths.insert(node_id, Health::new());
                self.capacities.insert(node_id, Capacity::new(1000.0));
                self.ownerships.insert(node_id, Ownership::sole(corp_id));

                if let Some(&(cell_lat, cell_lon)) = self.grid_cell_positions.get(cell_index) {
                    let jitter_range = (self.cell_spacing_km / 111.0) * 0.2;
                    let r1 = self.deterministic_random();
                    let r2 = self.deterministic_random();
                    self.positions.insert(
                        node_id,
                        Position {
                            x: cell_lon + (r1 * 2.0 - 1.0) * jitter_range,
                            y: cell_lat + (r2 * 2.0 - 1.0) * jitter_range,
                            region_id: Some(start_region_id),
                        },
                    );
                }
                self.network.add_node(node_id);
                self.corp_infra_nodes
                    .entry(corp_id)
                    .or_default()
                    .push(node_id);

                if let Some(fin) = self.financials.get_mut(&corp_id) {
                    fin.cost_per_tick += maintenance;
                }

                placed_node_ids.push((node_id, cell_index));
            }
        }

        // Place cell towers in additional cities (up to 2 more) to create a small network
        for &city_id in region.city_ids.iter().skip(1).take(2) {
            if let Some(city) = self.cities.get(&city_id) {
                // Find a suitable neighbor for the cell tower
                let city_center_cell = city.cell_index;
                let cell_index = self
                    .grid_cell_neighbors
                    .get(city_center_cell)
                    .and_then(|neighbors| {
                        neighbors
                            .iter()
                            .find(|&&n_idx| {
                                let is_not_city = !self.cell_to_city.contains_key(&n_idx);
                                let is_land = self
                                    .cell_to_parcel
                                    .get(&n_idx)
                                    .and_then(|p_id| self.land_parcels.get(p_id))
                                    .map(|p| {
                                        !matches!(
                                            p.terrain,
                                            TerrainType::OceanDeep | TerrainType::OceanShallow
                                        )
                                    })
                                    .unwrap_or(false);

                                is_not_city && is_land
                            })
                            .or_else(|| neighbors.get(1).or_else(|| neighbors.first()))
                    })
                    .copied()
                    .unwrap_or(city_center_cell);

                let node_id = self.allocate_entity();
                let node = InfraNode::new(NodeType::CellTower, cell_index, corp_id);
                let maintenance = node.maintenance_cost;

                self.infra_nodes.insert(node_id, node);
                self.healths.insert(node_id, Health::new());
                self.capacities.insert(node_id, Capacity::new(500.0));
                self.ownerships.insert(node_id, Ownership::sole(corp_id));

                if let Some(&(cell_lat, cell_lon)) = self.grid_cell_positions.get(cell_index) {
                    let jitter_range = (self.cell_spacing_km / 111.0) * 0.2;
                    let r1 = self.deterministic_random();
                    let r2 = self.deterministic_random();
                    self.positions.insert(
                        node_id,
                        Position {
                            x: cell_lon + (r1 * 2.0 - 1.0) * jitter_range,
                            y: cell_lat + (r2 * 2.0 - 1.0) * jitter_range,
                            region_id: Some(start_region_id),
                        },
                    );
                }
                self.network.add_node(node_id);
                self.corp_infra_nodes
                    .entry(corp_id)
                    .or_default()
                    .push(node_id);

                if let Some(fin) = self.financials.get_mut(&corp_id) {
                    fin.cost_per_tick += maintenance;
                }

                placed_node_ids.push((node_id, cell_index));
            }
        }

        // Connect all placed nodes with fiber optic edges so bandwidth/latency flows exist from tick 0
        if placed_node_ids.len() > 1 {
            let hub_id = placed_node_ids[0].0;

            for &(spoke_id, _spoke_cell) in &placed_node_ids[1..] {
                // Calculate distance using actual node positions (haversine)
                let length_km = match (
                    self.positions.get(&hub_id),
                    self.positions.get(&spoke_id),
                ) {
                    (Some(p1), Some(p2)) => {
                        let dlat = (p1.y - p2.y).to_radians();
                        let dlon = (p1.x - p2.x).to_radians();
                        let a = (dlat / 2.0).sin().powi(2)
                            + p1.y.to_radians().cos()
                                * p2.y.to_radians().cos()
                                * (dlon / 2.0).sin().powi(2);
                        let c = 2.0 * a.sqrt().asin();
                        6371.0 * c
                    }
                    _ => 50.0,
                };

                let edge =
                    InfraEdge::new(EdgeType::FiberLocal, hub_id, spoke_id, length_km, corp_id);
                let edge_maintenance = edge.maintenance_cost;
                let edge_id = self.allocate_entity();
                self.infra_edges.insert(edge_id, edge);
                self.network.add_edge_with_id(hub_id, spoke_id, edge_id);

                if let Some(fin) = self.financials.get_mut(&corp_id) {
                    fin.cost_per_tick += edge_maintenance;
                }
            }
        }
    }

    fn seed_tech_tree(&mut self) {
        let techs = crate::components::tech_research::generate_tech_tree();
        for tech in techs {
            let id = self.allocate_entity();
            self.tech_research.insert(id, tech);
        }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    /// Get the intel level that `spy_corp` has against `target_corp`.
    /// Returns 0 (no intel) by default.
    pub fn get_intel_level(&self, spy_corp: EntityId, target_corp: EntityId) -> u8 {
        self.intel_levels
            .get(&(spy_corp, target_corp))
            .copied()
            .unwrap_or(0)
    }

    /// Get a snapshot of all intel levels for a specific spy corp.
    /// Returns a map of target_corp -> intel_level for all targets where level > 0.
    pub fn get_intel_levels_for_corp(&self, spy_corp: EntityId) -> HashMap<EntityId, u8> {
        self.intel_levels
            .iter()
            .filter(|&(&(spy, _), _)| spy == spy_corp)
            .map(|(&(_, target), &level)| (target, level))
            .collect()
    }

    pub fn speed(&self) -> GameSpeed {
        self.speed
    }

    pub fn player_corp_id(&self) -> Option<EntityId> {
        self.player_corp_id
    }

    pub fn entity_count(&self) -> usize {
        (self.next_entity_id - 1) as usize
    }

    /// Serialize the entire game world to a JSON string for saving.
    pub fn save_game(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Save failed: {}", e))
    }

    /// Deserialize a game world from a JSON string.
    pub fn load_game(data: &str) -> Result<Self, String> {
        serde_json::from_str(data).map_err(|e| format!("Load failed: {}", e))
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
        bincode::deserialize(&decompressed)
            .map_err(|e| format!("Bincode deserialize failed: {}", e))
    }

    pub fn allocate_entity(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn deterministic_random(&mut self) -> f64 {
        // Simple LCG for deterministic per-tick randomness
        self.tick_rng_counter = self
            .tick_rng_counter
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.rng_seed);
        (self.tick_rng_counter >> 33) as f64 / (1u64 << 31) as f64
    }

    pub fn tick(&mut self) {
        if self.speed == GameSpeed::Paused {
            return;
        }
        self.tick += 1;
        self.tick_rng_counter = self.tick.wrapping_mul(self.rng_seed.wrapping_add(1));
        systems::run_all_systems(self);
    }

    pub fn process_command(&mut self, command: Command) {
        match command {
            Command::SetSpeed(speed) => {
                self.speed = speed;
            }
            Command::TogglePause => {
                self.speed = if self.speed == GameSpeed::Paused {
                    GameSpeed::Normal
                } else {
                    GameSpeed::Paused
                };
            }
            Command::BuildNode { node_type, lon, lat } => {
                self.cmd_build_node(node_type, lon, lat);
            }
            Command::BuildEdge {
                edge_type,
                from,
                to,
            } => {
                self.cmd_build_edge(edge_type, from, to);
            }
            Command::UpgradeNode { entity } => {
                self.cmd_upgrade_node(entity);
            }
            Command::DecommissionNode { entity } => {
                self.cmd_decommission_node(entity);
            }
            Command::TakeLoan {
                corporation,
                amount,
            } => {
                self.cmd_take_loan(corporation, amount);
            }
            Command::RepayLoan { loan, amount } => {
                self.cmd_repay_loan(loan, amount);
            }
            Command::SetBudget {
                corporation,
                category,
                amount,
            } => {
                if let Some(policy) = self.policies.get_mut(&corporation) {
                    policy.set(format!("budget_{}", category), amount.to_string());
                }
            }
            Command::ProposeContract { from, to, terms } => {
                self.cmd_propose_contract(from, to, &terms);
            }
            Command::AcceptContract { contract } => {
                if let Some(c) = self.contracts.get_mut(&contract) {
                    c.activate(self.tick);
                    let event = gt_common::events::GameEvent::ContractAccepted { entity: contract };
                    self.event_queue.push(self.tick, event);
                }
            }
            Command::RejectContract { contract } => {
                self.contracts.remove(&contract);
            }
            Command::StartResearch { corporation, tech } => {
                self.cmd_start_research(corporation, &tech);
            }
            Command::CancelResearch { corporation } => {
                // Find active research for this corp and remove it
                let to_remove: Vec<EntityId> = self
                    .tech_research
                    .iter()
                    .filter(|(_, r)| r.researcher == Some(corporation) && !r.completed)
                    .map(|(&id, _)| id)
                    .collect();
                for id in to_remove {
                    self.tech_research.remove(&id);
                }
            }
            Command::HireEmployee { corporation, .. } => {
                if let Some(wf) = self.workforces.get_mut(&corporation) {
                    // Per-employee cost = total salary budget / current headcount
                    let per_employee = if wf.employee_count > 0 {
                        wf.salary_per_tick / wf.employee_count as Money
                    } else {
                        wf.salary_per_tick
                    };
                    wf.employee_count += 1;
                    if let Some(fin) = self.financials.get_mut(&corporation) {
                        fin.cost_per_tick += per_employee;
                    }
                    // Scale total salary budget to match new headcount
                    wf.salary_per_tick += per_employee;
                }
            }
            Command::FireEmployee { entity } => {
                // entity here refers to the corporation
                if let Some(wf) = self.workforces.get_mut(&entity) {
                    if wf.employee_count > 1 {
                        let per_employee = wf.salary_per_tick / wf.employee_count as Money;
                        wf.employee_count -= 1;
                        wf.salary_per_tick -= per_employee;
                        if let Some(fin) = self.financials.get_mut(&entity) {
                            fin.cost_per_tick -= per_employee;
                        }
                    }
                }
            }
            Command::SetPolicy {
                corporation,
                policy,
                value,
            } => {
                if let Some(p) = self.policies.get_mut(&corporation) {
                    p.set(policy, value);
                }
            }
            Command::RepairNode { entity } => {
                self.cmd_repair_node(entity, false);
            }
            Command::EmergencyRepair { entity } => {
                self.cmd_repair_node(entity, true);
            }
            Command::CreateSubsidiary { parent, name } => {
                self.cmd_create_subsidiary(parent, &name);
            }
            Command::PurchaseInsurance { node } => {
                self.cmd_purchase_insurance(node);
            }
            Command::CancelInsurance { node } => {
                if let Some(n) = self.infra_nodes.get_mut(&node) {
                    n.insured = false;
                }
            }
            // Bankruptcy & Auctions
            Command::DeclareBankruptcy { entity } => {
                self.cmd_declare_bankruptcy(entity);
            }
            Command::RequestBailout { entity } => {
                self.cmd_request_bailout(entity);
            }
            Command::AcceptBailout { entity } => {
                self.cmd_accept_bailout(entity);
            }
            Command::PlaceBid { auction, amount } => {
                self.cmd_place_bid(auction, amount);
            }

            // Mergers & Acquisitions
            Command::ProposeAcquisition { target, offer } => {
                self.cmd_propose_acquisition(target, offer);
            }
            Command::RespondToAcquisition { proposal, accept } => {
                self.cmd_respond_to_acquisition(proposal, accept);
            }

            // Espionage & Sabotage
            Command::LaunchEspionage { target, region } => {
                self.cmd_launch_espionage(target, region);
            }
            Command::LaunchSabotage { target, node } => {
                self.cmd_launch_sabotage(target, node);
            }
            Command::UpgradeSecurity { level } => {
                self.cmd_upgrade_security(level);
            }

            // Lobbying
            Command::StartLobbying {
                region,
                policy,
                budget,
            } => {
                self.cmd_start_lobbying(region, &policy, budget);
            }
            Command::CancelLobbying { lobby_id } => {
                self.lobbying_campaigns.remove(&lobby_id);
            }

            // Cooperative Infrastructure
            Command::ProposeCoOwnership {
                node,
                target_corp,
                share_pct,
            } => {
                self.cmd_propose_co_ownership(node, target_corp, share_pct);
            }
            Command::RespondCoOwnership { proposal, accept } => {
                self.cmd_respond_co_ownership(proposal, accept);
            }
            Command::ProposeBuyout {
                node,
                target_corp,
                price,
            } => {
                self.cmd_propose_buyout(node, target_corp, price);
            }
            Command::VoteUpgrade { node, approve } => {
                self.cmd_vote_upgrade(node, approve);
            }

            Command::AssignTeam { .. } | Command::SaveGame { .. } | Command::LoadGame { .. } => {
                // Handled externally
            }
        }
    }

    /// Find the nearest grid cell to the given (lon, lat) coordinates.
    /// Returns (cell_index, distance_degrees) or None if no cells exist.
    pub fn find_nearest_cell(&self, lon: f64, lat: f64) -> Option<(usize, f64)> {
        if self.grid_cell_positions.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        for (idx, &(cell_lat, cell_lon)) in self.grid_cell_positions.iter().enumerate() {
            let dlat = cell_lat - lat;
            let dlon = cell_lon - lon;
            let dist_sq = dlat * dlat + dlon * dlon;
            if dist_sq < best_dist {
                best_dist = dist_sq;
                best_idx = idx;
            }
        }
        Some((best_idx, best_dist.sqrt()))
    }

    /// Get the terrain for a given cell index by looking up the land parcel.
    pub fn get_cell_terrain(&self, cell_index: usize) -> Option<TerrainType> {
        self.cell_to_parcel
            .get(&cell_index)
            .and_then(|pid| self.land_parcels.get(pid))
            .map(|p| p.terrain)
    }

    fn cmd_build_node(&mut self, node_type: NodeType, lon: f64, lat: f64) {
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
                return;
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
                return;
            }
        };

        // Get terrain from nearest cell's parcel
        let terrain = match self.get_cell_terrain(cell_index) {
            Some(t) => t,
            None => TerrainType::Rural, // fallback if no parcel data
        };

        // Terrain constraints for node types
        match node_type {
            NodeType::SubmarineLanding => {
                if !matches!(terrain, TerrainType::Coastal | TerrainType::OceanShallow) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Submarine landing requires coastal or shallow ocean terrain.".to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return;
                }
            }
            NodeType::CellTower
            | NodeType::WirelessRelay
            | NodeType::CentralOffice
            | NodeType::ExchangePoint
            | NodeType::DataCenter
            | NodeType::BackboneRouter => {
                if matches!(terrain, TerrainType::OceanDeep | TerrainType::OceanShallow) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: format!(
                                "Cannot build {:?}: Requires solid ground.",
                                node_type
                            ),
                            level: "error".to_string(),
                        },
                    );
                    return;
                }
            }
            NodeType::SatelliteGround => {
                if matches!(terrain, TerrainType::OceanDeep) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Satellite ground station cannot be placed on deep ocean.".to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return;
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
                return;
            }
        } else {
            return;
        }

        // Deduct cost
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        // Create construction (takes time)
        let difficulty = gt_common::config::DifficultyConfig::from_preset(self.config.difficulty);
        let base_duration = match node_type {
            NodeType::CellTower | NodeType::WirelessRelay => 10,
            NodeType::CentralOffice => 20,
            NodeType::ExchangePoint => 30,
            NodeType::DataCenter | NodeType::BackboneRouter => 50,
            NodeType::SatelliteGround => 40,
            NodeType::SubmarineLanding => 60,
        };
        let duration = (base_duration as f64 * difficulty.construction_time_multiplier) as Tick;

        let node_id = self.allocate_entity();
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
    }

    fn cmd_build_edge(&mut self, edge_type: EdgeType, from_node: EntityId, to_node: EntityId) {
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
                return;
            }
        };

        // Verify target node exists and belongs to the same corp
        match self.infra_nodes.get(&to_node) {
            Some(n) if n.owner == corp_id => {}
            Some(_) => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot connect: Target node belongs to a different corporation."
                            .to_string(),
                        level: "error".to_string(),
                    },
                );
                return;
            }
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Cannot connect: Target node not found.".to_string(),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        }

        // Enforce tier compatibility: edge type must be valid for the node tiers
        let from_type = self.infra_nodes.get(&from_node).map(|n| n.node_type);
        let to_type = self.infra_nodes.get(&to_node).map(|n| n.node_type);
        if let (Some(ft), Some(tt)) = (from_type, to_type) {
            if !edge_type.can_connect(ft, tt) {
                // Build a suggestion of which edge types CAN connect these two node types
                let all_types = [
                    EdgeType::Copper, EdgeType::FiberLocal, EdgeType::Microwave,
                    EdgeType::FiberRegional, EdgeType::FiberNational,
                    EdgeType::Satellite, EdgeType::Submarine,
                ];
                let suggestions: Vec<&str> = all_types.iter()
                    .filter(|et| et.can_connect(ft, tt))
                    .map(|et| et.display_name())
                    .collect();
                let hint = if suggestions.is_empty() {
                    format!("{} and {} are too far apart in tier — build intermediate nodes.", ft.display_name(), tt.display_name())
                } else {
                    format!("Try: {}", suggestions.join(", "))
                };
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "{} cannot connect {} ({}) to {} ({}). {}",
                            edge_type.display_name(),
                            ft.display_name(), ft.tier().display_name(),
                            tt.display_name(), tt.tier().display_name(),
                            hint
                        ),
                        level: "error".to_string(),
                    },
                );
                return;
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
                EdgeType::Copper, EdgeType::FiberLocal, EdgeType::Microwave,
                EdgeType::FiberRegional, EdgeType::FiberNational,
                EdgeType::Satellite, EdgeType::Submarine,
            ];
            let suggestion = if let (Some(ft), Some(tt)) = (from_ft, to_ft) {
                all_types.iter()
                    .filter(|et| et.can_connect(ft, tt) && self.cell_spacing_km * et.distance_multiplier() >= length_km)
                    .map(|et| format!("{} ({:.0}km range)", et.display_name(), self.cell_spacing_km * et.distance_multiplier()))
                    .next()
                    .map(|s| format!(" Try: {}", s))
                    .unwrap_or_else(|| " Build intermediate relay nodes to bridge the gap.".to_string())
            } else {
                String::new()
            };
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!(
                        "Too far for {}: {:.0}km distance, {:.0}km max range.{}",
                        edge_type.display_name(), length_km, max_distance_km, suggestion
                    ),
                    level: "warning".to_string(),
                },
            );
            return;
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
            EdgeType::Submarine => {
                // Submarine edges must have at least one endpoint on ocean/coastal
                let is_water = |t: Option<TerrainType>| {
                    matches!(
                        t,
                        Some(
                            TerrainType::OceanShallow
                                | TerrainType::OceanDeep
                                | TerrainType::Coastal
                        )
                    )
                };
                if !is_water(from_terrain) && !is_water(to_terrain) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Submarine edges require at least one endpoint on water.".to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return; // Submarine requires water
                }
            }
            EdgeType::Copper
            | EdgeType::FiberLocal
            | EdgeType::FiberRegional
            | EdgeType::FiberNational => {
                // Land-based cables can't span open ocean
                let is_deep_ocean =
                    |t: Option<TerrainType>| matches!(t, Some(TerrainType::OceanDeep));
                if is_deep_ocean(from_terrain) || is_deep_ocean(to_terrain) {
                    self.event_queue.push(
                        self.tick,
                        gt_common::events::GameEvent::GlobalNotification {
                            message: "Cannot build: Wired cables cannot be placed in deep ocean."
                                .to_string(),
                            level: "error".to_string(),
                        },
                    );
                    return; // Fiber/copper can't be on deep ocean
                }
            }
            _ => {} // Microwave, Satellite have no terrain restriction
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

        let edge = InfraEdge::new(edge_type, from_node, to_node, length_km, corp_id);
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
                            edge_type.display_name(), cost, fin.cash
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
            fin.cash -= cost;
        }

        let edge_id = self.allocate_entity();
        self.infra_edges.insert(edge_id, edge);
        self.network.add_edge_with_id(from_node, to_node, edge_id);

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
    }

    fn cmd_upgrade_node(&mut self, entity: EntityId) {
        let corp_id = match self.infra_nodes.get(&entity) {
            Some(n) => n.owner,
            None => return,
        };

        let upgrade_cost = match self.infra_nodes.get(&entity) {
            Some(n) => n.construction_cost / 2,
            None => return,
        };

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < upgrade_cost {
                return;
            }
        }

        if let Some(fin) = self.financials.get_mut(&corp_id) {
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
    }

    fn cmd_decommission_node(&mut self, entity: EntityId) {
        if let Some(node) = self.infra_nodes.remove(&entity) {
            let corp_id = node.owner;
            // Remove from network
            self.network.remove_node(entity);
            // Remove associated edges
            let edges_to_remove: Vec<EntityId> = self
                .infra_edges
                .iter()
                .filter(|(_, e)| e.source == entity || e.target == entity)
                .map(|(&id, _)| id)
                .collect();
            for eid in &edges_to_remove {
                if let Some(edge) = self.infra_edges.remove(eid) {
                    if let Some(fin) = self.financials.get_mut(&corp_id) {
                        fin.cost_per_tick = (fin.cost_per_tick - edge.maintenance_cost).max(0);
                    }
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
        }
    }

    fn cmd_repair_node(&mut self, entity: EntityId, emergency: bool) {
        let corp_id = match self.infra_nodes.get(&entity) {
            Some(n) => n.owner,
            None => return,
        };

        let current_health = match self.healths.get(&entity) {
            Some(h) => h.condition,
            None => return,
        };

        if current_health >= 0.95 {
            return; // Already healthy
        }

        let base_cost = match self.infra_nodes.get(&entity) {
            Some(n) => n.construction_cost,
            None => return,
        };

        // Repair cost is proportional to damage; emergency is 3x more expensive
        let damage = 1.0 - current_health;
        let multiplier = if emergency { 0.6 } else { 0.2 };
        let cost = (base_cost as f64 * damage * multiplier) as Money;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }

        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        if emergency {
            // Instant repair
            if let Some(health) = self.healths.get_mut(&entity) {
                health.condition = 1.0;
            }
            // Restore capacity
            if let Some(node) = self.infra_nodes.get(&entity) {
                let max_tp = node.max_throughput;
                if let Some(cap) = self.capacities.get_mut(&entity) {
                    cap.max_throughput = max_tp;
                }
            }
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::RepairCompleted { entity },
            );
        } else {
            // Gradual repair: boost health significantly
            if let Some(health) = self.healths.get_mut(&entity) {
                health.condition = (health.condition + 0.5).min(1.0);
            }
            // Partially restore capacity
            if let Some(node) = self.infra_nodes.get(&entity) {
                let max_tp = node.max_throughput;
                if let Some(cap) = self.capacities.get_mut(&entity) {
                    cap.max_throughput = cap.max_throughput.max(max_tp * 0.8);
                }
            }
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::RepairStarted { entity, cost },
            );
        }
    }

    fn cmd_create_subsidiary(&mut self, parent: EntityId, name: &str) {
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

    fn cmd_purchase_insurance(&mut self, node: EntityId) {
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

    fn cmd_take_loan(&mut self, corporation: EntityId, amount: Money) {
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

    fn cmd_repay_loan(&mut self, loan_id: EntityId, amount: Money) {
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
                self.debt_instruments.remove(&loan_id);
            }
        }
    }

    fn cmd_propose_contract(&mut self, from: EntityId, to: EntityId, _terms: &str) {
        let contract = Contract::new_proposal(
            ContractType::Transit,
            from,
            to,
            1000.0, // capacity
            500,    // price per tick
            self.tick,
            180,    // 6 months
            10_000, // penalty
        );
        let contract_id = self.allocate_entity();
        self.contracts.insert(contract_id, contract);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ContractProposed {
                entity: contract_id,
                from,
                to,
            },
        );
    }

    fn cmd_start_research(&mut self, corporation: EntityId, tech: &str) {
        let category = match tech {
            "optical" | "OpticalNetworks" => ResearchCategory::OpticalNetworks,
            "wireless" | "Wireless5G" => ResearchCategory::Wireless5G,
            "satellite" | "Satellite" => ResearchCategory::Satellite,
            "datacenter" | "DataCenter" => ResearchCategory::DataCenter,
            "resilience" | "NetworkResilience" => ResearchCategory::NetworkResilience,
            "efficiency" | "OperationalEfficiency" => ResearchCategory::OperationalEfficiency,
            _ => return,
        };

        // Check if already researching
        let already_researching = self
            .tech_research
            .values()
            .any(|r| r.researcher == Some(corporation) && !r.completed);
        if already_researching {
            return;
        }

        let research_id = self.allocate_entity();
        let mut research = TechResearch::new(category, tech);
        research.researcher = Some(corporation);

        self.tech_research.insert(research_id, research);
        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ResearchStarted {
                corporation,
                tech: tech.to_string(),
            },
        );
    }

    // === Phase 10.1: Bankruptcy & Auctions ===

    fn cmd_declare_bankruptcy(&mut self, entity: EntityId) {
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
            self.debt_instruments.remove(&id);
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

    fn cmd_request_bailout(&mut self, entity: EntityId) {
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

    fn cmd_accept_bailout(&mut self, entity: EntityId) {
        // Same as request — this is the confirmation path
        self.cmd_request_bailout(entity);
    }

    fn cmd_place_bid(&mut self, auction_id: EntityId, amount: Money) {
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

    // === Phase 10.2: Mergers & Acquisitions ===

    fn cmd_propose_acquisition(&mut self, target: EntityId, offer: Money) {
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

    fn cmd_respond_to_acquisition(&mut self, proposal_id: EntityId, accept: bool) {
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

    /// Transfer all assets from one corporation to another.
    pub fn transfer_corporation_assets(&mut self, from: EntityId, to: EntityId) {
        // Transfer infrastructure nodes
        let nodes = self.corp_infra_nodes.remove(&from).unwrap_or_default();
        for &node_id in &nodes {
            if let Some(node) = self.infra_nodes.get_mut(&node_id) {
                node.owner = to;
            }
            if let Some(own) = self.ownerships.get_mut(&node_id) {
                own.owner = to;
            }
        }
        self.corp_infra_nodes.entry(to).or_default().extend(nodes);

        // Transfer edge ownership
        for edge in self.infra_edges.values_mut() {
            if edge.owner == from {
                edge.owner = to;
            }
        }

        // Transfer contracts
        for contract in self.contracts.values_mut() {
            if contract.from == from {
                contract.from = to;
            }
            if contract.to == from {
                contract.to = to;
            }
        }

        // Merge financials
        let from_fin = self.financials.remove(&from).unwrap_or(Financial {
            cash: 0,
            revenue_per_tick: 0,
            cost_per_tick: 0,
            debt: 0,
        });
        if let Some(to_fin) = self.financials.get_mut(&to) {
            to_fin.cash += from_fin.cash;
            to_fin.debt += from_fin.debt;
        }

        // Transfer workforce
        if let Some(from_wf) = self.workforces.remove(&from) {
            if let Some(to_wf) = self.workforces.get_mut(&to) {
                to_wf.employee_count += from_wf.employee_count;
            }
        }

        // Transfer debt instruments
        for debt in self.debt_instruments.values_mut() {
            if debt.holder == from {
                debt.holder = to;
            }
        }

        // Remove the absorbed corporation
        self.corporations.remove(&from);
        self.ai_states.remove(&from);
        self.policies.remove(&from);
        self.covert_ops.remove(&from);
        self.achievements.remove(&from);
    }

    // === Phase 10.3: Espionage & Sabotage ===

    fn cmd_launch_espionage(&mut self, target: EntityId, region: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Base cost depends on target's security level
        let target_security = self
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 100_000 + target_security as Money * 100_000;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let attacker_security = self
            .covert_ops
            .get(&corp_id)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let success_chance = 0.8 - target_security as f64 * 0.1 + attacker_security as f64 * 0.05;

        let mission = Mission {
            mission_type: MissionType::Espionage,
            target,
            region,
            start_tick: self.tick,
            duration: 20,
            cost,
            success_chance: success_chance.clamp(0.1, 0.95),
            completed: false,
        };

        self.covert_ops
            .entry(corp_id)
            .or_insert_with(CovertOps::new)
            .active_missions
            .push(mission);
    }

    fn cmd_launch_sabotage(&mut self, target: EntityId, node: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // If no specific node given, pick a random node owned by target
        let actual_node = if node == 0 {
            self.corp_infra_nodes
                .get(&target)
                .and_then(|nodes| {
                    if nodes.is_empty() { None }
                    else {
                        // Use tick as a simple deterministic "random" index
                        let idx = self.tick as usize % nodes.len();
                        nodes.iter().nth(idx).copied()
                    }
                })
                .unwrap_or(0)
        } else {
            node
        };

        // Must have a valid target node
        if actual_node == 0 {
            return;
        }

        let target_security = self
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 200_000 + target_security as Money * 200_000;

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let attacker_security = self
            .covert_ops
            .get(&corp_id)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let detection_chance = 0.2 - attacker_security as f64 * 0.05 + target_security as f64 * 0.1;

        let region = self
            .positions
            .get(&actual_node)
            .and_then(|p| p.region_id)
            .unwrap_or(0);

        let mission = Mission {
            mission_type: MissionType::Sabotage,
            target,
            region,
            start_tick: self.tick,
            duration: 15,
            cost,
            success_chance: (1.0 - detection_chance).clamp(0.1, 0.95),
            completed: false,
        };

        self.covert_ops
            .entry(corp_id)
            .or_insert_with(CovertOps::new)
            .active_missions
            .push(mission);
    }

    fn cmd_upgrade_security(&mut self, level: u32) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let cost = level as Money * 500_000;
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
                return;
            }
        }
        if let Some(fin) = self.financials.get_mut(&corp_id) {
            fin.cash -= cost;
        }

        let ops = self
            .covert_ops
            .entry(corp_id)
            .or_insert_with(CovertOps::new);
        ops.security_level = level;

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SecurityUpgraded {
                corporation: corp_id,
                level,
            },
        );
    }

    // === Phase 10.4: Lobbying ===

    fn cmd_start_lobbying(&mut self, region: EntityId, policy: &str, budget: Money) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        if !self.regions.contains_key(&region) {
            return;
        }

        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < budget {
                return;
            }
        }

        let lobby_policy = match policy {
            "ReduceTax" => LobbyPolicy::ReduceTax,
            "RelaxZoning" => LobbyPolicy::RelaxZoning,
            "FastTrackPermits" => LobbyPolicy::FastTrackPermits,
            "IncreasedCompetitorBurden" => LobbyPolicy::IncreasedCompetitorBurden,
            "SubsidyRequest" => LobbyPolicy::SubsidyRequest,
            _ => return,
        };

        let campaign = LobbyingCampaign::new(corp_id, region, lobby_policy, budget, self.tick);
        let campaign_id = self.allocate_entity();
        self.lobbying_campaigns.insert(campaign_id, campaign);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::LobbyingStarted {
                corporation: corp_id,
                region,
                policy: policy.to_string(),
            },
        );
    }

    // === Phase 10.5: Cooperative Infrastructure ===

    fn cmd_propose_co_ownership(&mut self, node: EntityId, target_corp: EntityId, share_pct: f64) {
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

        // Add co-owner to ownership component
        if let Some(ownership) = self.ownerships.get_mut(&node) {
            // Check if already a co-owner
            if ownership.co_owners.iter().any(|(id, _)| *id == target_corp) {
                return;
            }
            ownership.co_owners.push((target_corp, share_pct));
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::CoOwnershipEstablished {
                node,
                partner: target_corp,
                share_pct,
            },
        );
    }

    fn cmd_respond_co_ownership(&mut self, _proposal: EntityId, _accept: bool) {
        // AI responds to co-ownership proposals — handled in AI system
    }

    fn cmd_propose_buyout(&mut self, node: EntityId, target_corp: EntityId, price: Money) {
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

    fn cmd_vote_upgrade(&mut self, node: EntityId, _approve: bool) {
        // For player voting on co-owned upgrade — simplified: just upgrade if player approves
        if _approve {
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

fn archetype_skill_bonus(archetype: AIArchetype) -> f64 {
    match archetype {
        AIArchetype::TechInnovator => 0.3,
        AIArchetype::DefensiveConsolidator => 0.2,
        AIArchetype::AggressiveExpander => 0.1,
        AIArchetype::BudgetOperator => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_world() {
        let world = GameWorld::new(WorldConfig::default());
        assert_eq!(world.current_tick(), 0);
        assert!(world.entity_count() > 0);
    }

    #[test]
    fn test_tick_advances() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.tick();
        assert_eq!(world.current_tick(), 1);
        world.tick();
        assert_eq!(world.current_tick(), 2);
    }

    #[test]
    fn test_pause_prevents_tick() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.process_command(Command::SetSpeed(GameSpeed::Paused));
        world.tick();
        assert_eq!(world.current_tick(), 0);
    }

    #[test]
    fn test_toggle_pause() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.process_command(Command::TogglePause);
        assert_eq!(world.speed(), GameSpeed::Paused);
        world.process_command(Command::TogglePause);
        assert_eq!(world.speed(), GameSpeed::Normal);
    }

    #[test]
    fn test_world_has_regions_and_cities() {
        let world = GameWorld::new(WorldConfig {
            map_size: MapSize::Small,
            ..WorldConfig::default()
        });
        assert!(!world.regions.is_empty(), "World should have regions");
        assert!(!world.cities.is_empty(), "World should have cities");
        assert!(
            !world.land_parcels.is_empty(),
            "World should have land parcels"
        );
    }

    #[test]
    fn test_world_has_corporations() {
        let config = WorldConfig {
            ai_corporations: 4,
            map_size: MapSize::Small,
            ..WorldConfig::default()
        };
        let world = GameWorld::new(config);
        assert_eq!(world.corporations.len(), 5); // 1 player + 4 AI
        assert!(world.player_corp_id().is_some());
    }

    #[test]
    fn test_construction_completes() {
        let mut world = GameWorld::new(WorldConfig {
            map_size: MapSize::Small,
            ..WorldConfig::default()
        });
        let entity = world.allocate_entity();
        world.constructions.insert(entity, Construction::new(0, 3));

        world.tick(); // tick 1
        world.tick(); // tick 2
        assert!(world.constructions.contains_key(&entity));

        world.tick(); // tick 3 — construction completes
        assert!(!world.constructions.contains_key(&entity));
    }

    #[test]
    fn test_cost_deducted_from_cash() {
        let mut world = GameWorld::new(WorldConfig {
            map_size: MapSize::Small,
            ..WorldConfig::default()
        });
        // Player corp gets created automatically — verify cost deduction works
        let corp_id = world.player_corp_id().unwrap();
        let initial_cash = world.financials[&corp_id].cash;

        // Set a known cost
        world.financials.get_mut(&corp_id).unwrap().cost_per_tick = 100;
        world.financials.get_mut(&corp_id).unwrap().revenue_per_tick = 0;

        world.tick();

        // Cash should decrease by cost_per_tick (cost system recalculates, so check decrease)
        assert!(
            world.financials[&corp_id].cash < initial_cash,
            "Cash should decrease after tick with costs"
        );
    }

    #[test]
    fn test_take_loan() {
        let mut world = GameWorld::new(WorldConfig {
            map_size: MapSize::Small,
            ..WorldConfig::default()
        });
        let corp_id = world.player_corp_id().unwrap();
        let initial_cash = world.financials[&corp_id].cash;

        world.process_command(Command::TakeLoan {
            corporation: corp_id,
            amount: 1_000_000,
        });

        assert_eq!(world.financials[&corp_id].cash, initial_cash + 1_000_000);
        assert!(world.financials[&corp_id].debt > 0);
        assert!(!world.debt_instruments.is_empty());
    }

    #[test]
    #[cfg(feature = "native-compression")]
    fn test_save_load_binary_roundtrip() {
        let config = WorldConfig {
            map_size: MapSize::Small,
            ai_corporations: 1,
            ..WorldConfig::default()
        };
        let mut world = GameWorld::new(config);
        for _ in 0..10 {
            world.tick();
        }

        let binary = world.save_game_binary().expect("save should succeed");
        let loaded = GameWorld::load_game_binary(&binary).expect("load should succeed");

        assert_eq!(world.current_tick(), loaded.current_tick());
        assert_eq!(world.regions.len(), loaded.regions.len());
        assert_eq!(world.corporations.len(), loaded.corporations.len());
    }

    #[test]
    #[cfg(feature = "native-compression")]
    fn test_save_binary_corruption_detected() {
        let config = WorldConfig {
            map_size: MapSize::Small,
            ai_corporations: 1,
            ..WorldConfig::default()
        };
        let world = GameWorld::new(config);
        let mut binary = world.save_game_binary().expect("save should succeed");

        // Verify it loads fine unmodified
        assert!(GameWorld::load_game_binary(&binary).is_ok());

        // Corrupt a byte in the payload (after version + crc)
        if binary.len() > 10 {
            binary[10] ^= 0xFF;
        }

        let result = GameWorld::load_game_binary(&binary);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("CRC32 mismatch") || err.contains("decompress"),
            "Expected corruption error, got: {}",
            err
        );
    }

    #[test]
    #[cfg(feature = "native-compression")]
    fn test_save_binary_empty_data() {
        let result = GameWorld::load_game_binary(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty save data"));
    }

    #[test]
    #[cfg(feature = "native-compression")]
    fn test_save_binary_unsupported_version() {
        let result = GameWorld::load_game_binary(&[99, 0, 0, 0, 0]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported save version"));
    }

    #[test]
    fn test_save_game_json_roundtrip() {
        let config = WorldConfig {
            map_size: MapSize::Small,
            ai_corporations: 1,
            ..WorldConfig::default()
        };
        let mut world = GameWorld::new(config);
        for _ in 0..10 {
            world.tick();
        }

        // This was previously broken due to tuple-keyed HashMaps
        let json = world.save_game().expect("JSON save should succeed");
        assert!(!json.is_empty());

        let loaded = GameWorld::load_game(&json).expect("JSON load should succeed");
        assert_eq!(world.current_tick(), loaded.current_tick());
        assert_eq!(world.regions.len(), loaded.regions.len());
        assert_eq!(world.corporations.len(), loaded.corporations.len());
    }

    #[test]
    fn test_determinism() {
        let config = WorldConfig {
            seed: 42,
            map_size: MapSize::Small,
            ai_corporations: 2,
            ..WorldConfig::default()
        };

        let mut w1 = GameWorld::new(config.clone());
        let mut w2 = GameWorld::new(config);

        for _ in 0..50 {
            w1.tick();
            w2.tick();
        }

        assert_eq!(w1.current_tick(), w2.current_tick());
        assert_eq!(w1.regions.len(), w2.regions.len());
        assert_eq!(w1.cities.len(), w2.cities.len());

        // Check financial state matches
        for (&id, f1) in &w1.financials {
            if let Some(f2) = w2.financials.get(&id) {
                assert_eq!(f1.cash, f2.cash, "Cash mismatch for entity {}", id);
            }
        }
    }
}
