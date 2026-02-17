use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // 1. Calculate jobs per city from infrastructure
    update_employment(world);

    // 2. Apply birth/death rates to city populations
    apply_demographics(world);

    // 3. Migration: people move from low-attractiveness to high-attractiveness cities
    apply_migration(world);

    // 4. Update region populations from city totals
    aggregate_region_populations(world);
}

/// Count infrastructure jobs per city and update employment rates.
fn update_employment(world: &mut GameWorld) {
    // Count jobs per cell from operational (non-construction) infrastructure nodes
    let mut jobs_per_cell: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
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

        // Migration: 0.1% of population per tick, scaled by score difference
        let migrants = (pop as f64 * 0.001 * diff) as i64;

        if let Some(city) = world.cities.get_mut(&city_id) {
            city.population = (city.population as i64 + migrants).max(100) as u64;
            city.migration_pressure = pressure;
        }
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
