use serde::{Deserialize, Serialize};

/// Target SIMD feature set detected on the host or requested by compiler directives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimdFeatureSet {
    pub sse2: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512f: bool,
    pub neon: bool,
}

impl SimdFeatureSet {
    pub fn best_tier(&self) -> SimdTargetTier {
        if self.avx512f {
            SimdTargetTier::Avx512
        } else if self.avx2 || self.avx {
            SimdTargetTier::Avx2
        } else if self.sse4_2 || self.sse2 {
            SimdTargetTier::Sse42
        } else if self.neon {
            SimdTargetTier::Neon
        } else {
            SimdTargetTier::Scalar
        }
    }
}

/// SIMD target tier classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SimdTargetTier {
    Scalar = 0,
    Sse42 = 1,
    Neon = 2,
    Avx2 = 3,
    Avx512 = 4,
}

/// Struct field specification for cache-aware memory layout layout optimization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub size_bytes: usize,
    pub align_bytes: usize,
}

/// Report containing field ordering, padding, and cache alignment properties.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructLayoutReport {
    pub optimized_field_order: Vec<String>,
    pub total_size_bytes: usize,
    pub padding_bytes: usize,
    pub cache_lines_spanned: usize,
}

/// Cache-oblivious struct layout optimizer.
pub struct StructLayoutOptimizer;

impl StructLayoutOptimizer {
    /// Optimize field ordering to eliminate alignment padding and compute cache line footprint.
    pub fn optimize_layout(fields: &[StructField], cache_line_size: usize) -> StructLayoutReport {
        let mut sorted_fields = fields.to_vec();
        // Sort descending by alignment first, then descending by size (Greedy Packing)
        sorted_fields.sort_by(|a, b| {
            b.align_bytes
                .cmp(&a.align_bytes)
                .then_with(|| b.size_bytes.cmp(&a.size_bytes))
        });

        let mut offset = 0;
        let mut max_align = 1;
        let mut optimized_order = Vec::with_capacity(sorted_fields.len());

        for field in &sorted_fields {
            max_align = max_align.max(field.align_bytes);
            let rem = offset % field.align_bytes;
            if rem != 0 {
                offset += field.align_bytes - rem;
            }
            optimized_order.push(field.name.clone());
            offset += field.size_bytes;
        }

        // Tail padding to align total struct size to max_align
        let tail_rem = offset % max_align;
        if tail_rem != 0 {
            offset += max_align - tail_rem;
        }

        let raw_sum: usize = fields.iter().map(|f| f.size_bytes).sum();
        let padding = offset - raw_sum;
        let line_size = if cache_line_size > 0 {
            cache_line_size
        } else {
            64
        };
        let cache_lines = offset.div_ceil(line_size);

        StructLayoutReport {
            optimized_field_order: optimized_order,
            total_size_bytes: offset,
            padding_bytes: padding,
            cache_lines_spanned: cache_lines,
        }
    }
}

/// Array-of-Structs (AoS) to Struct-of-Arrays (SoA) layout transform generator.
pub struct AosToSoaTransform;

impl AosToSoaTransform {
    /// Generate SoA parallel vector struct definition from AoS fields.
    pub fn generate_soa_struct_definition(soa_name: &str, fields: &[StructField]) -> String {
        let mut out = format!("struct {} {{\n", soa_name);
        for field in fields {
            out.push_str(&format!(
                "    {}: Vec<{}>,\n",
                field.name,
                match field.size_bytes {
                    1 => "u8",
                    2 => "u16",
                    4 => "f32",
                    8 => "f64",
                    16 => "u128",
                    _ => "u8",
                }
            ));
        }
        out.push_str("}\n");
        out
    }
}

/// Dynamic SIMD multi-versioning dispatch resolver.
pub struct SimdMultiVersionDispatcher;

impl SimdMultiVersionDispatcher {
    /// Resolve function symbol suffix for runtime multi-version dispatch.
    pub fn resolve_variant_symbol(base_name: &str, caps: &SimdFeatureSet) -> String {
        match caps.best_tier() {
            SimdTargetTier::Avx512 => format!("{base_name}__avx512"),
            SimdTargetTier::Avx2 => format!("{base_name}__avx2"),
            SimdTargetTier::Sse42 => format!("{base_name}__sse42"),
            SimdTargetTier::Neon => format!("{base_name}__neon"),
            SimdTargetTier::Scalar => format!("{base_name}__scalar"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_layout_optimization_eliminates_holes() {
        // Poorly arranged struct: u8, f64, u8, f64 (would take 32 bytes unoptimized with padding)
        let fields = vec![
            StructField {
                name: "a".into(),
                size_bytes: 1,
                align_bytes: 1,
            },
            StructField {
                name: "b".into(),
                size_bytes: 8,
                align_bytes: 8,
            },
            StructField {
                name: "c".into(),
                size_bytes: 1,
                align_bytes: 1,
            },
            StructField {
                name: "d".into(),
                size_bytes: 8,
                align_bytes: 8,
            },
        ];

        let report = StructLayoutOptimizer::optimize_layout(&fields, 64);
        // Optimized order puts 8-byte aligned fields first: b, d, a, c (18 bytes padded to 24)
        assert_eq!(report.total_size_bytes, 24);
        assert_eq!(report.padding_bytes, 6);
        assert_eq!(report.cache_lines_spanned, 1);
        assert_eq!(report.optimized_field_order[0], "b");
        assert_eq!(report.optimized_field_order[1], "d");
    }

    #[test]
    fn test_aos_to_soa_struct_generation() {
        let fields = vec![
            StructField {
                name: "pos_x".into(),
                size_bytes: 4,
                align_bytes: 4,
            },
            StructField {
                name: "pos_y".into(),
                size_bytes: 4,
                align_bytes: 4,
            },
        ];

        let code = AosToSoaTransform::generate_soa_struct_definition("ParticleSoa", &fields);
        assert!(code.contains("struct ParticleSoa"));
        assert!(code.contains("pos_x: Vec<f32>"));
        assert!(code.contains("pos_y: Vec<f32>"));
    }

    #[test]
    fn test_simd_multi_version_symbol_resolution() {
        let caps = SimdFeatureSet {
            sse2: true,
            sse4_2: true,
            avx: true,
            avx2: true,
            avx512f: false,
            neon: false,
        };

        let sym = SimdMultiVersionDispatcher::resolve_variant_symbol("vector_dot", &caps);
        assert_eq!(sym, "vector_dot__avx2");
    }
}
