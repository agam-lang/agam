//! Typed DataFrame — tabular data with named columns.
//!
//! Like Pandas but compiled and type-safe. Uses columnar storage
//! for cache-friendly column-wise operations (filter, group_by, sort).
//!
//! ## Performance
//! - Columnar layout: each column is a contiguous `Vec<f64>` or `Vec<String>`
//! - Filter/map operations iterate contiguous memory
//! - Sort uses the standard library's pattern-defeating quicksort

/// A column of homogeneous data.
#[derive(Debug, Clone)]
pub enum Column {
    Float(Vec<f64>),
    Int(Vec<i64>),
    Str(Vec<String>),
    Bool(Vec<bool>),
}

impl Column {
    pub fn len(&self) -> usize {
        match self {
            Column::Float(v) => v.len(),
            Column::Int(v) => v.len(),
            Column::Str(v) => v.len(),
            Column::Bool(v) => v.len(),
        }
    }

    pub fn as_float(&self) -> Option<&[f64]> {
        if let Column::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_int(&self) -> Option<&[i64]> {
        if let Column::Int(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&[String]> {
        if let Column::Str(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<&[bool]> {
        if let Column::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Sum (for numeric columns).
    pub fn sum_float(&self) -> Option<f64> {
        self.as_float().map(|v| v.iter().sum())
    }

    pub fn sum_int(&self) -> Option<i64> {
        self.as_int().map(|v| v.iter().sum())
    }

    /// Mean (for float columns).
    pub fn mean(&self) -> Option<f64> {
        self.as_float()
            .map(|v| v.iter().sum::<f64>() / v.len() as f64)
    }

    /// Min/max for float columns.
    pub fn min_float(&self) -> Option<f64> {
        self.as_float()
            .and_then(|v| v.iter().cloned().reduce(f64::min))
    }

    pub fn max_float(&self) -> Option<f64> {
        self.as_float()
            .and_then(|v| v.iter().cloned().reduce(f64::max))
    }
}

/// A tabular DataFrame with named columns (columnar storage).
#[derive(Debug, Clone)]
pub struct DataFrame {
    pub columns: Vec<(String, Column)>,
}

impl DataFrame {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
        }
    }

    /// Create from named columns.
    pub fn from_columns(cols: Vec<(String, Column)>) -> Self {
        // Verify all columns have same length
        if let Some(first) = cols.first() {
            let len = first.1.len();
            for (name, col) in &cols {
                assert_eq!(col.len(), len, "column '{}' has wrong length", name);
            }
        }
        Self { columns: cols }
    }

    /// Number of rows.
    pub fn nrows(&self) -> usize {
        self.columns.first().map(|(_, c)| c.len()).unwrap_or(0)
    }

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        self.columns.len()
    }

    /// Add a column.
    pub fn add_column(&mut self, name: String, col: Column) {
        if !self.columns.is_empty() {
            assert_eq!(col.len(), self.nrows(), "column length must match");
        }
        self.columns.push((name, col));
    }

    /// Get a column by name.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    /// Filter rows by a boolean mask.
    pub fn filter(&self, mask: &[bool]) -> DataFrame {
        assert_eq!(mask.len(), self.nrows());
        let columns = self
            .columns
            .iter()
            .map(|(name, col)| {
                let filtered = match col {
                    Column::Float(v) => Column::Float(
                        v.iter()
                            .zip(mask)
                            .filter(|(_, m)| **m)
                            .map(|(x, _)| *x)
                            .collect(),
                    ),
                    Column::Int(v) => Column::Int(
                        v.iter()
                            .zip(mask)
                            .filter(|(_, m)| **m)
                            .map(|(x, _)| *x)
                            .collect(),
                    ),
                    Column::Str(v) => Column::Str(
                        v.iter()
                            .zip(mask)
                            .filter(|(_, m)| **m)
                            .map(|(x, _)| x.clone())
                            .collect(),
                    ),
                    Column::Bool(v) => Column::Bool(
                        v.iter()
                            .zip(mask)
                            .filter(|(_, m)| **m)
                            .map(|(x, _)| *x)
                            .collect(),
                    ),
                };
                (name.clone(), filtered)
            })
            .collect();
        DataFrame { columns }
    }

    /// Select specific columns by name.
    pub fn select(&self, names: &[&str]) -> DataFrame {
        let columns = names
            .iter()
            .filter_map(|name| {
                self.columns
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(n, c)| (n.clone(), c.clone()))
            })
            .collect();
        DataFrame { columns }
    }

    /// Sort by a float column (ascending).
    pub fn sort_by(&self, col_name: &str) -> DataFrame {
        let col = self.column(col_name).expect("column not found");
        let indices: Vec<usize> = match col {
            Column::Float(v) => {
                let mut idx: Vec<usize> = (0..v.len()).collect();
                idx.sort_by(|a, b| v[*a].partial_cmp(&v[*b]).unwrap());
                idx
            }
            Column::Int(v) => {
                let mut idx: Vec<usize> = (0..v.len()).collect();
                idx.sort_by_key(|i| v[*i]);
                idx
            }
            _ => panic!("sort_by requires numeric column"),
        };
        self.reindex(&indices)
    }

    /// Reindex the DataFrame by a permutation.
    fn reindex(&self, indices: &[usize]) -> DataFrame {
        let columns = self
            .columns
            .iter()
            .map(|(name, col)| {
                let reindexed = match col {
                    Column::Float(v) => Column::Float(indices.iter().map(|i| v[*i]).collect()),
                    Column::Int(v) => Column::Int(indices.iter().map(|i| v[*i]).collect()),
                    Column::Str(v) => Column::Str(indices.iter().map(|i| v[*i].clone()).collect()),
                    Column::Bool(v) => Column::Bool(indices.iter().map(|i| v[*i]).collect()),
                };
                (name.clone(), reindexed)
            })
            .collect();
        DataFrame { columns }
    }

    /// Head: first n rows.
    pub fn head(&self, n: usize) -> DataFrame {
        let n = n.min(self.nrows());
        let columns = self
            .columns
            .iter()
            .map(|(name, col)| {
                let sliced = match col {
                    Column::Float(v) => Column::Float(v[..n].to_vec()),
                    Column::Int(v) => Column::Int(v[..n].to_vec()),
                    Column::Str(v) => Column::Str(v[..n].to_vec()),
                    Column::Bool(v) => Column::Bool(v[..n].to_vec()),
                };
                (name.clone(), sliced)
            })
            .collect();
        DataFrame { columns }
    }

    /// Tail: last n rows.
    pub fn tail(&self, n: usize) -> DataFrame {
        let total = self.nrows();
        let start = total.saturating_sub(n);
        let columns = self
            .columns
            .iter()
            .map(|(name, col)| {
                let sliced = match col {
                    Column::Float(v) => Column::Float(v[start..].to_vec()),
                    Column::Int(v) => Column::Int(v[start..].to_vec()),
                    Column::Str(v) => Column::Str(v[start..].to_vec()),
                    Column::Bool(v) => Column::Bool(v[start..].to_vec()),
                };
                (name.clone(), sliced)
            })
            .collect();
        DataFrame { columns }
    }

    /// Describe: summary statistics for numeric columns.
    pub fn describe(&self) -> Vec<(String, f64, f64, f64, f64)> {
        let mut results = Vec::new();
        for (name, col) in &self.columns {
            if let Some(data) = col.as_float() {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let var =
                    data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / data.len() as f64;
                results.push((name.clone(), mean, var.sqrt(), min, max));
            }
        }
        results
    }

    /// Apply a function to a float column, producing a new column.
    pub fn map_column<F: Fn(f64) -> f64>(&self, col_name: &str, f: F) -> Column {
        let col = self.column(col_name).expect("column not found");
        let data = col.as_float().expect("map_column requires float column");
        Column::Float(data.iter().map(|x| f(*x)).collect())
    }

    /// Group by an integer column and aggregate a float column (sum).
    pub fn group_by_sum(&self, group_col: &str, value_col: &str) -> Vec<(i64, f64)> {
        let groups = self.column(group_col).unwrap().as_int().unwrap();
        let values = self.column(value_col).unwrap().as_float().unwrap();

        let mut map: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
        for (g, v) in groups.iter().zip(values) {
            *map.entry(*g).or_default() += *v;
        }
        let mut result: Vec<(i64, f64)> = map.into_iter().collect();
        result.sort_by_key(|(k, _)| *k);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_df() -> DataFrame {
        DataFrame::from_columns(vec![
            (
                "name".into(),
                Column::Str(vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ]),
            ),
            ("age".into(), Column::Float(vec![30.0, 25.0, 35.0, 28.0])),
            ("score".into(), Column::Float(vec![90.0, 85.0, 92.0, 88.0])),
            ("group".into(), Column::Int(vec![1, 2, 1, 2])),
        ])
    }

    #[test]
    fn test_df_shape() {
        let df = sample_df();
        assert_eq!(df.nrows(), 4);
        assert_eq!(df.ncols(), 4);
    }

    #[test]
    fn test_df_column_access() {
        let df = sample_df();
        let age = df.column("age").unwrap();
        assert_eq!(age.mean().unwrap(), 29.5);
    }

    #[test]
    fn test_df_filter() {
        let df = sample_df();
        let ages = df.column("age").unwrap().as_float().unwrap();
        let mask: Vec<bool> = ages.iter().map(|a| *a > 27.0).collect();
        let filtered = df.filter(&mask);
        assert_eq!(filtered.nrows(), 3);
    }

    #[test]
    fn test_df_select() {
        let df = sample_df();
        let selected = df.select(&["name", "score"]);
        assert_eq!(selected.ncols(), 2);
    }

    #[test]
    fn test_df_sort() {
        let df = sample_df();
        let sorted = df.sort_by("age");
        let ages = sorted.column("age").unwrap().as_float().unwrap();
        assert_eq!(ages[0], 25.0);
        assert_eq!(ages[3], 35.0);
    }

    #[test]
    fn test_df_head_tail() {
        let df = sample_df();
        let h = df.head(2);
        assert_eq!(h.nrows(), 2);
        let t = df.tail(1);
        assert_eq!(t.nrows(), 1);
    }

    #[test]
    fn test_df_describe() {
        let df = sample_df();
        let desc = df.describe();
        assert_eq!(desc.len(), 2); // age + score (float columns)
        let (name, mean, _, _, _) = &desc[0];
        assert_eq!(name, "age");
        assert!((mean - 29.5).abs() < 1e-10);
    }

    #[test]
    fn test_df_map_column() {
        let df = sample_df();
        let doubled = df.map_column("score", |x| x * 2.0);
        assert_eq!(doubled.as_float().unwrap()[0], 180.0);
    }

    #[test]
    fn test_df_group_by_sum() {
        let df = sample_df();
        let grouped = df.group_by_sum("group", "score");
        assert_eq!(grouped.len(), 2);
        assert!((grouped[0].1 - 182.0).abs() < 1e-10); // group 1: 90+92
        assert!((grouped[1].1 - 173.0).abs() < 1e-10); // group 2: 85+88
    }

    #[test]
    fn test_column_stats() {
        let col = Column::Float(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(col.sum_float(), Some(15.0));
        assert_eq!(col.mean(), Some(3.0));
        assert_eq!(col.min_float(), Some(1.0));
        assert_eq!(col.max_float(), Some(5.0));
    }
}
