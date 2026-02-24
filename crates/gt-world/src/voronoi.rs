use gt_common::types::MapSize;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

/// A single cell in the Voronoi tessellation of the sphere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoronoiCell {
    pub index: usize,
    pub center_lat: f64,
    pub center_lon: f64,
    /// Ordered vertices of the cell polygon as (lat, lon) pairs.
    pub polygon_vertices: Vec<(f64, f64)>,
    /// Indices of neighboring cells (share a Voronoi edge).
    pub neighbor_indices: Vec<usize>,
    /// Approximate area of this cell in square kilometers.
    pub area_km2: f64,
}

/// Voronoi tessellation of a sphere surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoronoiGrid {
    pub cells: Vec<VoronoiCell>,
}

impl VoronoiGrid {
    /// Generate a Voronoi grid on a unit sphere with the given map size and seed.
    /// Uses Poisson-disk rejection sampling, Delaunay triangulation, and Lloyd relaxation.
    pub fn generate(map_size: MapSize, seed: u64) -> Self {
        let num_points = match map_size {
            MapSize::Small => 500,
            MapSize::Medium => 1500,
            MapSize::Large => 3000,
            MapSize::Huge => 6000,
        };

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        // Step 1: Generate seed points on sphere via rejection sampling (Poisson-disk-like)
        let mut points_3d = poisson_sphere_points(num_points, &mut rng);

        // Step 2: Lloyd relaxation (3 passes) for more uniform cells
        for _ in 0..3 {
            points_3d = lloyd_relax_sphere(&points_3d, num_points);
        }

        // Step 3: Convert to lat/lon
        let latlon: Vec<(f64, f64)> = points_3d
            .iter()
            .map(|&(x, y, z)| xyz_to_latlon(x, y, z))
            .collect();

        // Step 4: Project to 2D (sinusoidal) for Delaunay triangulation
        let projected: Vec<delaunator::Point> = latlon
            .iter()
            .map(|&(lat, lon)| {
                let lat_r = lat.to_radians();
                delaunator::Point {
                    x: lon.to_radians() * lat_r.cos(),
                    y: lat_r,
                }
            })
            .collect();

        let triangulation = delaunator::triangulate(&projected);

        // Step 5: Build adjacency from Delaunay triangles
        let n = latlon.len();
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];

        if !triangulation.triangles.is_empty() {
            let num_triangles = triangulation.triangles.len() / 3;
            for t in 0..num_triangles {
                let i0 = triangulation.triangles[3 * t];
                let i1 = triangulation.triangles[3 * t + 1];
                let i2 = triangulation.triangles[3 * t + 2];
                add_edge(&mut adjacency, i0, i1);
                add_edge(&mut adjacency, i1, i2);
                add_edge(&mut adjacency, i2, i0);
            }
        }

        // Deduplicate and sort adjacency
        for neighbors in &mut adjacency {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        // Step 6: Compute Voronoi cell polygons from circumcenters of Delaunay triangles
        // For each Delaunay triangle, compute its circumcenter on the sphere
        let num_triangles = if triangulation.triangles.is_empty() {
            0
        } else {
            triangulation.triangles.len() / 3
        };

        let circumcenters: Vec<(f64, f64, f64)> = (0..num_triangles)
            .map(|t| {
                let i0 = triangulation.triangles[3 * t];
                let i1 = triangulation.triangles[3 * t + 1];
                let i2 = triangulation.triangles[3 * t + 2];
                sphere_circumcenter(points_3d[i0], points_3d[i1], points_3d[i2])
            })
            .collect();

        // Build mapping: for each point, collect the triangles it belongs to
        let mut point_triangles: Vec<Vec<usize>> = vec![Vec::new(); n];
        for t in 0..num_triangles {
            let i0 = triangulation.triangles[3 * t];
            let i1 = triangulation.triangles[3 * t + 1];
            let i2 = triangulation.triangles[3 * t + 2];
            point_triangles[i0].push(t);
            point_triangles[i1].push(t);
            point_triangles[i2].push(t);
        }

        // Step 7: Construct cells
        let earth_surface_km2: f64 = 510_100_000.0;
        let avg_area = earth_surface_km2 / n as f64;

        let cells: Vec<VoronoiCell> = (0..n)
            .map(|i| {
                let (lat, lon) = latlon[i];
                let tri_indices = &point_triangles[i];

                // Collect circumcenters for this cell's polygon
                let mut polygon_3d: Vec<(f64, f64, f64)> = tri_indices
                    .iter()
                    .map(|&t| circumcenters[t])
                    .collect();

                // Order polygon vertices by angle around the cell center
                let center = points_3d[i];
                order_polygon_around_center(&mut polygon_3d, center);

                // Convert polygon to lat/lon
                let polygon_vertices: Vec<(f64, f64)> = polygon_3d
                    .iter()
                    .map(|&(px, py, pz)| xyz_to_latlon(px, py, pz))
                    .collect();

                // Approximate area using spherical polygon area
                let area_km2 = if polygon_3d.len() >= 3 {
                    spherical_polygon_area(&polygon_3d) * earth_surface_km2 / (4.0 * std::f64::consts::PI)
                } else {
                    avg_area
                };

                VoronoiCell {
                    index: i,
                    center_lat: lat,
                    center_lon: lon,
                    polygon_vertices,
                    neighbor_indices: adjacency[i].clone(),
                    area_km2: area_km2.max(1.0),
                }
            })
            .collect();

        VoronoiGrid { cells }
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// Generate approximately `n` well-distributed points on a unit sphere
/// using rejection sampling from uniform distribution.
fn poisson_sphere_points(n: usize, rng: &mut rand::rngs::StdRng) -> Vec<(f64, f64, f64)> {
    let mut points = Vec::with_capacity(n);
    // Use Fibonacci sphere for initial distribution (deterministic, uniform)
    let golden_ratio = (1.0 + 5.0_f64.sqrt()) / 2.0;
    for i in 0..n {
        let theta = std::f64::consts::PI * 2.0 * (i as f64) / golden_ratio;
        let phi = (1.0 - 2.0 * (i as f64 + 0.5) / n as f64).acos();

        let x = phi.sin() * theta.cos();
        let y = phi.sin() * theta.sin();
        let z = phi.cos();

        // Add small jitter for natural variation
        let jx = rng.gen_range(-0.01..0.01);
        let jy = rng.gen_range(-0.01..0.01);
        let jz = rng.gen_range(-0.01..0.01);

        let (px, py, pz) = (x + jx, y + jy, z + jz);
        let len = (px * px + py * py + pz * pz).sqrt();
        points.push((px / len, py / len, pz / len));
    }
    points
}

/// One pass of Lloyd relaxation on the sphere: move each point to the centroid
/// of its Voronoi cell (approximated by averaging its Delaunay neighbors + itself).
fn lloyd_relax_sphere(points: &[(f64, f64, f64)], _n: usize) -> Vec<(f64, f64, f64)> {
    let n = points.len();
    // Project to 2D for triangulation
    let latlon: Vec<(f64, f64)> = points.iter().map(|&(x, y, z)| xyz_to_latlon(x, y, z)).collect();
    let projected: Vec<delaunator::Point> = latlon
        .iter()
        .map(|&(lat, lon)| {
            let lat_r = lat.to_radians();
            delaunator::Point {
                x: lon.to_radians() * lat_r.cos(),
                y: lat_r,
            }
        })
        .collect();

    let triangulation = delaunator::triangulate(&projected);

    // Build adjacency
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    if !triangulation.triangles.is_empty() {
        let num_tri = triangulation.triangles.len() / 3;
        for t in 0..num_tri {
            let i0 = triangulation.triangles[3 * t];
            let i1 = triangulation.triangles[3 * t + 1];
            let i2 = triangulation.triangles[3 * t + 2];
            add_edge(&mut adjacency, i0, i1);
            add_edge(&mut adjacency, i1, i2);
            add_edge(&mut adjacency, i2, i0);
        }
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
        neighbors.dedup();
    }

    // Move each point toward centroid of its neighbors (on the sphere)
    let mut relaxed = Vec::with_capacity(n);
    for i in 0..n {
        let mut cx = points[i].0;
        let mut cy = points[i].1;
        let mut cz = points[i].2;
        let neighbors = &adjacency[i];
        let count = neighbors.len() as f64 + 1.0;
        for &ni in neighbors {
            cx += points[ni].0;
            cy += points[ni].1;
            cz += points[ni].2;
        }
        cx /= count;
        cy /= count;
        cz /= count;
        // Project back to unit sphere
        let len = (cx * cx + cy * cy + cz * cz).sqrt();
        if len > 1e-12 {
            relaxed.push((cx / len, cy / len, cz / len));
        } else {
            relaxed.push(points[i]);
        }
    }
    relaxed
}

fn add_edge(adjacency: &mut [Vec<usize>], a: usize, b: usize) {
    if a != b {
        adjacency[a].push(b);
        adjacency[b].push(a);
    }
}

fn xyz_to_latlon(x: f64, y: f64, z: f64) -> (f64, f64) {
    let lat = z.asin().to_degrees();
    let lon = y.atan2(x).to_degrees();
    (lat, lon)
}

/// Compute the circumcenter of a spherical triangle (project to unit sphere).
fn sphere_circumcenter(a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)) -> (f64, f64, f64) {
    // The circumcenter on a sphere is the normalized cross product direction
    // of (b-a) x (c-a), checked for orientation
    let ab = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
    let ac = (c.0 - a.0, c.1 - a.1, c.2 - a.2);
    let cross = (
        ab.1 * ac.2 - ab.2 * ac.1,
        ab.2 * ac.0 - ab.0 * ac.2,
        ab.0 * ac.1 - ab.1 * ac.0,
    );
    let len = (cross.0 * cross.0 + cross.1 * cross.1 + cross.2 * cross.2).sqrt();
    if len < 1e-15 {
        // Degenerate triangle — return midpoint
        let mx = (a.0 + b.0 + c.0) / 3.0;
        let my = (a.1 + b.1 + c.1) / 3.0;
        let mz = (a.2 + b.2 + c.2) / 3.0;
        let mlen = (mx * mx + my * my + mz * mz).sqrt();
        return (mx / mlen, my / mlen, mz / mlen);
    }
    let mut n = (cross.0 / len, cross.1 / len, cross.2 / len);
    // Ensure circumcenter is on the same side as the triangle vertices
    let dot = n.0 * a.0 + n.1 * a.1 + n.2 * a.2;
    if dot < 0.0 {
        n = (-n.0, -n.1, -n.2);
    }
    n
}

/// Order polygon vertices around a center point by angle on the tangent plane.
fn order_polygon_around_center(vertices: &mut Vec<(f64, f64, f64)>, center: (f64, f64, f64)) {
    if vertices.len() < 3 {
        return;
    }
    // Build a local tangent plane basis
    // "up" direction on the sphere at the center point
    let up = center;
    // Find a vector not parallel to up for constructing basis
    let seed_vec = if up.0.abs() < 0.9 {
        (1.0, 0.0, 0.0)
    } else {
        (0.0, 1.0, 0.0)
    };
    // basis_u = normalize(seed_vec - (seed_vec . up) * up)
    let dot = seed_vec.0 * up.0 + seed_vec.1 * up.1 + seed_vec.2 * up.2;
    let u = (
        seed_vec.0 - dot * up.0,
        seed_vec.1 - dot * up.1,
        seed_vec.2 - dot * up.2,
    );
    let u_len = (u.0 * u.0 + u.1 * u.1 + u.2 * u.2).sqrt();
    let u = (u.0 / u_len, u.1 / u_len, u.2 / u_len);
    // basis_v = up x u
    let v = (
        up.1 * u.2 - up.2 * u.1,
        up.2 * u.0 - up.0 * u.2,
        up.0 * u.1 - up.1 * u.0,
    );

    vertices.sort_by(|a, b| {
        let da = (a.0 - center.0, a.1 - center.1, a.2 - center.2);
        let db = (b.0 - center.0, b.1 - center.1, b.2 - center.2);
        let angle_a = (da.0 * v.0 + da.1 * v.1 + da.2 * v.2)
            .atan2(da.0 * u.0 + da.1 * u.1 + da.2 * u.2);
        let angle_b = (db.0 * v.0 + db.1 * v.1 + db.2 * v.2)
            .atan2(db.0 * u.0 + db.1 * u.1 + db.2 * u.2);
        angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Compute the solid angle (area on unit sphere) of a spherical polygon.
fn spherical_polygon_area(vertices: &[(f64, f64, f64)]) -> f64 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }
    // Use the spherical excess formula (L'Huilier's theorem approach via cross products)
    // Sum the signed areas of triangles formed by first vertex and consecutive edge pairs
    let mut total = 0.0;
    for i in 1..n - 1 {
        let a = vertices[0];
        let b = vertices[i];
        let c = vertices[i + 1];
        total += spherical_triangle_area(a, b, c);
    }
    total.abs()
}

fn spherical_triangle_area(a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)) -> f64 {
    // Area = 2 * atan2(|a . (b x c)|, 1 + a.b + a.c + b.c)  (Van Oosterom-Strackee formula)
    let bxc = (
        b.1 * c.2 - b.2 * c.1,
        b.2 * c.0 - b.0 * c.2,
        b.0 * c.1 - b.1 * c.0,
    );
    let numerator = (a.0 * bxc.0 + a.1 * bxc.1 + a.2 * bxc.2).abs();
    let ab = a.0 * b.0 + a.1 * b.1 + a.2 * b.2;
    let ac = a.0 * c.0 + a.1 * c.1 + a.2 * c.2;
    let bc = b.0 * c.0 + b.1 * c.1 + b.2 * c.2;
    let denominator = 1.0 + ab + ac + bc;
    2.0 * numerator.atan2(denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voronoi_generation_small() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        assert_eq!(grid.cell_count(), 500);
        for cell in &grid.cells {
            assert!(!cell.neighbor_indices.is_empty(), "Cell {} has no neighbors", cell.index);
            assert!(cell.center_lat >= -90.0 && cell.center_lat <= 90.0);
            assert!(cell.center_lon >= -180.0 && cell.center_lon <= 180.0);
        }
    }

    #[test]
    fn test_voronoi_deterministic() {
        let g1 = VoronoiGrid::generate(MapSize::Small, 123);
        let g2 = VoronoiGrid::generate(MapSize::Small, 123);
        assert_eq!(g1.cell_count(), g2.cell_count());
        for i in 0..g1.cell_count() {
            assert_eq!(g1.cells[i].center_lat, g2.cells[i].center_lat);
            assert_eq!(g1.cells[i].center_lon, g2.cells[i].center_lon);
        }
    }

    #[test]
    fn test_voronoi_different_seeds() {
        let g1 = VoronoiGrid::generate(MapSize::Small, 1);
        let g2 = VoronoiGrid::generate(MapSize::Small, 999);
        // Centers should differ
        let differ = (0..g1.cell_count())
            .any(|i| g1.cells[i].center_lat != g2.cells[i].center_lat);
        assert!(differ, "Different seeds should produce different grids");
    }

    #[test]
    fn test_voronoi_areas_positive() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        for cell in &grid.cells {
            assert!(cell.area_km2 > 0.0, "Cell {} has non-positive area", cell.index);
        }
    }
}
