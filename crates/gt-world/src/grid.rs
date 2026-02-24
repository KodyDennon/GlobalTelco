use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub index: usize,
    pub center: (f64, f64, f64),
    pub neighbors: Vec<usize>,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeodesicGrid {
    pub cells: Vec<GridCell>,
    #[serde(skip)]
    spatial_hash: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl GeodesicGrid {
    /// Create a GeodesicGrid from pre-built cells (used by VoronoiGrid conversion).
    pub fn from_cells(cells: Vec<GridCell>) -> Self {
        let mut grid = Self {
            cells,
            spatial_hash: HashMap::new(),
        };
        grid.build_spatial_hash();
        grid
    }

    pub fn new(subdivisions: u32) -> Self {
        let (vertices, faces) = icosahedron();
        let (vertices, faces) = subdivide(vertices, faces, subdivisions);

        // The midpoint cache in subdivide() ensures no duplicate vertices,
        // so we can use the vertices directly without deduplication.

        let mut cells: Vec<GridCell> = vertices
            .iter()
            .enumerate()
            .map(|(i, &(x, y, z))| {
                let lat = z.asin().to_degrees();
                let lon = y.atan2(x).to_degrees();
                GridCell {
                    index: i,
                    center: (x, y, z),
                    neighbors: Vec::new(),
                    lat,
                    lon,
                }
            })
            .collect();

        // Build adjacency from faces — use face indices directly
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); cells.len()];
        for &(a, b, c) in &faces {
            adjacency[a].push(b);
            adjacency[a].push(c);
            adjacency[b].push(a);
            adjacency[b].push(c);
            adjacency[c].push(a);
            adjacency[c].push(b);
        }
        for (i, cell) in cells.iter_mut().enumerate() {
            let n = &mut adjacency[i];
            n.sort_unstable();
            n.dedup();
            n.retain(|&x| x != i);
            cell.neighbors = n.clone();
        }

        let mut grid = Self {
            cells,
            spatial_hash: HashMap::new(),
        };
        grid.build_spatial_hash();
        grid
    }

    fn build_spatial_hash(&mut self) {
        self.spatial_hash.clear();
        let resolution = 10.0;
        for cell in &self.cells {
            let key = (
                (cell.center.0 * resolution) as i32,
                (cell.center.1 * resolution) as i32,
                (cell.center.2 * resolution) as i32,
            );
            self.spatial_hash.entry(key).or_default().push(cell.index);
        }
    }

    pub fn nearest_cell(&self, x: f64, y: f64, z: f64) -> usize {
        let len = (x * x + y * y + z * z).sqrt();
        let (nx, ny, nz) = if len > 0.0 {
            (x / len, y / len, z / len)
        } else {
            (0.0, 0.0, 1.0)
        };

        let resolution = 10.0;
        let key = (
            (nx * resolution) as i32,
            (ny * resolution) as i32,
            (nz * resolution) as i32,
        );

        let mut best_idx = 0;
        let mut best_dist = f64::MAX;

        // Search nearby buckets
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let search_key = (key.0 + dx, key.1 + dy, key.2 + dz);
                    if let Some(indices) = self.spatial_hash.get(&search_key) {
                        for &idx in indices {
                            let c = &self.cells[idx];
                            let d = dist_sq(nx, ny, nz, c.center.0, c.center.1, c.center.2);
                            if d < best_dist {
                                best_dist = d;
                                best_idx = idx;
                            }
                        }
                    }
                }
            }
        }

        // Fallback: if spatial hash missed, brute force
        if best_dist == f64::MAX {
            for cell in &self.cells {
                let d = dist_sq(nx, ny, nz, cell.center.0, cell.center.1, cell.center.2);
                if d < best_dist {
                    best_dist = d;
                    best_idx = cell.index;
                }
            }
        }

        best_idx
    }

    pub fn nearest_cell_latlon(&self, lat: f64, lon: f64) -> usize {
        let (x, y, z) = latlon_to_xyz(lat, lon);
        self.nearest_cell(x, y, z)
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn land_cells<'a>(&'a self, is_land: &'a [bool]) -> impl Iterator<Item = usize> + 'a {
        (0..self.cells.len()).filter(move |&i| is_land[i])
    }
}

fn dist_sq(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> f64 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    let dz = z1 - z2;
    dx * dx + dy * dy + dz * dz
}

pub fn latlon_to_xyz(lat: f64, lon: f64) -> (f64, f64, f64) {
    let lat_r = lat.to_radians();
    let lon_r = lon.to_radians();
    (
        lat_r.cos() * lon_r.cos(),
        lat_r.cos() * lon_r.sin(),
        lat_r.sin(),
    )
}

type Vertex3 = (f64, f64, f64);
type Triangle = (usize, usize, usize);

fn icosahedron() -> (Vec<Vertex3>, Vec<Triangle>) {
    let t = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let mut verts = vec![
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ];

    // Normalize to unit sphere
    for v in &mut verts {
        let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        v.0 /= len;
        v.1 /= len;
        v.2 /= len;
    }

    let faces = vec![
        (0, 11, 5),
        (0, 5, 1),
        (0, 1, 7),
        (0, 7, 10),
        (0, 10, 11),
        (1, 5, 9),
        (5, 11, 4),
        (11, 10, 2),
        (10, 7, 6),
        (7, 1, 8),
        (3, 9, 4),
        (3, 4, 2),
        (3, 2, 6),
        (3, 6, 8),
        (3, 8, 9),
        (4, 9, 5),
        (2, 4, 11),
        (6, 2, 10),
        (8, 6, 7),
        (9, 8, 1),
    ];

    (verts, faces)
}

fn subdivide(
    mut vertices: Vec<Vertex3>,
    faces: Vec<Triangle>,
    iterations: u32,
) -> (Vec<Vertex3>, Vec<Triangle>) {
    let mut current_faces = faces;

    for _ in 0..iterations {
        let mut midpoint_cache: HashMap<(usize, usize), usize> = HashMap::new();
        let mut new_faces = Vec::with_capacity(current_faces.len() * 4);

        for &(a, b, c) in &current_faces {
            let ab = get_midpoint(a, b, &mut vertices, &mut midpoint_cache);
            let bc = get_midpoint(b, c, &mut vertices, &mut midpoint_cache);
            let ca = get_midpoint(c, a, &mut vertices, &mut midpoint_cache);

            new_faces.push((a, ab, ca));
            new_faces.push((b, bc, ab));
            new_faces.push((c, ca, bc));
            new_faces.push((ab, bc, ca));
        }

        current_faces = new_faces;
    }

    (vertices, current_faces)
}

fn get_midpoint(
    a: usize,
    b: usize,
    vertices: &mut Vec<(f64, f64, f64)>,
    cache: &mut HashMap<(usize, usize), usize>,
) -> usize {
    let key = if a < b { (a, b) } else { (b, a) };

    if let Some(&idx) = cache.get(&key) {
        return idx;
    }

    let va = vertices[a];
    let vb = vertices[b];
    let mut mid = (
        (va.0 + vb.0) / 2.0,
        (va.1 + vb.1) / 2.0,
        (va.2 + vb.2) / 2.0,
    );

    // Project to unit sphere
    let len = (mid.0 * mid.0 + mid.1 * mid.1 + mid.2 * mid.2).sqrt();
    mid.0 /= len;
    mid.1 /= len;
    mid.2 /= len;

    let idx = vertices.len();
    vertices.push(mid);
    cache.insert(key, idx);
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icosahedron_base() {
        let (verts, faces) = icosahedron();
        assert_eq!(verts.len(), 12);
        assert_eq!(faces.len(), 20);
        for v in &verts {
            let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
            assert!((len - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_grid_creation_small() {
        let grid = GeodesicGrid::new(2);
        assert!(grid.cell_count() >= 42);
        for cell in &grid.cells {
            assert!(
                !cell.neighbors.is_empty(),
                "Cell {} has no neighbors",
                cell.index
            );
        }
    }

    #[test]
    fn test_nearest_cell() {
        let grid = GeodesicGrid::new(2);
        let idx = grid.nearest_cell(0.0, 0.0, 1.0);
        let cell = &grid.cells[idx];
        assert!(
            cell.lat > 45.0,
            "North pole nearest cell should be in northern hemisphere"
        );
    }

    #[test]
    fn test_latlon_roundtrip() {
        let (x, y, z) = latlon_to_xyz(45.0, 90.0);
        let len = (x * x + y * y + z * z).sqrt();
        assert!((len - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cell_count_subdivision_4() {
        let grid = GeodesicGrid::new(4);
        // 10 * 4^4 + 2 = 2562
        assert_eq!(grid.cell_count(), 2562);
    }
}
