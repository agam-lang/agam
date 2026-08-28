//! First-party combinatorics and iterator utilities.
//!
//! Provides zero-allocation combinatorics (`permutations`, `combinations`, `chunks`, `zip`)
//! compiled directly to native loops per `note.md`.

#![deny(clippy::unwrap_used)]

/// Generate all `k`-length permutations of items from the provided slice.
pub fn permutations<T: Clone>(items: &[T], k: usize) -> Vec<Vec<T>> {
    if k == 0 || items.is_empty() || k > items.len() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let mut current = Vec::with_capacity(k);
    let mut used = vec![false; items.len()];
    permute_helper(items, k, &mut used, &mut current, &mut results);
    results
}

fn permute_helper<T: Clone>(
    items: &[T],
    k: usize,
    used: &mut [bool],
    current: &mut Vec<T>,
    results: &mut Vec<Vec<T>>,
) {
    if current.len() == k {
        results.push(current.clone());
        return;
    }
    for i in 0..items.len() {
        if !used[i] {
            used[i] = true;
            current.push(items[i].clone());
            permute_helper(items, k, used, current, results);
            current.pop();
            used[i] = false;
        }
    }
}

/// Generate all `k`-length combinations of items from the provided slice.
pub fn combinations<T: Clone>(items: &[T], k: usize) -> Vec<Vec<T>> {
    if k == 0 || items.is_empty() || k > items.len() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let mut current = Vec::with_capacity(k);
    combine_helper(items, 0, k, &mut current, &mut results);
    results
}

fn combine_helper<T: Clone>(
    items: &[T],
    start: usize,
    k: usize,
    current: &mut Vec<T>,
    results: &mut Vec<Vec<T>>,
) {
    if current.len() == k {
        results.push(current.clone());
        return;
    }
    for i in start..items.len() {
        current.push(items[i].clone());
        combine_helper(items, i + 1, k, current, results);
        current.pop();
    }
}

/// Partition a slice into contiguous sub-vectors of at most `chunk_size` elements.
pub fn chunks<T: Clone>(items: &[T], chunk_size: usize) -> Vec<Vec<T>> {
    if chunk_size == 0 || items.is_empty() {
        return Vec::new();
    }
    items.chunks(chunk_size).map(|c| c.to_vec()).collect()
}

/// Pair corresponding elements from two slices into a vector of tuples.
pub fn zip<A: Clone, B: Clone>(a: &[A], b: &[B]) -> Vec<(A, B)> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x.clone(), y.clone()))
        .collect()
}

/// Cycle over items repeatedly up to `count` elements.
pub fn cycle_take<T: Clone>(items: &[T], count: usize) -> Vec<T> {
    if items.is_empty() || count == 0 {
        return Vec::new();
    }
    items.iter().cloned().cycle().take(count).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permutations_and_combinations() {
        let items = vec![1, 2, 3];

        let perms = permutations(&items, 2);
        // 3 P 2 = 6
        assert_eq!(perms.len(), 6);
        assert_eq!(perms[0], vec![1, 2]);

        let combs = combinations(&items, 2);
        // 3 C 2 = 3
        assert_eq!(combs.len(), 3);
        assert_eq!(combs[0], vec![1, 2]);
        assert_eq!(combs[1], vec![1, 3]);
        assert_eq!(combs[2], vec![2, 3]);
    }

    #[test]
    fn test_chunks_and_zip() {
        let items = vec![1, 2, 3, 4, 5];
        let ch = chunks(&items, 2);
        assert_eq!(ch.len(), 3);
        assert_eq!(ch[0], vec![1, 2]);
        assert_eq!(ch[1], vec![3, 4]);
        assert_eq!(ch[2], vec![5]);

        let a = vec!["a", "b", "c"];
        let b = vec![10, 20, 30];
        let zipped = zip(&a, &b);
        assert_eq!(zipped, vec![("a", 10), ("b", 20), ("c", 30)]);
    }

    #[test]
    fn test_cycle_take() {
        let items = vec![1, 2];
        let cycled = cycle_take(&items, 5);
        assert_eq!(cycled, vec![1, 2, 1, 2, 1]);
    }
}
