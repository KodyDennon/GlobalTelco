use serde::{Deserialize, Serialize};

use super::Money;

// ── Phase 8: Spectrum & Frequency Management ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrequencyBand {
    // Low band (wide coverage, lower speed)
    Band700MHz,
    Band800MHz,
    Band900MHz,
    // Mid band (balanced)
    Band1800MHz,
    Band2100MHz,
    Band2600MHz,
    // High band (short range, highest speed)
    Band3500MHz,
    Band28GHz,  // mmWave
    Band39GHz,  // mmWave
    // Satellite bands
    BandKu,     // 12-18 GHz (satellite TV, broadband)
    BandKa,     // 26.5-40 GHz (high-throughput satellite)
    BandV,      // 40-75 GHz (next-gen satellite)
    BandQ,      // 33-50 GHz (satellite feeder links)
}

impl FrequencyBand {
    /// Coverage radius in km for this frequency band.
    pub fn coverage_radius_km(&self) -> f64 {
        match self {
            FrequencyBand::Band700MHz => 30.0,
            FrequencyBand::Band800MHz => 25.0,
            FrequencyBand::Band900MHz => 20.0,
            FrequencyBand::Band1800MHz => 10.0,
            FrequencyBand::Band2100MHz => 8.0,
            FrequencyBand::Band2600MHz => 5.0,
            FrequencyBand::Band3500MHz => 2.0,
            FrequencyBand::Band28GHz => 0.5,
            FrequencyBand::Band39GHz => 0.3,
            // Satellite bands — coverage handled by orbital mechanics, not terrestrial
            FrequencyBand::BandKu => 0.0,
            FrequencyBand::BandKa => 0.0,
            FrequencyBand::BandV => 0.0,
            FrequencyBand::BandQ => 0.0,
        }
    }

    /// Maximum bandwidth available in this band (MHz).
    pub fn max_bandwidth_mhz(&self) -> f64 {
        match self {
            FrequencyBand::Band700MHz => 45.0,
            FrequencyBand::Band800MHz => 30.0,
            FrequencyBand::Band900MHz => 35.0,
            FrequencyBand::Band1800MHz => 75.0,
            FrequencyBand::Band2100MHz => 60.0,
            FrequencyBand::Band2600MHz => 70.0,
            FrequencyBand::Band3500MHz => 100.0,
            FrequencyBand::Band28GHz => 800.0,
            FrequencyBand::Band39GHz => 1000.0,
            FrequencyBand::BandKu => 500.0,
            FrequencyBand::BandKa => 1500.0,
            FrequencyBand::BandV => 3000.0,
            FrequencyBand::BandQ => 2000.0,
        }
    }

    /// Base auction price per MHz of spectrum in this band.
    pub fn cost_per_mhz(&self) -> Money {
        match self {
            FrequencyBand::Band700MHz => 500_000,
            FrequencyBand::Band800MHz => 400_000,
            FrequencyBand::Band900MHz => 350_000,
            FrequencyBand::Band1800MHz => 250_000,
            FrequencyBand::Band2100MHz => 200_000,
            FrequencyBand::Band2600MHz => 150_000,
            FrequencyBand::Band3500MHz => 100_000,
            FrequencyBand::Band28GHz => 50_000,
            FrequencyBand::Band39GHz => 30_000,
            FrequencyBand::BandKu => 80_000,
            FrequencyBand::BandKa => 60_000,
            FrequencyBand::BandV => 40_000,
            FrequencyBand::BandQ => 50_000,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            FrequencyBand::Band700MHz => "700 MHz",
            FrequencyBand::Band800MHz => "800 MHz",
            FrequencyBand::Band900MHz => "900 MHz",
            FrequencyBand::Band1800MHz => "1800 MHz",
            FrequencyBand::Band2100MHz => "2100 MHz",
            FrequencyBand::Band2600MHz => "2600 MHz",
            FrequencyBand::Band3500MHz => "3.5 GHz",
            FrequencyBand::Band28GHz => "28 GHz (mmWave)",
            FrequencyBand::Band39GHz => "39 GHz (mmWave)",
            FrequencyBand::BandKu => "Ku Band (Satellite)",
            FrequencyBand::BandKa => "Ka Band (Satellite)",
            FrequencyBand::BandV => "V Band (Satellite)",
            FrequencyBand::BandQ => "Q Band (Satellite)",
        }
    }

    /// Band category for UI color coding: "low", "mid", "high", or "satellite".
    pub fn category(&self) -> &'static str {
        match self {
            FrequencyBand::Band700MHz
            | FrequencyBand::Band800MHz
            | FrequencyBand::Band900MHz => "low",
            FrequencyBand::Band1800MHz
            | FrequencyBand::Band2100MHz
            | FrequencyBand::Band2600MHz => "mid",
            FrequencyBand::Band3500MHz
            | FrequencyBand::Band28GHz
            | FrequencyBand::Band39GHz => "high",
            FrequencyBand::BandKu
            | FrequencyBand::BandKa
            | FrequencyBand::BandV
            | FrequencyBand::BandQ => "satellite",
        }
    }

    /// All frequency band variants for iteration.
    pub fn all() -> &'static [FrequencyBand] {
        &[
            FrequencyBand::Band700MHz,
            FrequencyBand::Band800MHz,
            FrequencyBand::Band900MHz,
            FrequencyBand::Band1800MHz,
            FrequencyBand::Band2100MHz,
            FrequencyBand::Band2600MHz,
            FrequencyBand::Band3500MHz,
            FrequencyBand::Band28GHz,
            FrequencyBand::Band39GHz,
            FrequencyBand::BandKu,
            FrequencyBand::BandKa,
            FrequencyBand::BandV,
            FrequencyBand::BandQ,
        ]
    }

    /// Parse from string name (e.g. "Band700MHz").
    pub fn from_name(name: &str) -> Option<FrequencyBand> {
        match name {
            "Band700MHz" => Some(FrequencyBand::Band700MHz),
            "Band800MHz" => Some(FrequencyBand::Band800MHz),
            "Band900MHz" => Some(FrequencyBand::Band900MHz),
            "Band1800MHz" => Some(FrequencyBand::Band1800MHz),
            "Band2100MHz" => Some(FrequencyBand::Band2100MHz),
            "Band2600MHz" => Some(FrequencyBand::Band2600MHz),
            "Band3500MHz" => Some(FrequencyBand::Band3500MHz),
            "Band28GHz" => Some(FrequencyBand::Band28GHz),
            "Band39GHz" => Some(FrequencyBand::Band39GHz),
            "BandKu" => Some(FrequencyBand::BandKu),
            "BandKa" => Some(FrequencyBand::BandKa),
            "BandV" => Some(FrequencyBand::BandV),
            "BandQ" => Some(FrequencyBand::BandQ),
            _ => None,
        }
    }
}
