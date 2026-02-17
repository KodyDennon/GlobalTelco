use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // Update city populations with growth/decline based on infrastructure satisfaction
    let mut city_updates: Vec<(u64, u64, f64)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let satisfaction = city.infrastructure_satisfaction;
            // Growth is boosted by infra satisfaction, penalized by overcrowding
            let adjusted_rate = city.growth_rate * (0.5 + satisfaction * 0.5);
            (id, city.population, adjusted_rate)
        })
        .collect();
    city_updates.sort_unstable_by_key(|t| t.0);

    for (city_id, pop, rate) in city_updates {
        let growth = (pop as f64 * rate) as i64;
        if let Some(city) = world.cities.get_mut(&city_id) {
            city.population = (city.population as i64 + growth).max(100) as u64;
        }
        // Also update the population component if one exists for this city
        if let Some(pop_comp) = world.populations.get_mut(&city_id) {
            let growth = (pop_comp.count as f64 * pop_comp.growth_rate) as i64;
            pop_comp.count = (pop_comp.count as i64 + growth).max(100) as u64;
        }
    }

    // Update region populations from city totals
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

    // Migration: people move from low-satisfaction to high-satisfaction cities
    // Collect data first, then apply
    let mut city_scores: Vec<(u64, f64, u64)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let infra_quality = city.infrastructure_satisfaction;
            let development = city.development;
            let score = infra_quality * 0.5 + development * 0.5;
            (id, score, city.population)
        })
        .collect();
    city_scores.sort_unstable_by_key(|t| t.0);

    if city_scores.len() >= 2 {
        let avg_score: f64 =
            city_scores.iter().map(|(_, s, _)| s).sum::<f64>() / city_scores.len() as f64;

        for &(city_id, score, pop) in &city_scores {
            if pop < 1000 {
                continue;
            }
            let diff = score - avg_score;
            // Migration: 0.1% of population per tick, scaled by score difference
            let migrants = (pop as f64 * 0.001 * diff) as i64;
            if let Some(city) = world.cities.get_mut(&city_id) {
                city.population = (city.population as i64 + migrants).max(100) as u64;
            }
        }
    }
}
