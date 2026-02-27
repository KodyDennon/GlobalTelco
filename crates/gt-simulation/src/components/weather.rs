use gt_common::types::{EntityId, Tick};
use serde::{Deserialize, Serialize};

/// Types of weather events that affect infrastructure differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherType {
	/// No significant weather — baseline conditions.
	Clear,
	/// Severe thunderstorms with high winds and lightning.
	Storms,
	/// Freezing rain and ice accumulation on infrastructure.
	IceStorm,
	/// River/coastal flooding from sustained precipitation.
	Flooding,
	/// Prolonged extreme temperatures causing equipment stress.
	ExtremeHeat,
	/// Seismic activity (not weather per se, but same vulnerability system).
	Earthquake,
	/// Sustained tropical cyclone with destructive winds.
	Hurricane,
}

impl WeatherType {
	/// Human-readable display name.
	pub fn display_name(&self) -> &'static str {
		match self {
			WeatherType::Clear => "Clear",
			WeatherType::Storms => "Storms",
			WeatherType::IceStorm => "Ice Storm",
			WeatherType::Flooding => "Flooding",
			WeatherType::ExtremeHeat => "Extreme Heat",
			WeatherType::Earthquake => "Earthquake",
			WeatherType::Hurricane => "Hurricane",
		}
	}

	/// Base damage multiplier for this weather type (applied to infrastructure in affected cells).
	/// Higher values mean more damaging weather.
	pub fn base_severity_multiplier(&self) -> f64 {
		match self {
			WeatherType::Clear => 0.0,
			WeatherType::Storms => 0.6,
			WeatherType::IceStorm => 0.7,
			WeatherType::Flooding => 0.8,
			WeatherType::ExtremeHeat => 0.3,
			WeatherType::Earthquake => 1.0,
			WeatherType::Hurricane => 0.9,
		}
	}

	/// Terrain-based probability weight: how likely this weather type is for a given terrain.
	/// Returns a 0.0-1.0 weight factor (not a raw probability).
	pub fn terrain_affinity(&self, terrain: &gt_common::types::TerrainType) -> f64 {
		use gt_common::types::TerrainType;
		match self {
			WeatherType::Clear => 1.0, // always possible
			WeatherType::Storms => match terrain {
				TerrainType::Coastal => 1.5,
				TerrainType::Rural | TerrainType::Suburban => 1.2,
				TerrainType::Urban => 1.0,
				TerrainType::Mountainous => 0.8,
				TerrainType::Desert => 0.3,
				TerrainType::Tundra | TerrainType::Frozen => 0.4,
				TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => 1.8,
			},
			WeatherType::IceStorm => match terrain {
				TerrainType::Tundra | TerrainType::Frozen => 2.0,
				TerrainType::Mountainous => 1.5,
				TerrainType::Rural | TerrainType::Suburban => 0.8,
				TerrainType::Urban => 0.6,
				TerrainType::Desert => 0.0,
				TerrainType::Coastal => 0.5,
				TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => 0.3,
			},
			WeatherType::Flooding => match terrain {
				TerrainType::Coastal => 2.0,
				TerrainType::Rural => 1.2,
				TerrainType::Urban | TerrainType::Suburban => 1.0,
				TerrainType::Mountainous => 0.5,
				TerrainType::Desert => 0.2,
				TerrainType::Tundra => 0.3,
				TerrainType::Frozen => 0.1,
				TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => 0.1,
			},
			WeatherType::ExtremeHeat => match terrain {
				TerrainType::Desert => 2.0,
				TerrainType::Urban => 1.5,
				TerrainType::Suburban | TerrainType::Rural => 1.0,
				TerrainType::Coastal => 0.7,
				TerrainType::Mountainous => 0.3,
				TerrainType::Tundra | TerrainType::Frozen => 0.0,
				TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => 0.1,
			},
			WeatherType::Earthquake => match terrain {
				TerrainType::Mountainous => 1.8,
				TerrainType::Coastal => 1.3,
				TerrainType::Urban | TerrainType::Suburban => 1.0,
				TerrainType::Rural => 0.8,
				TerrainType::Desert => 0.6,
				TerrainType::OceanShallow | TerrainType::OceanDeep => 1.2,
				TerrainType::OceanTrench => 1.8, // trenches are seismically active
				TerrainType::Tundra | TerrainType::Frozen => 0.5,
			},
			WeatherType::Hurricane => match terrain {
				TerrainType::Coastal => 2.5,
				TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => 2.0,
				TerrainType::Rural | TerrainType::Suburban => 0.5,
				TerrainType::Urban => 0.4,
				TerrainType::Mountainous => 0.1,
				TerrainType::Desert => 0.1,
				TerrainType::Tundra | TerrainType::Frozen => 0.0,
			},
		}
	}
}

/// Active weather condition affecting a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherCondition {
	/// Region entity where this weather is occurring.
	pub region_id: EntityId,
	/// Type of weather event.
	pub condition: WeatherType,
	/// Intensity of the weather (0.0-1.0). Higher = more damaging.
	pub severity: f64,
	/// How many ticks this weather event lasts.
	pub duration_ticks: u32,
	/// The tick when this weather event started.
	pub started_tick: Tick,
}

impl WeatherCondition {
	/// Whether this weather condition has expired.
	pub fn is_expired(&self, current_tick: Tick) -> bool {
		current_tick >= self.started_tick + self.duration_ticks as u64
	}

	/// Remaining ticks until this weather condition expires.
	pub fn remaining_ticks(&self, current_tick: Tick) -> u32 {
		let end_tick = self.started_tick + self.duration_ticks as u64;
		if current_tick >= end_tick {
			0
		} else {
			(end_tick - current_tick) as u32
		}
	}
}

/// A predicted weather event generated by the forecast system.
/// Forecasts are generated 5-10 tick-windows ahead of actual weather events
/// using the deterministic RNG to peek into the future.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherForecast {
	/// Region entity where the weather is predicted.
	pub region_id: EntityId,
	/// Human-readable region name.
	pub region_name: String,
	/// Predicted weather type.
	pub predicted_type: WeatherType,
	/// Probability of this forecast materializing (0.0-1.0).
	pub probability: f64,
	/// Estimated ticks until the weather event arrives.
	pub eta_ticks: u32,
	/// Predicted severity if the event materializes (0.0-1.0).
	pub predicted_severity: f64,
}
