use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadSegment {
    pub id: u64,
    pub from: (f64, f64), // (lon, lat)
    pub to: (f64, f64),
    pub road_class: RoadClass,
    pub length_km: f64,
    pub region_id: EntityId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

    /// Find the nearest road segment to a point.
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
    pub fn pathfind(&self, from: (f64, f64), to: (f64, f64)) -> Vec<(f64, f64)> {
        if self.segments.is_empty() {
            return vec![from, to];
        }

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
                waypoints.dedup_by(|a, b| points_close(*a, *b, 0.0001));
                return waypoints;
            }

            if visited[curr_idx] { continue; }
            visited[curr_idx] = true;

            let curr_g = g_score.get(&curr_idx).copied().unwrap_or(f64::MAX);
            for &neighbor_idx in &self.adjacency[curr_idx] {
                if visited[neighbor_idx] { continue; }
                let edge_cost = self.segments[curr_idx].length_km;
                let tentative_g = curr_g + edge_cost;
                let prev_g = g_score.get(&neighbor_idx).copied().unwrap_or(f64::MAX);
                if tentative_g < prev_g {
                    came_from.insert(neighbor_idx, curr_idx);
                    g_score.insert(neighbor_idx, tentative_g);
                    let h = haversine_km_points(midpoint(self.segments[neighbor_idx].from, self.segments[neighbor_idx].to), target_mid);
                    open.push(AStarEntry { f_score: OrderedF64(-(tentative_g + h)), index: neighbor_idx });
                }
            }
        }
        vec![from, to]
    }

    pub fn fiber_route_cost(&self, from: (f64, f64), to: (f64, f64)) -> f64 {
        if self.segments.is_empty() { return haversine_km_points(from, to) * 1.5; }
        let start_idx = match self.nearest_segment_index(from.0, from.1) { Some(i) => i, None => return haversine_km_points(from, to) * 1.5 };
        let end_idx = match self.nearest_segment_index(to.0, to.1) { Some(i) => i, None => return haversine_km_points(from, to) * 1.5 };
        if start_idx == end_idx { return self.segments[start_idx].length_km * self.segments[start_idx].road_class.fiber_cost_multiplier(); }

        let target_mid = midpoint(self.segments[end_idx].from, self.segments[end_idx].to);
        let mut g_score: HashMap<usize, f64> = HashMap::new();
        g_score.insert(start_idx, 0.0);
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut open = BinaryHeap::new();
        let start_h = haversine_km_points(midpoint(self.segments[start_idx].from, self.segments[start_idx].to), target_mid);
        open.push(AStarEntry { f_score: OrderedF64(-start_h), index: start_idx });
        let mut visited = vec![false; self.segments.len()];

        while let Some(current) = open.pop() {
            let curr_idx = current.index;
            if curr_idx == end_idx {
                let mut path_indices = vec![end_idx];
                let mut node = end_idx;
                while let Some(&prev) = came_from.get(&node) { path_indices.push(prev); node = prev; }
                let mut total_cost = 0.0;
                for &idx in &path_indices { total_cost += self.segments[idx].length_km * self.segments[idx].road_class.fiber_cost_multiplier(); }
                return total_cost;
            }
            if visited[curr_idx] { continue; }
            visited[curr_idx] = true;
            let curr_g = g_score.get(&curr_idx).copied().unwrap_or(f64::MAX);
            for &neighbor_idx in &self.adjacency[curr_idx] {
                if visited[neighbor_idx] { continue; }
                let edge_cost = self.segments[curr_idx].length_km * self.segments[curr_idx].road_class.fiber_cost_multiplier();
                let tentative_g = curr_g + edge_cost;
                let prev_g = g_score.get(&neighbor_idx).copied().unwrap_or(f64::MAX);
                if tentative_g < prev_g {
                    came_from.insert(neighbor_idx, curr_idx);
                    g_score.insert(neighbor_idx, tentative_g);
                    let h = haversine_km_points(midpoint(self.segments[neighbor_idx].from, self.segments[neighbor_idx].to), target_mid);
                    open.push(AStarEntry { f_score: OrderedF64(-(tentative_g + h)), index: neighbor_idx });
                }
            }
        }
        haversine_km_points(from, to) * 1.5
    }

    fn nearest_segment_index(&self, lon: f64, lat: f64) -> Option<usize> {
        if self.segments.is_empty() { return None; }
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        for (i, seg) in self.segments.iter().enumerate() {
            let dist = point_to_segment_dist_sq(lon, lat, seg.from, seg.to);
            if dist < best_dist { best_dist = dist; best_idx = i; }
        }
        Some(best_idx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF64(f64);
impl Eq for OrderedF64 {}
impl PartialOrd for OrderedF64 { fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) } }
impl Ord for OrderedF64 { fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal) } }
#[derive(Debug, Clone, Eq, PartialEq)]
struct AStarEntry { f_score: OrderedF64, index: usize }
impl Ord for AStarEntry { fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.f_score.cmp(&other.f_score) } }
impl PartialOrd for AStarEntry { fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) } }

fn points_close(a: (f64, f64), b: (f64, f64), eps: f64) -> bool { (a.0 - b.0).abs() < eps && (a.1 - b.1).abs() < eps }
fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0) }
fn point_to_segment_dist_sq(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0; let dy = b.1 - a.1; let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 { let dx2 = px - a.0; let dy2 = py - a.1; return dx2 * dx2 + dy2 * dy2; }
    let t = (((px - a.0) * dx + (py - a.1) * dy) / len_sq).clamp(0.0, 1.0);
    let dx2 = px - (a.0 + t * dx); let dy2 = py - (a.1 + t * dy);
    dx2 * dx2 + dy2 * dy2
}
/// Haversine distance in km between two (lon, lat) tuples.
fn haversine_km_points(a: (f64, f64), b: (f64, f64)) -> f64 {
    gt_common::geo::haversine_km(a.1, a.0, b.1, b.0)
}
