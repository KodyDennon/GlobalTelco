use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // Phase 1: Update city telecom demand based on population and development
    // Use sqrt scaling so demand grows sub-linearly with population.
    // This keeps infrastructure capacity (hundreds) in the same ballpark as demand.
    // A 100K city → ~630 demand; a 10M city → ~6300 demand.
    let mut city_data: Vec<(u64, f64, Vec<usize>)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let demand = (city.population as f64).sqrt() * city.development * 2.0;
            (id, demand, city.cells.clone())
        })
        .collect();
    city_data.sort_unstable_by_key(|t| t.0);

    for (city_id, demand, _) in &city_data {
        if let Some(city) = world.cities.get_mut(city_id) {
            city.telecom_demand = *demand;
        }
        if let Some(d) = world.demands.get_mut(city_id) {
            d.base_demand = *demand;
            d.current_demand = *demand;
        }
    }

    // Phase 2: Calculate per-city satisfaction from per-cell coverage
    // Each city's cells have population; coverage on those cells determines satisfaction
    for (city_id, demand, cells) in &city_data {
        if cells.is_empty() {
            continue;
        }

        // Zero demand = fully satisfied (no unmet need)
        if *demand <= 0.0 {
            if let Some(city) = world.cities.get_mut(city_id) {
                city.infrastructure_satisfaction = 1.0;
            }
            if let Some(d) = world.demands.get_mut(city_id) {
                d.satisfaction = 1.0;
            }
            continue;
        }

        let cell_count = cells.len() as f64;
        let demand_per_cell = demand / cell_count;

        let mut total_satisfaction = 0.0;
        let mut covered_cells = 0u32;

        for &ci in cells {
            let coverage = world.cell_coverage.get(&ci);
            let cell_bandwidth = coverage.map(|c| c.bandwidth).unwrap_or(0.0);

            // Satisfaction for this cell: how much of its demand is met by coverage
            let cell_sat = if demand_per_cell > 0.0 {
                (cell_bandwidth / demand_per_cell).clamp(0.0, 1.0)
            } else {
                0.0
            };

            total_satisfaction += cell_sat;
            if cell_bandwidth > 0.0 {
                covered_cells += 1;
            }
        }

        // City satisfaction = average across all its cells
        // Bonus for having ALL cells covered (network completeness)
        let avg_satisfaction = total_satisfaction / cell_count;
        let coverage_ratio = covered_cells as f64 / cell_count;
        // Blend: 70% raw satisfaction + 30% coverage completeness
        let final_satisfaction = (avg_satisfaction * 0.7 + coverage_ratio * 0.3).clamp(0.0, 1.0);

        if let Some(city) = world.cities.get_mut(city_id) {
            city.infrastructure_satisfaction = final_satisfaction;
        }
        if let Some(d) = world.demands.get_mut(city_id) {
            d.satisfaction = final_satisfaction;
        }
    }

    // Phase 3: Aggregate to region level
    let mut region_ids: Vec<u64> = world.regions.keys().copied().collect();
    region_ids.sort_unstable();

    for region_id in region_ids {
        let city_ids = world
            .regions
            .get(&region_id)
            .map(|r| r.city_ids.clone())
            .unwrap_or_default();

        let total_demand: f64 = city_ids
            .iter()
            .filter_map(|&cid| world.demands.get(&cid).map(|d| d.current_demand))
            .sum();

        // Region satisfaction = population-weighted average of city satisfactions
        let mut weighted_sat = 0.0;
        let mut total_pop = 0u64;
        for &cid in &city_ids {
            if let Some(city) = world.cities.get(&cid) {
                weighted_sat += city.infrastructure_satisfaction * city.population as f64;
                total_pop += city.population;
            }
        }
        let region_satisfaction = if total_pop > 0 {
            (weighted_sat / total_pop as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        if let Some(d) = world.demands.get_mut(&region_id) {
            d.base_demand = total_demand;
            d.current_demand = total_demand;
            d.satisfaction = region_satisfaction;
        }
    }
}
