use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadSegment {
    pub id: u64,
    pub from: (f64, f64), // (lon, lat)
    pub to: (f64, f64),
    pub road_class: RoadClass,
    pub length_km: f64,
    pub region_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RoadClass {
    Highway,
    Primary,
    Secondary,
    Residential,
    Local,
}

impl RoadClass {
    /// Cost multiplier for fiber routing along this road class.
    pub fn fiber_cost_multiplier(&self) -> f64 {
        match self {
            RoadClass::Highway => 0.8,    // Cheapest - existing ROW
            RoadClass::Primary => 0.9,
            RoadClass::Secondary => 1.0,
            RoadClass::Residential => 1.2, // Permit costs
            RoadClass::Local => 1.5,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoadNetwork {
    pub segments: Vec<RoadSegment>,
    adjacency: Vec<Vec<usize>>, // adjacency list by segment index
}

impl RoadNetwork {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            adjacency: Vec::new(),
        }
    }

    /// Add a road segment and update the adjacency list.
    /// Two segments are adjacent if they share an endpoint (within a small epsilon).
    pub fn add_segment(&mut self, seg: RoadSegment) {
        let new_idx = self.segments.len();
        let eps = 0.001; // ~111 meters at equator

        let mut adj = Vec::new();

        for (i, existing) in self.segments.iter().enumerate() {
            let connected = points_close(seg.from, existing.from, eps)
                || points_close(seg.from, existing.to, eps)
                || points_close(seg.to, existing.from, eps)
                || points_close(seg.to, existing.to, eps);

            if connected {
                adj.push(i);
                self.adjacency[i].push(new_idx);
            }
        }

        self.segments.push(seg);
        self.adjacency.push(adj);
    }

    /// Find the nearest road segment to a point (simple linear scan).
    pub fn nearest_segment(&self, lon: f64, lat: f64) -> Option<&RoadSegment> {
        if self.segments.is_empty() {
            return None;
        }

        self.segments
            .iter()
            .min_by(|a, b| {
                let dist_a = point_to_segment_dist_sq(lon, lat, a.from, a.to);
                let dist_b = point_to_segment_dist_sq(lon, lat, b.from, b.to);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// A* pathfinding along the road graph. Returns waypoints as (lon, lat).
    /// If no path is found, returns a direct line [from, to].
    pub fn pathfind(&self, from: (f64, f64), to: (f64, f64)) -> Vec<(f64, f64)> {
        if self.segments.is_empty() {
            return vec![from, to];
        }

        // Find nearest segments to start and end
        let start_idx = match self.nearest_segment_index(from.0, from.1) {
            Some(i) => i,
            None => return vec![from, to],
        };
        let end_idx = match self.nearest_segment_index(to.0, to.1) {
            Some(i) => i,
            None => return vec![from, to],
        };

        if start_idx == end_idx {
            return vec![from, self.segments[start_idx].from, self.segments[start_idx].to, to];
        }

        // A* search on segment graph
        // Cost = segment length_km, heuristic = haversine to target segment midpoint
        let target_mid = midpoint(self.segments[end_idx].from, self.segments[end_idx].to);

        let mut g_score: HashMap<usize, f64> = HashMap::new();
        g_score.insert(start_idx, 0.0);

        let mut came_from: HashMap<usize, usize> = HashMap::new();

        // (negative f_score for max-heap → min-heap, segment_index)
        let mut open = BinaryHeap::new();
        let start_h = haversine_km_points(
            midpoint(self.segments[start_idx].from, self.segments[start_idx].to),
            target_mid,
        );
        open.push(AStarEntry {
            f_score: OrderedF64(-start_h),
            index: start_idx,
        });

        let mut visited = vec![false; self.segments.len()];

        while let Some(current) = open.pop() {
            let curr_idx = current.index;

            if curr_idx == end_idx {
                // Reconstruct path
                let mut path_indices = vec![end_idx];
                let mut node = end_idx;
                while let Some(&prev) = came_from.get(&node) {
                    path_indices.push(prev);
                    node = prev;
                }
                path_indices.reverse();

                let mut waypoints = vec![from];
                for &idx in &path_indices {
                    let seg = &self.segments[idx];
                    waypoints.push(seg.from);
                    waypoints.push(seg.to);
                }
                waypoints.push(to);
                // Deduplicate consecutive near-identical points
                waypoints.dedup_by(|a, b| points_close(*a, *b, 0.0001));
                return waypoints;
            }

            if visited[curr_idx] {
                continue;
            }
            visited[curr_idx] = true;

            let curr_g = g_score.get(&curr_idx).copied().unwrap_or(f64::MAX);

            for &neighbor_idx in &self.adjacency[curr_idx] {
                if visited[neighbor_idx] {
                    continue;
                }

                let edge_cost = self.segments[curr_idx].length_km;
                let tentative_g = curr_g + edge_cost;

                let prev_g = g_score.get(&neighbor_idx).copied().unwrap_or(f64::MAX);
                if tentative_g < prev_g {
                    came_from.insert(neighbor_idx, curr_idx);
                    g_score.insert(neighbor_idx, tentative_g);
                    let h = haversine_km_points(
                        midpoint(
                            self.segments[neighbor_idx].from,
                            self.segments[neighbor_idx].to,
                        ),
                        target_mid,
                    );
                    open.push(AStarEntry {
                        f_score: OrderedF64(-(tentative_g + h)),
                        index: neighbor_idx,
                    });
                }
            }
        }

        // No path found — direct line
        vec![from, to]
    }

    /// Cost of routing fiber along roads between two points.
    /// Uses pathfinding to find the road route, then sums segment costs
    /// with road class fiber cost multipliers applied.
    pub fn fiber_route_cost(&self, from: (f64, f64), to: (f64, f64)) -> f64 {
        if self.segments.is_empty() {
            // No roads: use direct haversine distance with default multiplier
            return haversine_km_points(from, to) * 1.5;
        }

        let start_idx = match self.nearest_segment_index(from.0, from.1) {
            Some(i) => i,
            None => return haversine_km_points(from, to) * 1.5,
        };
        let end_idx = match self.nearest_segment_index(to.0, to.1) {
            Some(i) => i,
            None => return haversine_km_points(from, to) * 1.5,
        };

        if start_idx == end_idx {
            let seg = &self.segments[start_idx];
            return seg.length_km * seg.road_class.fiber_cost_multiplier();
        }

        // Find path using same A* logic but track visited segments for cost
        let target_mid = midpoint(self.segments[end_idx].from, self.segments[end_idx].to);
        let mut g_score: HashMap<usize, f64> = HashMap::new();
        g_score.insert(start_idx, 0.0);
        let mut came_from: HashMap<usize, usize> = HashMap::new();

        let mut open = BinaryHeap::new();
        let start_h = haversine_km_points(
            midpoint(self.segments[start_idx].from, self.segments[start_idx].to),
            target_mid,
        );
        open.push(AStarEntry {
            f_score: OrderedF64(-start_h),
            index: start_idx,
        });

        let mut visited = vec![false; self.segments.len()];

        while let Some(current) = open.pop() {
            let curr_idx = current.index;

            if curr_idx == end_idx {
                // Reconstruct path and sum costs
                let mut path_indices = vec![end_idx];
                let mut node = end_idx;
                while let Some(&prev) = came_from.get(&node) {
                    path_indices.push(prev);
                    node = prev;
                }

                let mut total_cost = 0.0;
                for &idx in &path_indices {
                    let seg = &self.segments[idx];
                    total_cost += seg.length_km * seg.road_class.fiber_cost_multiplier();
                }
                return total_cost;
            }

            if visited[curr_idx] {
                continue;
            }
            visited[curr_idx] = true;

            let curr_g = g_score.get(&curr_idx).copied().unwrap_or(f64::MAX);

            for &neighbor_idx in &self.adjacency[curr_idx] {
                if visited[neighbor_idx] {
                    continue;
                }

                let seg = &self.segments[curr_idx];
                let edge_cost = seg.length_km * seg.road_class.fiber_cost_multiplier();
                let tentative_g = curr_g + edge_cost;

                let prev_g = g_score.get(&neighbor_idx).copied().unwrap_or(f64::MAX);
                if tentative_g < prev_g {
                    came_from.insert(neighbor_idx, curr_idx);
                    g_score.insert(neighbor_idx, tentative_g);
                    let h = haversine_km_points(
                        midpoint(
                            self.segments[neighbor_idx].from,
                            self.segments[neighbor_idx].to,
                        ),
                        target_mid,
                    );
                    open.push(AStarEntry {
                        f_score: OrderedF64(-(tentative_g + h)),
                        index: neighbor_idx,
                    });
                }
            }
        }

        // No path found — direct distance with penalty
        haversine_km_points(from, to) * 1.5
    }

    /// Find the index of the nearest segment to a point.
    fn nearest_segment_index(&self, lon: f64, lat: f64) -> Option<usize> {
        if self.segments.is_empty() {
            return None;
        }

        let mut best_idx = 0;
        let mut best_dist = f64::MAX;

        for (i, seg) in self.segments.iter().enumerate() {
            let dist = point_to_segment_dist_sq(lon, lat, seg.from, seg.to);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }

        Some(best_idx)
    }
}

// ── A* helpers ──────────────────────────────────────────────────────────────

/// Wrapper for f64 that implements Ord for use in BinaryHeap.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AStarEntry {
    f_score: OrderedF64,
    index: usize,
}

impl Ord for AStarEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.f_score.cmp(&other.f_score)
    }
}

impl PartialOrd for AStarEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ── Geometry utilities ──────────────────────────────────────────────────────

fn points_close(a: (f64, f64), b: (f64, f64), eps: f64) -> bool {
    (a.0 - b.0).abs() < eps && (a.1 - b.1).abs() < eps
}

fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// Squared distance from point (px, py) to the closest point on segment (a, b).
/// All coordinates are (lon, lat).
fn point_to_segment_dist_sq(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-12 {
        // Degenerate segment (point)
        let dx2 = px - a.0;
        let dy2 = py - a.1;
        return dx2 * dx2 + dy2 * dy2;
    }

    // Project point onto line, clamp t to [0, 1]
    let t = ((px - a.0) * dx + (py - a.1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = a.0 + t * dx;
    let proj_y = a.1 + t * dy;

    let dx2 = px - proj_x;
    let dy2 = py - proj_y;
    dx2 * dx2 + dy2 * dy2
}

/// Haversine distance in km between two (lon, lat) points.
fn haversine_km_points(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dlat = (a.1 - b.1).to_radians();
    let dlon = (a.0 - b.0).to_radians();
    let lat1 = a.1.to_radians();
    let lat2 = b.1.to_radians();
    let a_val =
        (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a_val.sqrt().asin();
    6371.0 * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_road_network() {
        let net = RoadNetwork::new();
        assert!(net.nearest_segment(0.0, 0.0).is_none());
        assert_eq!(net.pathfind((0.0, 0.0), (1.0, 1.0)), vec![(0.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn test_add_segment_and_find_nearest() {
        let mut net = RoadNetwork::new();
        net.add_segment(RoadSegment {
            id: 1,
            from: (10.0, 50.0),
            to: (11.0, 50.0),
            road_class: RoadClass::Highway,
            length_km: 70.0,
            region_id: 1,
        });
        let nearest = net.nearest_segment(10.5, 50.0).unwrap();
        assert_eq!(nearest.id, 1);
    }

    #[test]
    fn test_adjacency() {
        let mut net = RoadNetwork::new();
        net.add_segment(RoadSegment {
            id: 1,
            from: (10.0, 50.0),
            to: (11.0, 50.0),
            road_class: RoadClass::Primary,
            length_km: 70.0,
            region_id: 1,
        });
        // This segment shares endpoint (11.0, 50.0) with the first
        net.add_segment(RoadSegment {
            id: 2,
            from: (11.0, 50.0),
            to: (12.0, 50.0),
            road_class: RoadClass::Primary,
            length_km: 70.0,
            region_id: 1,
        });
        assert_eq!(net.adjacency[0], vec![1]);
        assert_eq!(net.adjacency[1], vec![0]);
    }

    #[test]
    fn test_pathfind_connected() {
        let mut net = RoadNetwork::new();
        net.add_segment(RoadSegment {
            id: 1,
            from: (10.0, 50.0),
            to: (11.0, 50.0),
            road_class: RoadClass::Highway,
            length_km: 70.0,
            region_id: 1,
        });
        net.add_segment(RoadSegment {
            id: 2,
            from: (11.0, 50.0),
            to: (12.0, 50.0),
            road_class: RoadClass::Highway,
            length_km: 70.0,
            region_id: 1,
        });
        let path = net.pathfind((10.0, 50.0), (12.0, 50.0));
        assert!(path.len() >= 3);
        // First waypoint should be near start, last near end
        assert!((path[0].0 - 10.0).abs() < 0.01);
        assert!((path.last().unwrap().0 - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_fiber_route_cost() {
        let mut net = RoadNetwork::new();
        net.add_segment(RoadSegment {
            id: 1,
            from: (10.0, 50.0),
            to: (11.0, 50.0),
            road_class: RoadClass::Highway,
            length_km: 70.0,
            region_id: 1,
        });
        let cost = net.fiber_route_cost((10.0, 50.0), (11.0, 50.0));
        // Highway multiplier is 0.8, so cost = 70.0 * 0.8 = 56.0
        assert!((cost - 56.0).abs() < 0.01);
    }

    #[test]
    fn test_fiber_cost_multipliers() {
        assert_eq!(RoadClass::Highway.fiber_cost_multiplier(), 0.8);
        assert_eq!(RoadClass::Primary.fiber_cost_multiplier(), 0.9);
        assert_eq!(RoadClass::Secondary.fiber_cost_multiplier(), 1.0);
        assert_eq!(RoadClass::Residential.fiber_cost_multiplier(), 1.2);
        assert_eq!(RoadClass::Local.fiber_cost_multiplier(), 1.5);
    }
}
