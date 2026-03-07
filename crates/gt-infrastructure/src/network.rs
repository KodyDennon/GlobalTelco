use gt_common::types::EntityId;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkGraph {
    adjacency: IndexMap<EntityId, Vec<EntityId>>,
    #[serde(with = "gt_common::serde_helpers::entity_pair_map")]
    edge_map: IndexMap<(EntityId, EntityId), EntityId>,
    dirty_nodes: IndexSet<EntityId>,
    #[serde(skip)]
    cached_paths: IndexMap<EntityId, IndexMap<EntityId, Vec<EntityId>>>,
}

impl NetworkGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, id: EntityId) {
        self.adjacency.entry(id).or_default();
        self.dirty_nodes.insert(id);
    }

    pub fn add_edge(&mut self, from: EntityId, to: EntityId) {
        self.adjacency.entry(from).or_default().push(to);
        self.adjacency.entry(to).or_default().push(from);
        self.dirty_nodes.insert(from);
        self.dirty_nodes.insert(to);
    }

    pub fn add_edge_with_id(&mut self, from: EntityId, to: EntityId, edge_id: EntityId) {
        self.add_edge(from, to);
        let key = if from < to { (from, to) } else { (to, from) };
        self.edge_map.insert(key, edge_id);
    }

    pub fn remove_node(&mut self, id: EntityId) {
        if let Some(neighbors) = self.adjacency.shift_remove(&id) {
            for neighbor in &neighbors {
                if let Some(adj) = self.adjacency.get_mut(neighbor) {
                    adj.retain(|&n| n != id);
                }
                self.dirty_nodes.insert(*neighbor);
            }
        }
        // Remove edges involving this node
        let keys_to_remove: Vec<(EntityId, EntityId)> = self
            .edge_map
            .keys()
            .filter(|(a, b)| *a == id || *b == id)
            .copied()
            .collect();
        for key in keys_to_remove {
            self.edge_map.shift_remove(&key);
        }
        self.dirty_nodes.shift_remove(&id);
        self.cached_paths.shift_remove(&id);
    }

    pub fn remove_edge(&mut self, from: EntityId, to: EntityId) {
        if let Some(adj) = self.adjacency.get_mut(&from) {
            adj.retain(|&n| n != to);
        }
        if let Some(adj) = self.adjacency.get_mut(&to) {
            adj.retain(|&n| n != from);
        }
        let key = if from < to { (from, to) } else { (to, from) };
        self.edge_map.shift_remove(&key);
        self.dirty_nodes.insert(from);
        self.dirty_nodes.insert(to);
    }

    pub fn neighbors(&self, id: EntityId) -> &[EntityId] {
        self.adjacency.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn node_count(&self) -> usize {
        self.adjacency.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_map.len()
    }

    pub fn nodes(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.adjacency.keys().copied()
    }

    pub fn has_dirty_nodes(&self) -> bool {
        !self.dirty_nodes.is_empty()
    }

    pub fn take_dirty_nodes(&mut self) -> IndexSet<EntityId> {
        std::mem::take(&mut self.dirty_nodes)
    }

    pub fn invalidate_node(&mut self, id: EntityId) {
        self.dirty_nodes.insert(id);
        self.cached_paths.shift_remove(&id);
    }

    /// Compute shortest path between two nodes using Dijkstra
    /// Weight function: latency * (1 / bandwidth) — lower is better
    pub fn shortest_path(
        &self,
        from: EntityId,
        to: EntityId,
        weight_fn: &dyn Fn(EntityId, EntityId) -> f64,
    ) -> Option<Vec<EntityId>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut dist: HashMap<EntityId, f64> = HashMap::new();
        let mut prev: HashMap<EntityId, EntityId> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(OrderedFloat, EntityId)>> = BinaryHeap::new();

        dist.insert(from, 0.0);
        heap.push(Reverse((OrderedFloat(0.0), from)));

        while let Some(Reverse((OrderedFloat(cost), node))) = heap.pop() {
            if node == to {
                // Reconstruct path
                let mut path = vec![to];
                let mut current = to;
                while let Some(&p) = prev.get(&current) {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some(path);
            }

            if cost > *dist.get(&node).unwrap_or(&f64::MAX) {
                continue;
            }

            for &neighbor in self.neighbors(node) {
                let edge_weight = weight_fn(node, neighbor);
                let new_dist = cost + edge_weight;
                if new_dist < *dist.get(&neighbor).unwrap_or(&f64::MAX) {
                    dist.insert(neighbor, new_dist);
                    prev.insert(neighbor, node);
                    heap.push(Reverse((OrderedFloat(new_dist), neighbor)));
                }
            }
        }

        None
    }

    /// Recompute cached paths for dirty nodes (No-op in lazy mode, kept for trait compatibility if needed)
    pub fn recompute_dirty(&mut self, _weight_fn: &dyn Fn(EntityId, EntityId) -> f64) {
        // In lazy mode, we don't precompute.
        // We just clear affected cache entries when nodes become dirty.
        let dirty = self.take_dirty_nodes();
        for id in dirty {
            self.cached_paths.shift_remove(&id);
        }
    }

    /// Get a path between two nodes, using cache if available, otherwise computing it.
    pub fn get_or_compute_path(
        &mut self,
        from: EntityId,
        to: EntityId,
        edge_weights: &HashMap<EntityId, f64>,
    ) -> Option<Vec<EntityId>> {
        if let Some(src_cache) = self.cached_paths.get(&from) {
            if let Some(path) = src_cache.get(&to) {
                return Some(path.clone());
            }
        }

        // Compute on demand
        let path = self.shortest_path_with_map(from, to, edge_weights)?;
        
        // Cache it (LRU-ish: we just insert, the simulation tick clears dirty sources)
        self.cached_paths
            .entry(from)
            .or_default()
            .insert(to, path.clone());
            
        Some(path)
    }

    /// Compute shortest path using an edge weight map (EntityId -> f64)
    pub fn shortest_path_with_map(
        &self,
        from: EntityId,
        to: EntityId,
        edge_weights: &HashMap<EntityId, f64>,
    ) -> Option<Vec<EntityId>> {
        let weight_fn = |a: EntityId, b: EntityId| -> f64 {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&eid) = self.edge_map.get(&key) {
                edge_weights.get(&eid).copied().unwrap_or(f64::MAX)
            } else {
                f64::MAX
            }
        };
        self.shortest_path(from, to, &weight_fn)
    }

    pub fn get_cached_path(&self, from: EntityId, to: EntityId) -> Option<&Vec<EntityId>> {
        self.cached_paths.get(&from)?.get(&to)
    }

    /// Get all cached paths from all sources (for traffic flow computation).
    pub fn get_all_cached_paths(&self) -> &IndexMap<EntityId, IndexMap<EntityId, Vec<EntityId>>> {
        &self.cached_paths
    }

    /// Get the edge entity ID for a pair of nodes.
    pub fn get_edge_id(&self, from: EntityId, to: EntityId) -> Option<EntityId> {
        let key = if from < to { (from, to) } else { (to, from) };
        self.edge_map.get(&key).copied()
    }

    /// Find an alternate path excluding specific nodes/edges with health below threshold.
    /// Used for rerouting during cascading failures.
    pub fn find_alternate_path(
        &self,
        from: EntityId,
        to: EntityId,
        exclude_nodes: &IndexSet<EntityId>,
        weight_fn: &dyn Fn(EntityId, EntityId) -> f64,
    ) -> Option<Vec<EntityId>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut dist: HashMap<EntityId, f64> = HashMap::new();
        let mut prev: HashMap<EntityId, EntityId> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(OrderedFloat, EntityId)>> = BinaryHeap::new();

        dist.insert(from, 0.0);
        heap.push(Reverse((OrderedFloat(0.0), from)));

        while let Some(Reverse((OrderedFloat(cost), node))) = heap.pop() {
            if node == to {
                let mut path = vec![to];
                let mut current = to;
                while let Some(&p) = prev.get(&current) {
                    path.push(p);
                    current = p;
                }
                path.reverse();
                return Some(path);
            }

            if cost > *dist.get(&node).unwrap_or(&f64::MAX) {
                continue;
            }

            for &neighbor in self.neighbors(node) {
                if exclude_nodes.contains(&neighbor) {
                    continue;
                }
                let edge_weight = weight_fn(node, neighbor);
                if edge_weight >= f64::MAX {
                    continue; // Damaged edge
                }
                let new_dist = cost + edge_weight;
                if new_dist < *dist.get(&neighbor).unwrap_or(&f64::MAX) {
                    dist.insert(neighbor, new_dist);
                    prev.insert(neighbor, node);
                    heap.push(Reverse((OrderedFloat(new_dist), neighbor)));
                }
            }
        }

        None
    }

    /// Get all nodes connected to the given node (reachable)
    pub fn connected_nodes(&self, start: EntityId) -> IndexSet<EntityId> {
        let mut visited = IndexSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited.insert(node) {
                for &neighbor in self.neighbors(node) {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
        visited
    }
}

/// Wrapper for f64 to implement Ord for BinaryHeap
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_nodes() {
        let mut graph = NetworkGraph::new();
        graph.add_node(1);
        graph.add_node(2);
        graph.add_edge(1, 2);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.neighbors(1), &[2]);

        graph.remove_node(2);
        assert_eq!(graph.node_count(), 1);
        assert!(graph.neighbors(1).is_empty());
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = NetworkGraph::new();
        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);

        let path = graph.shortest_path(1, 3, &|_, _| 1.0);
        assert_eq!(path, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_connected_nodes() {
        let mut graph = NetworkGraph::new();
        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);
        graph.add_node(4);
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        // 4 is isolated

        let connected = graph.connected_nodes(1);
        assert!(connected.contains(&1));
        assert!(connected.contains(&2));
        assert!(connected.contains(&3));
        assert!(!connected.contains(&4));
    }
}
