use serde::{Deserialize, Serialize};

use super::Money;

// ── Satellite System Enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrbitType {
    LEO,
    MEO,
    GEO,
    HEO,
}

impl OrbitType {
    /// Typical altitude range for this orbit type (min, max) in km.
    pub fn altitude_range_km(&self) -> (f64, f64) {
        match self {
            OrbitType::LEO => (160.0, 2000.0),
            OrbitType::MEO => (2000.0, 35786.0),
            OrbitType::GEO => (35786.0, 35786.0),
            OrbitType::HEO => (200.0, 40000.0),
        }
    }

    /// Typical latency one-way in milliseconds.
    pub fn latency_ms(&self) -> f64 {
        match self {
            OrbitType::LEO => 4.0,    // ~4ms one-way at 550km
            OrbitType::MEO => 60.0,   // ~60ms one-way at 8000km
            OrbitType::GEO => 240.0,  // ~240ms one-way at 35786km
            OrbitType::HEO => 100.0,  // variable
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            OrbitType::LEO => "Low Earth Orbit",
            OrbitType::MEO => "Medium Earth Orbit",
            OrbitType::GEO => "Geostationary Orbit",
            OrbitType::HEO => "Highly Elliptical Orbit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SatelliteStatus {
    Manufacturing,
    AwaitingLaunch,
    Operational,
    Decaying,
    Deorbiting,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RocketType {
    Small,
    Medium,
    Heavy,
    SuperHeavy,
}

impl RocketType {
    /// Maximum payload mass in kg.
    pub fn payload_capacity_kg(&self) -> f64 {
        match self {
            RocketType::Small => 500.0,
            RocketType::Medium => 5_000.0,
            RocketType::Heavy => 20_000.0,
            RocketType::SuperHeavy => 100_000.0,
        }
    }

    /// Base launch cost.
    pub fn launch_cost(&self) -> Money {
        match self {
            RocketType::Small => 10_000_000,
            RocketType::Medium => 50_000_000,
            RocketType::Heavy => 150_000_000,
            RocketType::SuperHeavy => 400_000_000,
        }
    }

    /// Base reliability (probability of success).
    pub fn reliability(&self) -> f64 {
        match self {
            RocketType::Small => 0.90,
            RocketType::Medium => 0.93,
            RocketType::Heavy => 0.95,
            RocketType::SuperHeavy => 0.97,
        }
    }

    /// Cooldown ticks between launches from same pad.
    pub fn cooldown_ticks(&self) -> u64 {
        match self {
            RocketType::Small => 10,
            RocketType::Medium => 20,
            RocketType::Heavy => 40,
            RocketType::SuperHeavy => 60,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RocketType::Small => "Small Rocket",
            RocketType::Medium => "Medium Rocket",
            RocketType::Heavy => "Heavy Rocket",
            RocketType::SuperHeavy => "Super Heavy Rocket",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactoryTier {
    SmallBatch,
    StandardProduction,
    MassProduction,
}

impl FactoryTier {
    /// Production rate: satellites per tick for satellite factories.
    pub fn satellite_production_rate(&self) -> f64 {
        match self {
            FactoryTier::SmallBatch => 0.05,        // 1 sat per 20 ticks
            FactoryTier::StandardProduction => 0.2,  // 1 sat per 5 ticks
            FactoryTier::MassProduction => 1.0,      // 1 sat per tick
        }
    }

    /// Production rate: terminals per tick for terminal factories.
    pub fn terminal_production_rate(&self) -> u32 {
        match self {
            FactoryTier::SmallBatch => 10,
            FactoryTier::StandardProduction => 50,
            FactoryTier::MassProduction => 200,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            FactoryTier::SmallBatch => "Small Batch",
            FactoryTier::StandardProduction => "Standard Production",
            FactoryTier::MassProduction => "Mass Production",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Refuel,
    Repair,
}
