//! High-Performance Collections & Graph Data Structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-throughput circular ring buffer with fixed capacity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FastRingBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    tail: usize,
    size: usize,
}

impl<T: Clone> FastRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be positive");
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }
        Self {
            buffer,
            capacity,
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        let old = self.buffer[self.tail].take();
        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;

        if self.size < self.capacity {
            self.size += 1;
            None
        } else {
            self.head = (self.head + 1) % self.capacity;
            old
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        let item = self.buffer[self.head].take();
        self.head = (self.head + 1) % self.capacity;
        self.size -= 1;
        item
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Compact directed graph with adjacency representation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompactGraph {
    pub node_count: usize,
    pub edges: HashMap<usize, Vec<(usize, f64)>>, // u -> [(v, weight)]
}

impl CompactGraph {
    pub fn new(node_count: usize) -> Self {
        Self {
            node_count,
            edges: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, u: usize, v: usize, weight: f64) {
        assert!(
            u < self.node_count && v < self.node_count,
            "Node index out of bounds"
        );
        self.edges.entry(u).or_default().push((v, weight));
    }

    /// Single-source shortest path using Dijkstra's algorithm.
    pub fn shortest_path(&self, start: usize, target: usize) -> Option<(f64, Vec<usize>)> {
        let mut dist: Vec<f64> = vec![f64::INFINITY; self.node_count];
        let mut prev: Vec<Option<usize>> = vec![None; self.node_count];
        let mut visited: Vec<bool> = vec![false; self.node_count];

        dist[start] = 0.0;

        for _ in 0..self.node_count {
            let mut u = None;
            let mut min_d = f64::INFINITY;
            for i in 0..self.node_count {
                if !visited[i] && dist[i] < min_d {
                    min_d = dist[i];
                    u = Some(i);
                }
            }

            let u = match u {
                Some(node) => node,
                None => break,
            };

            if u == target {
                break;
            }

            visited[u] = true;

            if let Some(neighbors) = self.edges.get(&u) {
                for (v, weight) in neighbors {
                    if dist[u] + weight < dist[*v] {
                        dist[*v] = dist[u] + weight;
                        prev[*v] = Some(u);
                    }
                }
            }
        }

        if dist[target].is_infinite() {
            return None;
        }

        let mut path = Vec::new();
        let mut curr = Some(target);
        while let Some(node) = curr {
            path.push(node);
            curr = prev[node];
        }
        path.reverse();

        Some((dist[target], path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_ring_buffer() {
        let mut rb = FastRingBuffer::new(3);
        assert_eq!(rb.push(10), None);
        assert_eq!(rb.push(20), None);
        assert_eq!(rb.push(30), None);
        // Overwriting oldest
        assert_eq!(rb.push(40), Some(10));
        assert_eq!(rb.pop(), Some(20));
        assert_eq!(rb.pop(), Some(30));
        assert_eq!(rb.pop(), Some(40));
        assert_eq!(rb.pop(), None);
    }

    #[test]
    fn test_compact_graph_shortest_path() {
        let mut g = CompactGraph::new(4);
        g.add_edge(0, 1, 1.0);
        g.add_edge(1, 2, 2.0);
        g.add_edge(0, 2, 4.0);
        g.add_edge(2, 3, 1.0);

        let (dist, path) = g.shortest_path(0, 3).expect("path exists");
        assert_eq!(dist, 4.0); // 0 -> 1 (1) -> 2 (2) -> 3 (1) = 4
        assert_eq!(path, vec![0, 1, 2, 3]);
    }
}
