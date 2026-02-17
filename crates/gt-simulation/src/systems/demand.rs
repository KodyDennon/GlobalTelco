use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // Update city telecom demand based on population and development
    let mut city_data: Vec<(u64, f64)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let demand = city.population as f64 * city.development * 0.001;
            (id, demand)
        })
        .collect();
    city_data.sort_unstable_by_key(|t| t.0);

    for (city_id, demand) in &city_data {
        if let Some(city) = world.cities.get_mut(city_id) {
            city.telecom_demand = *demand;
        }
        if let Some(d) = world.demands.get_mut(city_id) {
            d.base_demand = *demand;
            d.current_demand = *demand;
        }
    }

    // Update region demands from city totals
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

        // Calculate how much infrastructure capacity exists in this region
        let region_cells: Vec<usize> = world
            .regions
            .get(&region_id)
            .map(|r| r.cells.clone())
            .unwrap_or_default();

        let region_capacity: f64 = world
            .infra_nodes
            .values()
            .filter(|n| region_cells.contains(&n.cell_index))
            .filter_map(|n| {
                // Only count non-construction nodes
                let node_ids: Vec<u64> = world
                    .infra_nodes
                    .iter()
                    .filter(|(_, v)| v.cell_index == n.cell_index)
                    .map(|(&k, _)| k)
                    .collect();
                let any_under_construction = node_ids
                    .iter()
                    .any(|id| world.constructions.contains_key(id));
                if any_under_construction {
                    None
                } else {
                    Some(n.max_throughput)
                }
            })
            .sum();

        let satisfaction = if total_demand > 0.0 {
            (region_capacity / total_demand).clamp(0.0, 1.0)
        } else {
            0.0
        };

        if let Some(d) = world.demands.get_mut(&region_id) {
            d.base_demand = total_demand;
            d.current_demand = total_demand;
            d.satisfaction = satisfaction;
        }

        // Update city satisfaction from region
        for &cid in &city_ids {
            if let Some(city) = world.cities.get_mut(&cid) {
                city.infrastructure_satisfaction = satisfaction;
            }
            if let Some(d) = world.demands.get_mut(&cid) {
                d.satisfaction = satisfaction;
            }
        }
    }
}
