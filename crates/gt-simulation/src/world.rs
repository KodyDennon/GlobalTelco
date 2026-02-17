use std::collections::HashMap;

use gt_common::commands::Command;
use gt_common::types::*;
use serde::{Deserialize, Serialize};

use crate::components::*;
use crate::events::EventQueue;
use crate::systems;

#[derive(Serialize, Deserialize)]
pub struct GameWorld {
    config: WorldConfig,
    tick: Tick,
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

    // Mappings for fast lookup
    pub cell_to_parcel: HashMap<usize, EntityId>,
    pub cell_to_region: HashMap<usize, EntityId>,
    pub cell_to_city: HashMap<usize, EntityId>,
    pub corp_infra_nodes: HashMap<EntityId, Vec<EntityId>>,

    // World grid reference (used by systems for spatial queries)
    pub grid_cell_count: usize,
    pub grid_cell_positions: Vec<(f64, f64)>, // (lat, lon) for each cell

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
            cell_to_parcel: HashMap::new(),
            cell_to_region: HashMap::new(),
            cell_to_city: HashMap::new(),
            corp_infra_nodes: HashMap::new(),
            grid_cell_count: 0,
            grid_cell_positions: Vec::new(),
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
                    base_demand: region.population as f64 * region.development * 0.5,
                    current_demand: 0.0,
                    satisfaction: 0.0,
                },
            );

            // Map cells to this region
            for &cell_idx in &region.cells {
                self.cell_to_region.insert(cell_idx, id);
            }
        }

        // Create city entities
        for city in &gen_world.cities {
            let id = self.allocate_entity();
            let real_region_id = region_id_map.get(&city.region_id).copied().unwrap_or(0);

            let econ = gen_world.economics.cities.iter().find(|e| {
                gen_world.cities.get(e.city_index).map(|c| c.cell_index) == Some(city.cell_index)
            });
            let telecom_demand = econ
                .map(|e| e.telecom_demand)
                .unwrap_or(city.population as f64 * city.development);

            self.cities.insert(
                id,
                CityComponent {
                    name: city.name.clone(),
                    region_id: real_region_id,
                    cell_index: city.cell_index,
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
            self.cell_to_city.insert(city.cell_index, id);

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
        self.corporations
            .insert(player_id, Corporation::new("Player Corp", true));
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
                let cell_index = city.cell_index;
                let node_id = self.allocate_entity();
                let node = InfraNode::new(NodeType::CentralOffice, cell_index, corp_id);
                let maintenance = node.maintenance_cost;

                self.infra_nodes.insert(node_id, node);
                self.healths.insert(node_id, Health::new());
                self.capacities.insert(node_id, Capacity::new(1000.0));
                self.ownerships.insert(node_id, Ownership::sole(corp_id));
                if let Some(&(lat, lon)) = self.grid_cell_positions.get(cell_index) {
                    self.positions.insert(
                        node_id,
                        Position {
                            x: lon,
                            y: lat,
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
                let cell_index = city.cell_index;
                let node_id = self.allocate_entity();
                let node = InfraNode::new(NodeType::CellTower, cell_index, corp_id);
                let maintenance = node.maintenance_cost;

                self.infra_nodes.insert(node_id, node);
                self.healths.insert(node_id, Health::new());
                self.capacities.insert(node_id, Capacity::new(500.0));
                self.ownerships.insert(node_id, Ownership::sole(corp_id));
                if let Some(&(lat, lon)) = self.grid_cell_positions.get(cell_index) {
                    self.positions.insert(
                        node_id,
                        Position {
                            x: lon,
                            y: lat,
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
            let hub_cell = placed_node_ids[0].1;

            for &(spoke_id, spoke_cell) in &placed_node_ids[1..] {
                // Calculate distance using haversine
                let length_km = match (
                    self.grid_cell_positions.get(hub_cell),
                    self.grid_cell_positions.get(spoke_cell),
                ) {
                    (Some(&(lat1, lon1)), Some(&(lat2, lon2))) => {
                        let dlat = (lat1 - lat2).to_radians();
                        let dlon = (lon1 - lon2).to_radians();
                        let a = (dlat / 2.0).sin().powi(2)
                            + lat1.to_radians().cos()
                                * lat2.to_radians().cos()
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
                self.network.add_edge(hub_id, spoke_id);

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
            Command::BuildNode { node_type, parcel } => {
                self.cmd_build_node(node_type, parcel);
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
                    wf.employee_count += 1;
                    if let Some(fin) = self.financials.get_mut(&corporation) {
                        fin.cost_per_tick += wf.salary_per_tick / wf.employee_count as Money;
                    }
                }
            }
            Command::FireEmployee { entity } => {
                // entity here refers to the corporation
                if let Some(wf) = self.workforces.get_mut(&entity) {
                    if wf.employee_count > 1 {
                        wf.employee_count -= 1;
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
            Command::AssignTeam { .. } | Command::SaveGame { .. } | Command::LoadGame { .. } => {
                // Handled externally
            }
        }
    }

    fn cmd_build_node(&mut self, node_type: NodeType, parcel_id: EntityId) {
        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        // Validate parcel exists and get cell index + terrain
        let (cell_index, terrain) = match self.land_parcels.get(&parcel_id) {
            Some(p) => (p.cell_index, p.terrain),
            None => return,
        };

        let node = InfraNode::new_on_terrain(node_type, cell_index, corp_id, terrain);
        let cost = node.construction_cost;

        // Check funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
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
            NodeType::DataCenter => 50,
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

        if let Some(&(lat, lon)) = self.grid_cell_positions.get(cell_index) {
            let region_id = self.cell_to_region.get(&cell_index).copied();
            self.positions.insert(
                node_id,
                Position {
                    x: lon,
                    y: lat,
                    region_id,
                },
            );
        }

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
            None => return,
        };

        // Verify target node exists
        if !self.infra_nodes.contains_key(&to_node) {
            return;
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

        let edge = InfraEdge::new(edge_type, from_node, to_node, length_km, corp_id);
        let cost = edge.construction_cost;
        let maintenance = edge.maintenance_cost;

        // Check funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < cost {
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
        self.network.add_edge(from_node, to_node);

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
