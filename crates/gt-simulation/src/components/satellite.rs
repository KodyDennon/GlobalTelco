use gt_common::types::{EntityId, OrbitType, SatelliteStatus, Tick};
use serde::{Deserialize, Serialize};

/// Core satellite component — attached to satellite InfraNode entities.
/// Orbital parameters define position each tick via Keplerian mechanics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Satellite {
    pub orbit_type: OrbitType,
    /// Current altitude in km (decreases when decaying).
    pub altitude_km: f64,
    /// Target altitude for station-keeping.
    pub base_altitude_km: f64,
    /// Orbital inclination in degrees (0 = equatorial, 90 = polar).
    pub inclination_deg: f64,
    /// Right ascension of ascending node in degrees (0-360).
    pub ascending_node_deg: f64,
    /// Mean anomaly in degrees (0-360), advances each tick.
    pub mean_anomaly_deg: f64,
    /// Fuel remaining (0.0-1.0 fraction of capacity).
    pub fuel_remaining: f64,
    /// Fuel capacity in arbitrary units.
    pub fuel_capacity: f64,
    /// Fuel consumed per tick for station-keeping (fraction of capacity).
    pub station_keeping_rate: f64,
    /// Current operational status.
    pub status: SatelliteStatus,
    /// Constellation this satellite belongs to (0 if independent).
    pub constellation_id: EntityId,
    /// Satellite mass in kg (determines launch payload requirements).
    pub mass_kg: f64,
    /// Maximum inter-satellite links this satellite can maintain.
    pub max_isl_links: u32,
    /// Half-angle of coverage cone in degrees.
    pub coverage_cone_half_angle_deg: f64,
    /// Tick when the satellite became operational.
    pub launched_tick: Tick,
    /// Orbital plane index within constellation (0-based).
    pub plane_index: u32,
    /// Index within plane (0-based).
    pub index_in_plane: u32,
}

impl Satellite {
    /// Earth's gravitational parameter (GM) in km^3/s^2.
    const GM_KM3_S2: f64 = 398600.4418;
    /// Earth's radius in km.
    const EARTH_RADIUS_KM: f64 = 6371.0;

    /// Calculate orbital period in seconds.
    pub fn orbital_period_seconds(&self) -> f64 {
        let r = Self::EARTH_RADIUS_KM + self.altitude_km;
        2.0 * std::f64::consts::PI * (r * r * r / Self::GM_KM3_S2).sqrt()
    }

    /// Calculate mean motion in degrees per second.
    pub fn mean_motion_deg_per_sec(&self) -> f64 {
        360.0 / self.orbital_period_seconds()
    }

    /// Calculate sub-satellite point (longitude, latitude) from orbital elements.
    /// Uses simplified circular orbit model (no eccentricity).
    pub fn sub_satellite_point(&self, elapsed_seconds: f64) -> (f64, f64) {
        let incl = self.inclination_deg.to_radians();
        let raan = self.ascending_node_deg.to_radians();
        let ma = self.mean_anomaly_deg.to_radians();

        // For circular orbit, true anomaly = mean anomaly
        let u = ma; // argument of latitude = omega + true_anomaly (omega=0 for circular)

        // Latitude: sin(lat) = sin(incl) * sin(u)
        let lat = (incl.sin() * u.sin()).asin().to_degrees();

        // Longitude: measured from ascending node, corrected for Earth rotation
        let lon_from_raan = u.sin().atan2(u.cos() * incl.cos());

        // Earth rotation: ~0.00417807 deg/sec
        let earth_rotation_deg = elapsed_seconds * 0.00417807;
        let mut lon = (raan + lon_from_raan).to_degrees() - earth_rotation_deg;

        // Normalize to -180..180
        while lon > 180.0 {
            lon -= 360.0;
        }
        while lon < -180.0 {
            lon += 360.0;
        }

        (lon, lat)
    }

    /// Calculate the ground footprint radius in km based on altitude and minimum elevation angle.
    pub fn footprint_radius_km(&self, min_elevation_deg: f64) -> f64 {
        let r_e = Self::EARTH_RADIUS_KM;
        let h = self.altitude_km;
        let elev = min_elevation_deg.to_radians();

        // Nadir angle from satellite geometry
        let sin_rho = r_e / (r_e + h);
        let eta = (sin_rho * elev.cos()).asin();
        // Earth central angle
        let lambda = std::f64::consts::FRAC_PI_2 - elev - eta;

        // Arc length on Earth's surface
        lambda * r_e
    }
}

/// Constellation definition — groups satellites into an orbital pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constellation {
    pub name: String,
    pub owner: EntityId,
    pub orbit_type: OrbitType,
    pub target_altitude_km: f64,
    pub target_inclination_deg: f64,
    pub num_planes: u32,
    pub sats_per_plane: u32,
    /// All satellite entity IDs in this constellation.
    pub satellite_ids: Vec<EntityId>,
    /// Count of currently operational satellites.
    pub operational_count: u32,
}

/// Orbital shell — tracks debris and collision risk for an altitude band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitalShell {
    pub min_altitude_km: f64,
    pub max_altitude_km: f64,
    pub debris_count: u32,
    /// Base collision probability per satellite per tick.
    pub collision_probability: f64,
    /// Debris threshold above which Kessler cascade begins.
    pub kessler_threshold: u32,
    /// Whether cascade is actively growing debris.
    pub cascade_active: bool,
}

impl OrbitalShell {
    /// Standard orbital shells.
    pub fn standard_shells() -> Vec<OrbitalShell> {
        vec![
            OrbitalShell {
                min_altitude_km: 200.0,
                max_altitude_km: 600.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 5000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 600.0,
                max_altitude_km: 1200.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 8000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 1200.0,
                max_altitude_km: 2000.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 10000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 2000.0,
                max_altitude_km: 10000.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 3000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 10000.0,
                max_altitude_km: 35786.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 2000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 35786.0,
                max_altitude_km: 36000.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 1000,
                cascade_active: false,
            },
            OrbitalShell {
                min_altitude_km: 200.0,
                max_altitude_km: 40000.0,
                debris_count: 0,
                collision_probability: 0.0,
                kessler_threshold: 15000,
                cascade_active: false,
            },
        ]
    }

    /// Find which shell a given altitude belongs to.
    pub fn shell_index_for_altitude(shells: &[OrbitalShell], altitude_km: f64) -> Option<usize> {
        shells
            .iter()
            .position(|s| altitude_km >= s.min_altitude_km && altitude_km < s.max_altitude_km)
    }
}

/// Satellite factory component — produces satellites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteFactoryComponent {
    pub tier: gt_common::types::FactoryTier,
    pub production_progress: f64,
    /// Queue of (constellation_id, count_remaining).
    pub queue: Vec<(EntityId, u32)>,
    pub owner: EntityId,
}

/// Terminal factory component — produces customer terminals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalFactoryComponent {
    pub tier: gt_common::types::FactoryTier,
    pub production_progress: f64,
    /// Terminals produced and stored at the factory, ready to ship.
    pub produced_stored: u32,
    pub owner: EntityId,
    /// Optional production target. Factory stops when produced_stored >= target.
    /// None = unlimited production.
    #[serde(default)]
    pub production_target: Option<u32>,
}

/// Regional warehouse for terminal distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseComponent {
    pub region_id: EntityId,
    pub terminal_inventory: u32,
    /// Terminals distributed per tick to cities in region.
    pub distribution_rate: u32,
    pub owner: EntityId,
}

/// Launch pad for rocket launches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchPadComponent {
    pub owner: EntityId,
    /// Queue of (rocket_type_str, Vec<satellite_ids>).
    pub launch_queue: Vec<(String, Vec<EntityId>)>,
    /// Ticks remaining before next launch can occur.
    pub cooldown_remaining: u64,
    /// Whether reusable rocket research has been completed.
    pub reusable: bool,
}

/// Satellite subscription tracking per city per corporation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteSubscription {
    pub city_id: EntityId,
    pub corp_id: EntityId,
    pub subscribers: u32,
    pub terminals_deployed: u32,
    pub monthly_rate: i64,
}

/// Active service mission on a satellite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMission {
    pub satellite_id: EntityId,
    pub service_type: gt_common::types::ServiceType,
    pub ticks_remaining: u64,
    pub cost: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orbital_period() {
        let sat = Satellite {
            orbit_type: OrbitType::LEO,
            altitude_km: 550.0,
            base_altitude_km: 550.0,
            inclination_deg: 53.0,
            ascending_node_deg: 0.0,
            mean_anomaly_deg: 0.0,
            fuel_remaining: 1.0,
            fuel_capacity: 100.0,
            station_keeping_rate: 0.0001,
            status: SatelliteStatus::Operational,
            constellation_id: 0,
            mass_kg: 260.0,
            max_isl_links: 4,
            coverage_cone_half_angle_deg: 45.0,
            launched_tick: 0,
            plane_index: 0,
            index_in_plane: 0,
        };

        let period = sat.orbital_period_seconds();
        // ISS at ~400km has period ~92 min, 550km should be ~96 min
        assert!(period > 5600.0 && period < 6000.0, "Period was {}", period);
    }

    #[test]
    fn test_sub_satellite_point_equatorial() {
        let sat = Satellite {
            orbit_type: OrbitType::LEO,
            altitude_km: 550.0,
            base_altitude_km: 550.0,
            inclination_deg: 0.0, // equatorial
            ascending_node_deg: 0.0,
            mean_anomaly_deg: 0.0,
            fuel_remaining: 1.0,
            fuel_capacity: 100.0,
            station_keeping_rate: 0.0001,
            status: SatelliteStatus::Operational,
            constellation_id: 0,
            mass_kg: 260.0,
            max_isl_links: 4,
            coverage_cone_half_angle_deg: 45.0,
            launched_tick: 0,
            plane_index: 0,
            index_in_plane: 0,
        };

        let (lon, lat) = sat.sub_satellite_point(0.0);
        // Equatorial orbit at MA=0 should be near equator
        assert!(lat.abs() < 1.0, "Latitude should be near 0, was {}", lat);
        assert!(lon.abs() < 1.0, "Longitude should be near 0, was {}", lon);
    }

    #[test]
    fn test_footprint_radius() {
        let sat = Satellite {
            orbit_type: OrbitType::LEO,
            altitude_km: 550.0,
            base_altitude_km: 550.0,
            inclination_deg: 53.0,
            ascending_node_deg: 0.0,
            mean_anomaly_deg: 0.0,
            fuel_remaining: 1.0,
            fuel_capacity: 100.0,
            station_keeping_rate: 0.0001,
            status: SatelliteStatus::Operational,
            constellation_id: 0,
            mass_kg: 260.0,
            max_isl_links: 4,
            coverage_cone_half_angle_deg: 45.0,
            launched_tick: 0,
            plane_index: 0,
            index_in_plane: 0,
        };

        let radius = sat.footprint_radius_km(25.0); // 25 degree min elevation
        // At 550km with 25deg elevation, footprint ~500-700km radius
        assert!(
            radius > 300.0 && radius < 1000.0,
            "Footprint radius was {}",
            radius
        );
    }

    #[test]
    fn test_standard_shells() {
        let shells = OrbitalShell::standard_shells();
        assert_eq!(shells.len(), 7);
        assert_eq!(
            OrbitalShell::shell_index_for_altitude(&shells, 550.0),
            Some(0)
        );
        assert_eq!(
            OrbitalShell::shell_index_for_altitude(&shells, 800.0),
            Some(1)
        );
    }
}
