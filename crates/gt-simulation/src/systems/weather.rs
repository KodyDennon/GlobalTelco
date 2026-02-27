//! Weather system: regional weather patterns and forecasting.
//!
//! Generates weather conditions based on terrain, latitude, and seasonal
//! patterns. Weather conditions affect infrastructure damage multipliers
//! when disasters occur (see disaster.rs deployment vulnerability matrix).
//!
//! Weather events are generated 5-10 ticks ahead of actual effects,
//! giving players time to reinforce or reroute infrastructure.

use crate::components::weather::{WeatherCondition, WeatherType};
use crate::world::GameWorld;

/// Weather types with associated selection weights for the weighted random pick.
const WEATHER_TYPES: &[(WeatherType, f64)] = &[
	(WeatherType::Clear, 0.0),       // never randomly picked — it's the default
	(WeatherType::Storms, 0.30),
	(WeatherType::IceStorm, 0.15),
	(WeatherType::Flooding, 0.20),
	(WeatherType::ExtremeHeat, 0.15),
	(WeatherType::Earthquake, 0.10),
	(WeatherType::Hurricane, 0.10),
];

/// Run the weather system each tick.
///
/// Weather events are checked every 25 ticks (half the disaster frequency).
/// Active weather conditions are tracked in `world.weather_conditions` and
/// expired entries are pruned each run.
pub fn run(world: &mut GameWorld) {
	let tick = world.current_tick();

	// Prune expired weather conditions
	world
		.weather_conditions
		.retain(|wc| !wc.is_expired(tick));

	// Only generate new weather every 25 ticks
	if !tick.is_multiple_of(25) {
		return;
	}

	// Collect region data sorted by ID for deterministic iteration
	let region_data: Vec<(u64, f64, Vec<usize>)> = {
		let mut v: Vec<_> = world
			.regions
			.iter()
			.map(|(&id, r)| (id, r.disaster_risk, r.cells.clone()))
			.collect();
		v.sort_unstable_by_key(|t| t.0);
		v
	};

	for (region_id, disaster_risk, cells) in region_data {
		// Skip regions that already have active weather
		let has_active = world
			.weather_conditions
			.iter()
			.any(|wc| wc.region_id == region_id && !wc.is_expired(tick));
		if has_active {
			continue;
		}

		// Weather generation chance: 5% base * disaster_risk factor
		let roll = world.deterministic_random();
		let threshold = 0.05 * disaster_risk;
		if roll >= threshold {
			continue;
		}

		// Determine dominant terrain for weather type selection
		let dominant_terrain = dominant_terrain_for_region(world, &cells);

		// Pick a weather type weighted by terrain affinity
		let weather_type = pick_weather_type(world, &dominant_terrain);

		// Skip clear weather (shouldn't happen, but safety)
		if matches!(weather_type, WeatherType::Clear) {
			continue;
		}

		// Severity: 0.1-0.6 based on terrain affinity and RNG
		let sev_roll = world.deterministic_random();
		let affinity = weather_type.terrain_affinity(&dominant_terrain);
		let severity = (sev_roll * 0.5 + 0.1) * (0.5 + affinity * 0.5);
		let severity = severity.clamp(0.1, 0.8);

		// Duration: 15-40 ticks
		let dur_roll = world.deterministic_random();
		let duration_ticks = 15 + (dur_roll * 25.0) as u32;

		world.weather_conditions.push(WeatherCondition {
			region_id,
			condition: weather_type,
			severity,
			duration_ticks,
			started_tick: tick,
		});

		// Emit a weather event so the frontend can react
		world.event_queue.push(
			tick,
			gt_common::events::GameEvent::WeatherStarted {
				region: region_id,
				weather_type: weather_type.display_name().to_string(),
				severity,
				duration_ticks,
			},
		);
	}
}

/// Pick a weather type using terrain-weighted random selection.
fn pick_weather_type(
	world: &mut GameWorld,
	terrain: &gt_common::types::TerrainType,
) -> WeatherType {
	let type_roll = world.deterministic_random();

	// Build weighted list based on terrain affinity
	let weights: Vec<(WeatherType, f64)> = WEATHER_TYPES
		.iter()
		.filter(|(wt, _)| !matches!(wt, WeatherType::Clear))
		.map(|&(wt, base_weight)| {
			let affinity = wt.terrain_affinity(terrain);
			(wt, base_weight * affinity)
		})
		.collect();

	// Normalize weights
	let total: f64 = weights.iter().map(|(_, w)| w).sum();
	if total <= 0.0 {
		return WeatherType::Storms; // fallback
	}

	let mut cumulative = 0.0;
	for (wt, weight) in &weights {
		cumulative += weight / total;
		if type_roll < cumulative {
			return *wt;
		}
	}

	weights.last().map(|(wt, _)| *wt).unwrap_or(WeatherType::Storms)
}

/// Determine the dominant terrain type for a set of cells.
fn dominant_terrain_for_region(
	world: &GameWorld,
	cells: &[usize],
) -> gt_common::types::TerrainType {
	use gt_common::types::TerrainType;
	use std::collections::HashMap;

	let mut counts: HashMap<u8, u32> = HashMap::new();
	for &cell_idx in cells {
		let terrain = world.get_cell_terrain(cell_idx).unwrap_or(TerrainType::Rural);
		*counts.entry(terrain as u8).or_insert(0) += 1;
	}

	let best = counts.into_iter().max_by_key(|&(_, c)| c).map(|(t, _)| t);
	match best {
		Some(0) => TerrainType::Urban,
		Some(1) => TerrainType::Suburban,
		Some(2) => TerrainType::Rural,
		Some(3) => TerrainType::Mountainous,
		Some(4) => TerrainType::Desert,
		Some(5) => TerrainType::Coastal,
		Some(6) => TerrainType::OceanShallow,
		Some(7) => TerrainType::OceanDeep,
		Some(8) => TerrainType::Tundra,
		Some(9) => TerrainType::Frozen,
		_ => TerrainType::Rural,
	}
}
