use gt_common::types::WorldConfig;
use serde::{Deserialize, Serialize};

/// Preset world configurations for quick game setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorldPreset {
    /// Use real Earth geography (existing real_earth module).
    RealEarth,
    /// Single supercontinent with inland seas.
    Pangaea,
    /// Many small islands and island chains.
    Archipelago,
    /// Balanced multi-continent world (default procgen).
    Continents,
    /// Fully randomized parameters.
    Random,
}

impl WorldPreset {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            WorldPreset::RealEarth => "Real Earth",
            WorldPreset::Pangaea => "Pangaea",
            WorldPreset::Archipelago => "Archipelago",
            WorldPreset::Continents => "Continents",
            WorldPreset::Random => "Random",
        }
    }

    /// Short description for the UI.
    pub fn description(&self) -> &'static str {
        match self {
            WorldPreset::RealEarth => "Play on a map based on real Earth geography and data.",
            WorldPreset::Pangaea => "A single massive supercontinent surrounded by ocean.",
            WorldPreset::Archipelago => "Hundreds of islands scattered across vast oceans.",
            WorldPreset::Continents => "Multiple distinct continents with varied terrain.",
            WorldPreset::Random => "Every parameter randomized for a unique experience.",
        }
    }
}

/// Apply a world preset to a WorldConfig, overwriting generation parameters
/// while preserving gameplay settings (seed, era, difficulty, AI count, corp name).
pub fn apply_preset(config: &mut WorldConfig, preset: WorldPreset) {
    match preset {
        WorldPreset::RealEarth => {
            config.use_real_earth = true;
            // Other params don't matter for real earth
        }
        WorldPreset::Pangaea => {
            config.use_real_earth = false;
            config.continent_count = 1;
            config.ocean_percentage = 0.55;
            config.terrain_roughness = 0.6;
            config.climate_variation = 0.4;
            config.city_density = 0.6;
        }
        WorldPreset::Archipelago => {
            config.use_real_earth = false;
            config.continent_count = 8;
            config.ocean_percentage = 0.85;
            config.terrain_roughness = 0.7;
            config.climate_variation = 0.6;
            config.city_density = 0.3;
        }
        WorldPreset::Continents => {
            config.use_real_earth = false;
            config.continent_count = 4;
            config.ocean_percentage = 0.70;
            config.terrain_roughness = 0.5;
            config.climate_variation = 0.5;
            config.city_density = 0.5;
        }
        WorldPreset::Random => {
            config.use_real_earth = false;
            // Use the seed to derive random parameters deterministically
            let mut h = config.seed;
            h = splitmix(h);
            config.continent_count = ((h % 8) + 1) as u8;
            h = splitmix(h);
            config.ocean_percentage = 0.3 + (h % 601) as f64 / 1000.0; // 0.3-0.9
            h = splitmix(h);
            config.terrain_roughness = (h % 1001) as f64 / 1000.0;
            h = splitmix(h);
            config.climate_variation = (h % 1001) as f64 / 1000.0;
            h = splitmix(h);
            config.city_density = 0.2 + (h % 601) as f64 / 1000.0; // 0.2-0.8
        }
    }
}

/// All available presets in display order.
pub fn all_presets() -> &'static [WorldPreset] {
    &[
        WorldPreset::Continents,
        WorldPreset::Pangaea,
        WorldPreset::Archipelago,
        WorldPreset::RealEarth,
        WorldPreset::Random,
    ]
}

/// Simple splitmix64 hash for deriving deterministic values from seed.
fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gt_common::types::{MapSize, Era, DifficultyPreset};

    #[test]
    fn test_apply_preset_real_earth() {
        let mut config = WorldConfig::default();
        apply_preset(&mut config, WorldPreset::RealEarth);
        assert!(config.use_real_earth);
    }

    #[test]
    fn test_apply_preset_pangaea() {
        let mut config = WorldConfig::default();
        apply_preset(&mut config, WorldPreset::Pangaea);
        assert!(!config.use_real_earth);
        assert_eq!(config.continent_count, 1);
        assert!(config.ocean_percentage < 0.6);
    }

    #[test]
    fn test_apply_preset_archipelago() {
        let mut config = WorldConfig::default();
        apply_preset(&mut config, WorldPreset::Archipelago);
        assert!(config.ocean_percentage > 0.8);
        assert_eq!(config.continent_count, 8);
    }

    #[test]
    fn test_apply_preset_preserves_gameplay() {
        let mut config = WorldConfig {
            seed: 12345,
            starting_era: Era::Telegraph,
            difficulty: DifficultyPreset::Hard,
            map_size: MapSize::Huge,
            ai_corporations: 8,
            use_real_earth: true,
            corp_name: Some("TestCorp".to_string()),
            continent_count: 1,
            ocean_percentage: 0.3,
            terrain_roughness: 0.1,
            climate_variation: 0.1,
            city_density: 0.1,
        };
        apply_preset(&mut config, WorldPreset::Continents);

        // Gameplay settings preserved
        assert_eq!(config.seed, 12345);
        assert_eq!(config.starting_era, Era::Telegraph);
        assert_eq!(config.difficulty, DifficultyPreset::Hard);
        assert_eq!(config.map_size, MapSize::Huge);
        assert_eq!(config.ai_corporations, 8);
        assert_eq!(config.corp_name, Some("TestCorp".to_string()));

        // Generation params changed
        assert!(!config.use_real_earth);
        assert_eq!(config.continent_count, 4);
    }

    #[test]
    fn test_random_preset_deterministic() {
        let mut c1 = WorldConfig { seed: 42, ..WorldConfig::default() };
        let mut c2 = WorldConfig { seed: 42, ..WorldConfig::default() };
        apply_preset(&mut c1, WorldPreset::Random);
        apply_preset(&mut c2, WorldPreset::Random);
        assert_eq!(c1.continent_count, c2.continent_count);
        assert_eq!(c1.ocean_percentage, c2.ocean_percentage);
        assert_eq!(c1.terrain_roughness, c2.terrain_roughness);
    }

    #[test]
    fn test_all_presets_list() {
        let presets = all_presets();
        assert_eq!(presets.len(), 5);
        for p in presets {
            assert!(!p.display_name().is_empty());
            assert!(!p.description().is_empty());
        }
    }
}
