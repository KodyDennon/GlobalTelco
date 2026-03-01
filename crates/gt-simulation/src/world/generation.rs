use std::collections::HashMap;

use gt_common::types::*;

use crate::components::*;

use super::utils::{archetype_skill_bonus, haversine_km_deg};
use super::GameWorld;

impl GameWorld {
    pub(super) fn generate_world(&mut self) {
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
            let cell_budget = cell_budget.clamp(1, 15);

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

        // Generate road network connecting cities
        self.generate_road_network();
    }

    /// Build a road network graph:
    /// 1. MST of cities within each region (Secondary roads)
    /// 2. Inter-region highways connecting adjacent region capitals
    pub(super) fn generate_road_network(&mut self) {
        let mut next_road_id: u64 = 1;

        // Collect city positions: (city_id, lon, lat, region_id)
        let city_data: Vec<(EntityId, f64, f64, EntityId)> = self
            .cities
            .iter()
            .filter_map(|(&id, city)| {
                let pos = self.positions.get(&id)?;
                Some((id, pos.x, pos.y, city.region_id))
            })
            .collect();

        // Group cities by region
        let mut region_cities: HashMap<EntityId, Vec<(EntityId, f64, f64)>> = HashMap::new();
        for &(city_id, lon, lat, region_id) in &city_data {
            region_cities
                .entry(region_id)
                .or_default()
                .push((city_id, lon, lat));
        }

        // For each region, build MST of cities (Prim's algorithm)
        for (&region_id, cities) in &region_cities {
            if cities.len() < 2 {
                continue;
            }

            let n = cities.len();
            let mut in_mst = vec![false; n];
            let mut min_cost = vec![f64::MAX; n];
            let mut min_from = vec![0usize; n];
            in_mst[0] = true;

            // Initialize costs from city 0
            for j in 1..n {
                let dist = haversine_km_deg(cities[0].1, cities[0].2, cities[j].1, cities[j].2);
                min_cost[j] = dist;
                min_from[j] = 0;
            }

            for _ in 1..n {
                // Find closest city not yet in MST
                let mut best_idx = 0;
                let mut best_cost = f64::MAX;
                for j in 0..n {
                    if !in_mst[j] && min_cost[j] < best_cost {
                        best_cost = min_cost[j];
                        best_idx = j;
                    }
                }

                in_mst[best_idx] = true;

                // Add road segment between min_from[best_idx] and best_idx
                let from_city = &cities[min_from[best_idx]];
                let to_city = &cities[best_idx];
                let length_km = haversine_km_deg(from_city.1, from_city.2, to_city.1, to_city.2);

                // Road class based on city populations
                let from_pop = self
                    .cities
                    .get(&from_city.0)
                    .map(|c| c.population)
                    .unwrap_or(0);
                let to_pop = self
                    .cities
                    .get(&to_city.0)
                    .map(|c| c.population)
                    .unwrap_or(0);
                let max_pop = from_pop.max(to_pop);
                let road_class = if max_pop > 500_000 {
                    RoadClass::Primary
                } else if max_pop > 100_000 {
                    RoadClass::Secondary
                } else {
                    RoadClass::Residential
                };

                self.road_network.add_segment(RoadSegment {
                    id: next_road_id,
                    from: (from_city.1, from_city.2),
                    to: (to_city.1, to_city.2),
                    road_class,
                    length_km,
                    region_id,
                });
                next_road_id += 1;

                // Update costs
                for j in 0..n {
                    if !in_mst[j] {
                        let dist = haversine_km_deg(
                            cities[best_idx].1,
                            cities[best_idx].2,
                            cities[j].1,
                            cities[j].2,
                        );
                        if dist < min_cost[j] {
                            min_cost[j] = dist;
                            min_from[j] = best_idx;
                        }
                    }
                }
            }
        }

        // Inter-region highways: connect nearest cities between adjacent regions
        // Two regions are "adjacent" if their centers are within a reasonable distance
        let region_data: Vec<(EntityId, f64, f64)> = self
            .regions
            .iter()
            .map(|(&id, r)| (id, r.center_lon, r.center_lat))
            .collect();

        // Find average inter-region distance for threshold
        let mut all_dists: Vec<f64> = Vec::new();
        for i in 0..region_data.len() {
            for j in (i + 1)..region_data.len() {
                let dist = haversine_km_deg(
                    region_data[i].1,
                    region_data[i].2,
                    region_data[j].1,
                    region_data[j].2,
                );
                all_dists.push(dist);
            }
        }
        all_dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Use median distance * 1.5 as adjacency threshold, or a default
        let adjacency_threshold = if all_dists.len() >= 2 {
            all_dists[all_dists.len() / 2] * 1.5
        } else {
            5000.0 // fallback: 5000km
        };

        for i in 0..region_data.len() {
            for j in (i + 1)..region_data.len() {
                let region_dist = haversine_km_deg(
                    region_data[i].1,
                    region_data[i].2,
                    region_data[j].1,
                    region_data[j].2,
                );
                if region_dist > adjacency_threshold {
                    continue;
                }

                let r1_id = region_data[i].0;
                let r2_id = region_data[j].0;

                // Find nearest pair of cities between the two regions
                let r1_cities = match region_cities.get(&r1_id) {
                    Some(c) => c,
                    None => continue,
                };
                let r2_cities = match region_cities.get(&r2_id) {
                    Some(c) => c,
                    None => continue,
                };

                type CoordPair = ((f64, f64), (f64, f64), f64);
                let mut best_pair: Option<CoordPair> = None;
                for c1 in r1_cities {
                    for c2 in r2_cities {
                        let dist = haversine_km_deg(c1.1, c1.2, c2.1, c2.2);
                        if best_pair.is_none() || dist < best_pair.unwrap().2 {
                            best_pair = Some(((c1.1, c1.2), (c2.1, c2.2), dist));
                        }
                    }
                }

                if let Some((from, to, length_km)) = best_pair {
                    self.road_network.add_segment(RoadSegment {
                        id: next_road_id,
                        from,
                        to,
                        road_class: RoadClass::Highway,
                        length_km,
                        region_id: r1_id, // assign to first region
                    });
                    next_road_id += 1;
                }
            }
        }
    }

    pub(super) fn create_corporations(&mut self) {
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
                maintenance_crew_count: 1,
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
                    maintenance_crew_count: 2,
                },
            );

            // Give AI some starting infrastructure in random regions
            self.seed_ai_infrastructure(ai_id, i);
        }
    }

    pub(super) fn seed_ai_infrastructure(&mut self, corp_id: EntityId, corp_index: usize) {
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
                                    .map(|p| p.terrain.is_land())
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
                                    .map(|p| p.terrain.is_land())
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
                let length_km = match (self.positions.get(&hub_id), self.positions.get(&spoke_id)) {
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

    pub(super) fn seed_tech_tree(&mut self) {
        let techs = crate::components::tech_research::generate_tech_tree();
        for tech in techs {
            let id = self.allocate_entity();
            self.tech_research.insert(id, tech);
        }
    }

    /// Seed initial building footprints for all cities based on their population.
    /// Buildings are distributed across city cells with higher demand in center cells
    /// and lower demand (fringe) in outer cells.
    pub(super) fn seed_buildings(&mut self) {
        use crate::components::building::{
            compute_building_target, BuildingFootprint, CityBuildingCensus,
        };

        let city_data: Vec<(EntityId, u64, Vec<usize>, usize, f64)> = self
            .cities
            .iter()
            .map(|(&id, city)| {
                (
                    id,
                    city.population,
                    city.cells.clone(),
                    city.cell_index,
                    city.telecom_demand,
                )
            })
            .collect();

        for (city_id, population, cells, center_cell, telecom_demand) in city_data {
            let target = compute_building_target(population);
            if target == 0 || cells.is_empty() {
                continue;
            }

            // Distribute buildings across cells. Center cell gets the densest allocation.
            let cell_count = cells.len();
            let buildings_per_cell = target / cell_count as u32;
            let mut remainder = target % cell_count as u32;

            // Base demand per building: city's telecom demand divided across all buildings.
            let base_demand = if target > 0 {
                telecom_demand / target as f64
            } else {
                0.0
            };

            for (i, &cell_idx) in cells.iter().enumerate() {
                let is_center = cell_idx == center_cell;
                let is_fringe = i >= cell_count.saturating_sub(cell_count / 3).max(1);

                let mut count = buildings_per_cell;
                if remainder > 0 {
                    count += 1;
                    remainder -= 1;
                }

                // Center buildings have 1.5x demand, fringe has 0.6x
                let demand_multiplier = if is_center {
                    1.5
                } else if is_fringe {
                    0.6
                } else {
                    1.0
                };

                for _ in 0..count {
                    let id = self.allocate_entity();
                    let bldg = BuildingFootprint::new(
                        city_id,
                        cell_idx,
                        base_demand * demand_multiplier,
                        is_fringe,
                    );
                    self.building_footprints.insert(id, bldg);
                }
            }

            self.city_building_census
                .insert(city_id, CityBuildingCensus::new(population));
        }
    }
}
