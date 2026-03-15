/// Haversine distance in km between two points given in degrees.
///
/// Parameters: (lat1, lon1, lat2, lon2) — latitude first, then longitude.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat1 - lat2).to_radians();
    let dlon = (lon1 - lon2).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}

/// Haversine distance from (lat, lon) tuples.
pub fn haversine_km_points(a: &(f64, f64), b: &(f64, f64)) -> f64 {
    haversine_km(a.0, a.1, b.0, b.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_distance() {
        // New York (40.7128, -74.0060) to London (51.5074, -0.1278) ≈ 5570 km
        let dist = haversine_km(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((dist - 5570.0).abs() < 50.0, "NYC-London distance was {dist}");
    }

    #[test]
    fn test_same_point() {
        let dist = haversine_km(0.0, 0.0, 0.0, 0.0);
        assert!((dist - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_points_helper() {
        let a = (40.7128, -74.0060);
        let b = (51.5074, -0.1278);
        let dist = haversine_km_points(&a, &b);
        assert!((dist - 5570.0).abs() < 50.0);
    }
}
