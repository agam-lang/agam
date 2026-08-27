//! High-Performance Collections & Graph Data Structures.
//!
//! Exposes contiguous memory vectors, circular ring buffers, flat SwissTable hash maps,
//! deterministic B-Tree ordered maps, and compact graphs.

#![deny(clippy::unwrap_used)]

use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;

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
        let cap = if capacity == 0 { 1 } else { capacity };
        let mut buffer = Vec::with_capacity(cap);
        for _ in 0..cap {
            buffer.push(None);
        }
        Self {
            buffer,
            capacity: cap,
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

/// Contiguous fast growable vector with capacity pre-allocation and binary search.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastVec<T> {
    inner: Vec<T>,
}

impl<T> FastVec<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }

    pub fn get_slice(&self, start: usize, end: usize) -> Option<&[T]> {
        if start <= end && end <= self.inner.len() {
            Some(&self.inner[start..end])
        } else {
            None
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<T> {
        if index < self.inner.len() {
            Some(self.inner.swap_remove(index))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.inner
    }
}

impl<T: Ord> FastVec<T> {
    /// Binary search leftmost insertion position (Python `bisect_left`).
    pub fn bisect_left(&self, value: &T) -> usize {
        match self.inner.binary_search(value) {
            Ok(mut idx) => {
                while idx > 0 && &self.inner[idx - 1] == value {
                    idx -= 1;
                }
                idx
            }
            Err(idx) => idx,
        }
    }

    /// Binary search rightmost insertion position (Python `bisect_right`).
    pub fn bisect_right(&self, value: &T) -> usize {
        match self.inner.binary_search(value) {
            Ok(mut idx) => {
                while idx + 1 < self.inner.len() && &self.inner[idx + 1] == value {
                    idx += 1;
                }
                idx + 1
            }
            Err(idx) => idx,
        }
    }

    pub fn sort(&mut self) {
        self.inner.sort();
    }
}

impl<T> From<Vec<T>> for FastVec<T> {
    fn from(inner: Vec<T>) -> Self {
        Self { inner }
    }
}

/// SwissTable-backed high-throughput hash map with safe non-panicking lookup.
#[derive(Clone, Debug, Default)]
pub struct FastHashMap<K, V> {
    inner: HashMap<K, V>,
}

impl<K: Eq + Hash, V: PartialEq> PartialEq for FastHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K: Eq + Hash, V: Eq> Eq for FastHashMap<K, V> {}

impl<K: Hash + Eq, V> FastHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_mut(key)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }
}

/// Hash-based set backed by `FastHashMap`.
#[derive(Clone, Debug, Default)]
pub struct FastHashSet<T> {
    inner: HashSet<T>,
}

impl<T: Eq + Hash> PartialEq for FastHashSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq + Hash> Eq for FastHashSet<T> {}

impl<T: Hash + Eq> FastHashSet<T> {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashSet::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }

    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.remove(value)
    }

    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains(value)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Frequency analysis counter map (`Counter.from(iter)` from `note.md`).
#[derive(Clone, Debug, Default)]
pub struct Counter<T: Hash + Eq> {
    counts: HashMap<T, usize>,
}

impl<T: Hash + Eq + Clone> Counter<T> {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut counter = Self::new();
        for item in iter {
            counter.increment(item);
        }
        counter
    }

    pub fn increment(&mut self, item: T) {
        *self.counts.entry(item).or_insert(0) += 1;
    }

    pub fn count<Q>(&self, item: &Q) -> usize
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.counts.get(item).copied().unwrap_or(0)
    }

    pub fn most_common(&self, n: usize) -> Vec<(T, usize)> {
        let mut items: Vec<(T, usize)> = self.counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(n);
        items
    }
}

/// Deterministic ordered map backed by balanced B-Tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrderedMap<K, V> {
    inner: BTreeMap<K, V>,
}

impl<K: Ord, V> OrderedMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.get(key)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.get_mut(key)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.remove(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.contains_key(key)
    }

    pub fn min_key(&self) -> Option<&K> {
        self.inner.keys().next()
    }

    pub fn max_key(&self) -> Option<&K> {
        self.inner.keys().next_back()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
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

    pub fn add_edge(&mut self, u: usize, v: usize, weight: f64) -> bool {
        if u >= self.node_count || v >= self.node_count {
            return false;
        }
        self.edges.entry(u).or_default().push((v, weight));
        true
    }

    /// Single-source shortest path using Dijkstra's algorithm.
    pub fn shortest_path(&self, start: usize, target: usize) -> Option<(f64, Vec<usize>)> {
        if start >= self.node_count || target >= self.node_count {
            return None;
        }

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
    fn test_fast_vec_and_bisect() {
        let mut v = FastVec::with_capacity(4);
        v.push(10);
        v.push(20);
        v.push(20);
        v.push(30);

        assert_eq!(v.len(), 4);
        assert_eq!(v.get_slice(1, 3), Some(&[20, 20][..]));
        assert_eq!(v.bisect_left(&20), 1);
        assert_eq!(v.bisect_right(&20), 3);
        assert_eq!(v.bisect_left(&25), 3);
    }

    #[test]
    fn test_counter_and_hash_map() {
        let items = vec!["apple", "banana", "apple", "cherry", "apple", "banana"];
        let counter = Counter::from_iter(items);
        assert_eq!(counter.count("apple"), 3);
        assert_eq!(counter.count("banana"), 2);
        assert_eq!(counter.count("cherry"), 1);
        assert_eq!(counter.count("durian"), 0);

        let top2 = counter.most_common(2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0], ("apple", 3));
        assert_eq!(top2[1], ("banana", 2));
    }

    #[test]
    fn test_ordered_map_range() {
        let mut map = OrderedMap::new();
        map.insert(3, "three");
        map.insert(1, "one");
        map.insert(2, "two");

        assert_eq!(map.min_key(), Some(&1));
        assert_eq!(map.max_key(), Some(&3));
        assert_eq!(map.get(&2), Some(&"two"));
    }

    #[test]
    fn test_compact_graph_shortest_path() {
        let mut g = CompactGraph::new(4);
        assert!(g.add_edge(0, 1, 1.0));
        assert!(g.add_edge(1, 2, 2.0));
        assert!(g.add_edge(0, 2, 4.0));
        assert!(g.add_edge(2, 3, 1.0));

        let res = g.shortest_path(0, 3);
        assert!(res.is_some());
        if let Some((dist, path)) = res {
            assert_eq!(dist, 4.0);
            assert_eq!(path, vec![0, 1, 2, 3]);
        }
    }
}
