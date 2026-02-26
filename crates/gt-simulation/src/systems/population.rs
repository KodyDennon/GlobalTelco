use crate::components::*;
use crate::world::GameWorld;
use std::collections::HashMap;

pub fn run(world: &mut GameWorld) {
    // 1. Calculate jobs per city from infrastructure
    update_employment(world);

    // 2. Apply birth/death rates to city populations
    apply_demographics(world);

    // 3. Migration: people move from low-attractiveness to high-attractiveness cities
    apply_migration(world);

    // 4. Expand growing cities to neighboring cells
    expand_cities(world);

    // 5. Spawn new settlements near infrastructure in unclaimed areas (every 50 ticks)
    if world.current_tick() % 50 == 0 && world.current_tick() > 0 {
        spawn_settlements(world);
    }

    // 6. Dynamic building spawn/destruction based on population changes (every 10 ticks)
    if world.current_tick() % 10 == 0 {
        update_buildings_dynamic(world);
    }

    // 7. Update region populations from city totals
    aggregate_region_populations(world);
}

/// Count infrastructure jobs per city and update employment rates.
fn update_employment(world: &mut GameWorld) {
    // Count jobs per cell from operational (non-construction) infrastructure nodes
    let mut jobs_per_cell: HashMap<usize, u32> = HashMap::new();
    for (&node_id, node) in &world.infra_nodes {
        if !world.constructions.contains_key(&node_id) {
            *jobs_per_cell.entry(node.cell_index).or_default() += node.jobs_created();
        }
    }

    let mut city_ids: Vec<u64> = world.cities.keys().copied().collect();
    city_ids.sort_unstable();

    for city_id in city_ids {
        if let Some(city) = world.cities.get_mut(&city_id) {
            let cell = city.cell_index;
            let jobs = jobs_per_cell.get(&cell).copied().unwrap_or(0);
            city.jobs_available = jobs;

            // Employment rate: jobs / working_age_population (assume 50% of pop is working age)
            let working_age = (city.population as f64 * 0.5).max(1.0);
            let raw_rate = jobs as f64 / working_age;
            // Smooth transition (don't jump instantly), base employment from economy
            let base_employment = 0.4 + city.development * 0.3;
            let infra_bonus = raw_rate.min(0.3); // Infrastructure adds up to 30% employment
            city.employment_rate = (base_employment + infra_bonus).clamp(0.1, 0.98);
        }
    }
}

/// Apply birth and death rates to update city populations.
fn apply_demographics(world: &mut GameWorld) {
    let mut city_ids: Vec<u64> = world.cities.keys().copied().collect();
    city_ids.sort_unstable();

    for city_id in city_ids {
        if let Some(city) = world.cities.get_mut(&city_id) {
            // Birth rate boosted by infrastructure and employment
            let birth_bonus = city.infrastructure_satisfaction * 0.002
                + (city.employment_rate - 0.5).max(0.0) * 0.005;
            let effective_birth = city.birth_rate + birth_bonus;

            // Death rate slightly higher in underdeveloped areas
            let death_penalty = (1.0 - city.development) * 0.002;
            let effective_death = city.death_rate + death_penalty;

            let net_growth = effective_birth - effective_death;
            let pop_change = (city.population as f64 * net_growth) as i64;
            city.population = (city.population as i64 + pop_change).max(100) as u64;
        }

        // Sync the Population component if it exists
        if let Some(pop_comp) = world.populations.get_mut(&city_id) {
            if let Some(city) = world.cities.get(&city_id) {
                pop_comp.count = city.population;
            }
        }
    }
}

/// Migration: people move toward more attractive cities.
fn apply_migration(world: &mut GameWorld) {
    let city_data: Vec<(u64, f64, u64)> = {
        let mut data: Vec<(u64, f64, u64)> = world
            .cities
            .iter()
            .map(|(&id, city)| (id, city.attractiveness(), city.population))
            .collect();
        data.sort_unstable_by_key(|t| t.0);
        data
    };

    if city_data.len() < 2 {
        return;
    }

    let avg_score: f64 =
        city_data.iter().map(|(_, s, _)| s).sum::<f64>() / city_data.len() as f64;

    for &(city_id, score, pop) in &city_data {
        if pop < 1000 {
            continue;
        }
        let diff = score - avg_score;

        // Migration pressure: positive = gaining people, negative = losing
        let pressure = diff;

        // Migration: 0.02% of population per tick, clamped to prevent wild swings
        // Max migration = 0.5% per tick regardless of score difference
        let migration_rate = (diff * 0.0002).clamp(-0.005, 0.005);
        let migrants = (pop as f64 * migration_rate) as i64;

        if let Some(city) = world.cities.get_mut(&city_id) {
            // Never let a city drop below 5% of its starting population
            let min_pop = (pop / 20).max(500);
            city.population = (city.population as i64 + migrants).max(min_pop as i64) as u64;
            city.migration_pressure = pressure;
        }
    }
}

/// Expand cities to neighboring cells when population outgrows current footprint.
/// Cities claim adjacent land cells in the same region as they grow.
fn expand_cities(world: &mut GameWorld) {
    // Build cell_index → terrain lookup for deterministic access
    let cell_terrain: HashMap<usize, gt_common::types::TerrainType> = world
        .land_parcels
        .values()
        .map(|p| (p.cell_index, p.terrain))
        .collect();

    // Collect expansion candidates: cities that need more cells
    // Sort by id for deterministic processing order
    let mut expansions: Vec<(u64, usize, u64, u64, Vec<usize>)> = world
        .cities
        .iter()
        .filter_map(|(&id, city)| {
            let target_cells = compute_target_cells(city.population);
            if city.cells.len() >= target_cells {
                return None;
            }
            Some((
                id,
                city.cell_index,
                city.region_id,
                city.population,
                city.cells.clone(),
            ))
        })
        .collect();
    expansions.sort_by_key(|e| e.0); // Deterministic order

    for (city_id, _center_cell, region_id, _pop, current_cells) in expansions {
        let region_cells: Vec<usize> = world
            .regions
            .get(&region_id)
            .map(|r| r.cells.clone())
            .unwrap_or_default();
        let region_set: std::collections::HashSet<usize> =
            region_cells.iter().copied().collect();

        // Collect and score all candidate cells, then sort for determinism
        let mut scored: Vec<(usize, f64)> = Vec::new();

        for &ci in &current_cells {
            if let Some(neighbors) = world.grid_cell_neighbors.get(ci) {
                for &ni in neighbors {
                    if !region_set.contains(&ni) || world.cell_to_city.contains_key(&ni) {
                        continue;
                    }
                    if !world.cell_to_parcel.contains_key(&ni) {
                        continue;
                    }
                    let terrain_bonus = cell_terrain
                        .get(&ni)
                        .map(|t| match t {
                            gt_common::types::TerrainType::Urban => 1.0,
                            gt_common::types::TerrainType::Suburban => 0.8,
                            gt_common::types::TerrainType::Rural => 0.6,
                            gt_common::types::TerrainType::Coastal => 0.5,
                            _ => 0.3,
                        })
                        .unwrap_or(0.0);
                    scored.push((ni, terrain_bonus));
                }
            }
        }

        // Sort by score descending, then cell index for deterministic tie-breaking
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.dedup_by_key(|s| s.0);

        if let Some(&(new_cell, _)) = scored.first() {
            world.cell_to_city.insert(new_cell, city_id);
            if let Some(city) = world.cities.get_mut(&city_id) {
                city.cells.push(new_cell);
            }
        }
    }
}

/// Spawn new settlements near infrastructure in regions that have unclaimed habitable cells.
/// Settlements start as small villages (~500 pop) and grow naturally via demographics.
fn spawn_settlements(world: &mut GameWorld) {
    // Find cells with infrastructure but no nearby city (sorted for determinism)
    let mut infra_cells: Vec<(usize, u64)> = world
        .infra_nodes
        .iter()
        .filter(|(&id, _)| !world.constructions.contains_key(&id))
        .map(|(_, n)| (n.cell_index, n.owner))
        .collect();
    infra_cells.sort_by_key(|c| c.0);
    infra_cells.dedup_by_key(|c| c.0);

    // Build set of cells that are within 2 hops of any city
    let mut near_city: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for city in world.cities.values() {
        for &ci in &city.cells {
            near_city.insert(ci);
            if let Some(neighbors) = world.grid_cell_neighbors.get(ci) {
                for &ni in neighbors {
                    near_city.insert(ni);
                    if let Some(n2) = world.grid_cell_neighbors.get(ni) {
                        for &ni2 in n2 {
                            near_city.insert(ni2);
                        }
                    }
                }
            }
        }
    }

    // Find infra cells far from any city → candidates for new settlement
    let mut candidates: Vec<(usize, u64)> = Vec::new();
    for (cell_idx, _owner) in &infra_cells {
        if near_city.contains(cell_idx) {
            continue;
        }
        // Must be land, in a region, and not already a city
        if world.cell_to_city.contains_key(cell_idx) {
            continue;
        }
        if !world.cell_to_parcel.contains_key(cell_idx) {
            continue;
        }
        if let Some(&region_id) = world.cell_to_region.get(cell_idx) {
            candidates.push((*cell_idx, region_id));
        }
    }

    // Deterministic: sort by cell index and only spawn one per tick cycle
    candidates.sort_by_key(|c| c.0);
    if let Some(&(cell_idx, region_id)) = candidates.first() {
        // Generate a settlement name using deterministic hash
        let name = generate_settlement_name(world.current_tick(), cell_idx);

        let new_id = world.allocate_entity();

        // Position from grid
        let (lat, lon) = world
            .grid_cell_positions
            .get(cell_idx)
            .copied()
            .unwrap_or((0.0, 0.0));

        let initial_pop = 500u64;
        let development = 0.2;
        let telecom_demand = (initial_pop as f64).sqrt() * development * 2.0;

        world.cell_to_city.insert(cell_idx, new_id);

        world.cities.insert(
            new_id,
            CityComponent {
                name: name.clone(),
                region_id,
                cell_index: cell_idx,
                cells: vec![cell_idx],
                population: initial_pop,
                growth_rate: 0.005,
                development,
                telecom_demand,
                infrastructure_satisfaction: 0.0,
                jobs_available: 0,
                employment_rate: 0.4,
                birth_rate: 0.015,
                death_rate: 0.008,
                migration_pressure: 0.0,
            },
        );
        world.positions.insert(
            new_id,
            Position {
                x: lon,
                y: lat,
                region_id: Some(region_id),
            },
        );
        world.populations.insert(
            new_id,
            Population {
                count: initial_pop,
                growth_rate: 0.005,
                income_level: development,
            },
        );
        world.demands.insert(
            new_id,
            Demand {
                base_demand: telecom_demand,
                current_demand: telecom_demand,
                satisfaction: 0.0,
            },
        );

        // Add to region's city list
        if let Some(region) = world.regions.get_mut(&region_id) {
            region.city_ids.push(new_id);
        }

        // Seed initial buildings for the new settlement
        {
            use crate::components::building::{
                compute_building_target, BuildingFootprint, CityBuildingCensus,
            };
            let target = compute_building_target(initial_pop);
            let base_demand = if target > 0 {
                telecom_demand / target as f64
            } else {
                0.0
            };
            for _ in 0..target {
                let bldg_id = world.allocate_entity();
                let bldg = BuildingFootprint::new(new_id, cell_idx, base_demand * 0.6, true);
                world.building_footprints.insert(bldg_id, bldg);
            }
            world
                .city_building_census
                .insert(new_id, CityBuildingCensus::new(initial_pop));
        }

        world.event_queue.push(
            world.current_tick(),
            gt_common::events::GameEvent::GlobalNotification {
                message: format!("New settlement founded: {}", name),
                level: "info".to_string(),
            },
        );
    }
}

/// Update region populations from city totals.
fn aggregate_region_populations(world: &mut GameWorld) {
    let mut region_ids: Vec<u64> = world.regions.keys().copied().collect();
    region_ids.sort_unstable();

    for region_id in region_ids {
        let region_cities = world
            .regions
            .get(&region_id)
            .map(|r| r.city_ids.clone())
            .unwrap_or_default();

        let total_pop: u64 = region_cities
            .iter()
            .filter_map(|&cid| world.cities.get(&cid).map(|c| c.population))
            .sum();

        if let Some(region) = world.regions.get_mut(&region_id) {
            region.population = total_pop;
        }
        if let Some(pop) = world.populations.get_mut(&region_id) {
            pop.count = total_pop;
        }
    }
}

/// Compute target cell count based on population.
fn compute_target_cells(population: u64) -> usize {
    if population > 1_000_000 {
        ((population as f64).log10() * 3.0) as usize
    } else if population > 200_000 {
        ((population as f64).log10() * 2.0) as usize
    } else if population > 50_000 {
        3
    } else if population > 10_000 {
        2
    } else {
        1
    }
    .max(1)
    .min(20)
}

/// Dynamic building spawn/destruction based on city population changes.
///
/// **Growing cities:** When population exceeds the current building target, new
/// buildings spawn in the suburban fringe zone (outermost cells). These buildings
/// start as fringe buildings with 0.6x demand.
///
/// **Declining cities:** When population drops below the building target, fringe
/// buildings are marked as "Abandoned" (greyed out, zero demand). Abandoned buildings
/// persist and can be reactivated if population rebounds.
fn update_buildings_dynamic(world: &mut GameWorld) {
    use crate::components::building::{
        compute_building_target, BuildingFootprint, BuildingStatus,
    };

    // Collect city data for processing (sorted for determinism)
    let mut city_ids: Vec<u64> = world.cities.keys().copied().collect();
    city_ids.sort_unstable();

    for city_id in city_ids {
        let (population, cells, telecom_demand) = match world.cities.get(&city_id) {
            Some(city) => (city.population, city.cells.clone(), city.telecom_demand),
            None => continue,
        };

        let target = compute_building_target(population);

        // Initialize census if missing (backward compat for old saves / newly spawned cities)
        if !world.city_building_census.contains_key(&city_id) {
            world.city_building_census.insert(
                city_id,
                crate::components::CityBuildingCensus::new(population),
            );
        }

        // Count current active and abandoned buildings for this city
        let mut active_ids: Vec<u64> = Vec::new();
        let mut abandoned_ids: Vec<u64> = Vec::new();
        let mut fringe_active_ids: Vec<u64> = Vec::new();
        let mut fringe_abandoned_ids: Vec<u64> = Vec::new();

        // Collect building IDs sorted for determinism
        let mut bldg_ids: Vec<u64> = world
            .building_footprints
            .iter()
            .filter(|(_, b)| b.city_id == city_id)
            .map(|(&id, _)| id)
            .collect();
        bldg_ids.sort_unstable();

        for &bldg_id in &bldg_ids {
            let bldg = match world.building_footprints.get(&bldg_id) {
                Some(b) => b,
                None => continue,
            };
            match bldg.status {
                BuildingStatus::Active => {
                    active_ids.push(bldg_id);
                    if bldg.is_fringe {
                        fringe_active_ids.push(bldg_id);
                    }
                }
                BuildingStatus::Abandoned => {
                    abandoned_ids.push(bldg_id);
                    if bldg.is_fringe {
                        fringe_abandoned_ids.push(bldg_id);
                    }
                }
            }
        }

        let active_count = active_ids.len() as u32;

        if active_count < target {
            // ── Growing city: spawn new buildings or reactivate abandoned ones ──
            let deficit = target - active_count;

            // First, reactivate abandoned fringe buildings (cheapest recovery)
            let mut reactivated = 0u32;
            for &bldg_id in &fringe_abandoned_ids {
                if reactivated >= deficit {
                    break;
                }
                if let Some(bldg) = world.building_footprints.get_mut(&bldg_id) {
                    bldg.status = BuildingStatus::Active;
                    reactivated += 1;
                }
            }

            // Then reactivate non-fringe abandoned buildings
            if reactivated < deficit {
                for &bldg_id in &abandoned_ids {
                    if reactivated >= deficit {
                        break;
                    }
                    // Skip fringe — already handled above
                    if fringe_abandoned_ids.contains(&bldg_id) {
                        continue;
                    }
                    if let Some(bldg) = world.building_footprints.get_mut(&bldg_id) {
                        bldg.status = BuildingStatus::Active;
                        reactivated += 1;
                    }
                }
            }

            // If still deficit, spawn new buildings in fringe cells
            let still_needed = deficit - reactivated;
            if still_needed > 0 && !cells.is_empty() {
                let base_demand = if target > 0 {
                    telecom_demand / target as f64
                } else {
                    0.0
                };
                let fringe_demand = base_demand * 0.6;

                // Pick fringe cells: last third of city cells (outermost ring)
                // Ensure we have at least one cell even for very small cities
                let fringe_size = (cells.len() / 3).max(1);
                let fringe_start = cells.len().saturating_sub(fringe_size);
                let fringe_cells = &cells[fringe_start..];

                // Distribute new buildings across fringe cells deterministically
                // Cap spawns per tick to 50 to avoid large spikes
                let spawn_count = still_needed.min(50);
                for i in 0..spawn_count {
                    let cell_idx = fringe_cells[i as usize % fringe_cells.len()];
                    let id = world.allocate_entity();
                    let bldg = BuildingFootprint::new(city_id, cell_idx, fringe_demand, true);
                    world.building_footprints.insert(id, bldg);
                }
            }
        } else if active_count > target {
            // ── Declining city: abandon fringe buildings first ──
            let surplus = active_count - target;

            // Abandon fringe buildings first (outermost ring decays first)
            let mut abandoned = 0u32;

            // Process fringe active buildings in reverse order (highest ID = newest)
            for &bldg_id in fringe_active_ids.iter().rev() {
                if abandoned >= surplus {
                    break;
                }
                if let Some(bldg) = world.building_footprints.get_mut(&bldg_id) {
                    bldg.status = BuildingStatus::Abandoned;
                    abandoned += 1;
                }
            }

            // If more need abandoning, abandon non-fringe buildings (also reverse order)
            if abandoned < surplus {
                for &bldg_id in active_ids.iter().rev() {
                    if abandoned >= surplus {
                        break;
                    }
                    // Skip fringe — already handled
                    if fringe_active_ids.contains(&bldg_id) {
                        continue;
                    }
                    if let Some(bldg) = world.building_footprints.get_mut(&bldg_id) {
                        bldg.status = BuildingStatus::Abandoned;
                        abandoned += 1;
                    }
                }
            }
        }

        // Update census
        if let Some(census) = world.city_building_census.get_mut(&city_id) {
            // Recount after modifications
            let mut new_active = 0u32;
            let mut new_abandoned = 0u32;
            for bldg in world.building_footprints.values() {
                if bldg.city_id == city_id {
                    match bldg.status {
                        BuildingStatus::Active => new_active += 1,
                        BuildingStatus::Abandoned => new_abandoned += 1,
                    }
                }
            }
            census.active_count = new_active;
            census.abandoned_count = new_abandoned;
            census.target_count = target;
            census.prev_population = population;
        }
    }
}

/// Generate a deterministic settlement name from tick and cell index.
fn generate_settlement_name(tick: u64, cell_index: usize) -> String {
    let prefixes = [
        "New", "Fort", "Port", "Lake", "Bay", "North", "South", "East", "West", "Old",
        "Iron", "Silver", "Red", "Blue", "Green", "High", "Low", "Stone", "River", "Pine",
    ];
    let roots = [
        "haven", "bridge", "field", "dale", "wood", "gate", "holm", "wick", "stead", "cliff",
        "ridge", "brook", "creek", "springs", "falls", "mill", "cross", "well", "bury", "vale",
        "moor", "worth", "thorpe", "bourne", "crest", "peak", "mouth", "ford", "ton", "ville",
    ];

    let mut hash: u64 = tick.wrapping_mul(7919).wrapping_add(cell_index as u64);
    hash = hash
        .wrapping_mul(6364136223846793005u64)
        .wrapping_add(1442695040888963407u64);
    let p = prefixes[(hash % prefixes.len() as u64) as usize];
    hash = hash
        .wrapping_mul(6364136223846793005u64)
        .wrapping_add(1442695040888963407u64);
    let r = roots[(hash % roots.len() as u64) as usize];

    let mut chars = r.chars();
    let capitalized = match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    };

    format!("{} {}", p, capitalized)
}
