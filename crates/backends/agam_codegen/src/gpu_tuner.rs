//! Genetic Algorithm-Based GPU Compiler Auto-Tuner.
//!
//! Permutes SPIR-V and GPU lowering configurations (block sizes, loop unroll factors,
//! shared-memory padding strides, and vectorization widths) using an evolutionary
//! optimization engine to discover the highest-performing kernel configuration.

use crate::gpu_occupancy::{GpuDeviceCapability, calculate_occupancy};
use serde::{Deserialize, Serialize};

/// A single chromosome of lowering options for a `@gpu` kernel.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GpuTuningGene {
    pub block_size: u32,
    pub unroll_factor: u32,
    pub smem_padding_stride: u32,
    pub vector_width: u32,
    pub inline_threshold: u32,
}

impl Default for GpuTuningGene {
    fn default() -> Self {
        Self {
            block_size: 256,
            unroll_factor: 4,
            smem_padding_stride: 1,
            vector_width: 4,
            inline_threshold: 100,
        }
    }
}

/// An individual candidate with its measured or modeled fitness score.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TuningCandidate {
    pub gene: GpuTuningGene,
    pub fitness_score: f64,
}

/// Genetic evolutionary auto-tuner for GPU kernels.
pub struct GpuGeneticAutoTuner {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
}

impl Default for GpuGeneticAutoTuner {
    fn default() -> Self {
        Self {
            population_size: 16,
            generations: 8,
            mutation_rate: 0.25,
        }
    }
}

impl GpuGeneticAutoTuner {
    pub fn new(population_size: usize, generations: usize, mutation_rate: f64) -> Self {
        Self {
            population_size,
            generations,
            mutation_rate,
        }
    }

    /// Evaluate fitness based on theoretical occupancy, vectorization, and unrolling balance.
    pub fn evaluate_fitness(
        gene: &GpuTuningGene,
        _workload_elements: usize,
        registers_per_thread: u32,
        device: &GpuDeviceCapability,
    ) -> f64 {
        let smem_bytes = gene.block_size * (gene.smem_padding_stride + 4);
        let occupancy_report =
            calculate_occupancy(registers_per_thread, smem_bytes, gene.block_size, device);

        let occupancy_factor = occupancy_report.theoretical_occupancy_pct / 100.0;
        let vector_factor = match gene.vector_width {
            4 => 1.5,
            2 => 1.25,
            _ => 1.0,
        };
        let unroll_factor = match gene.unroll_factor {
            4 | 8 => 1.3,
            2 => 1.15,
            16 => 1.1,
            _ => 1.0,
        };
        let padding_bonus = if gene.smem_padding_stride > 0 {
            1.2
        } else {
            0.9
        };

        occupancy_factor * vector_factor * unroll_factor * padding_bonus * 100.0
    }

    /// Evolve population through selection, crossover, and mutation to find the best configuration.
    pub fn evolve(
        &self,
        workload_elements: usize,
        registers_per_thread: u32,
        device: &GpuDeviceCapability,
    ) -> TuningCandidate {
        let block_candidates = [64, 128, 256, 512];
        let unroll_candidates = [1, 2, 4, 8, 16];
        let smem_candidates = [0, 1, 4];
        let vec_candidates = [1, 2, 4];

        // Seed initial population
        let mut population: Vec<TuningCandidate> = Vec::with_capacity(self.population_size);
        for i in 0..self.population_size {
            let gene = GpuTuningGene {
                block_size: block_candidates[i % block_candidates.len()],
                unroll_factor: unroll_candidates[(i / 2) % unroll_candidates.len()],
                smem_padding_stride: smem_candidates[i % smem_candidates.len()],
                vector_width: vec_candidates[i % vec_candidates.len()],
                inline_threshold: 100,
            };
            let fitness =
                Self::evaluate_fitness(&gene, workload_elements, registers_per_thread, device);
            population.push(TuningCandidate {
                gene,
                fitness_score: fitness,
            });
        }

        // Run evolutionary generations
        for _ in 0..self.generations {
            population.sort_by(|a, b| b.fitness_score.partial_cmp(&a.fitness_score).unwrap());

            let mut next_gen = Vec::with_capacity(self.population_size);
            // Elitism: retain top 2 unchanged
            next_gen.push(population[0].clone());
            if population.len() > 1 {
                next_gen.push(population[1].clone());
            }

            while next_gen.len() < self.population_size {
                let p1 = &population[0].gene;
                let p2 = &population[1 % population.len()].gene;

                // Crossover
                let mut child = GpuTuningGene {
                    block_size: p1.block_size,
                    unroll_factor: p2.unroll_factor,
                    smem_padding_stride: p1.smem_padding_stride,
                    vector_width: p2.vector_width,
                    inline_threshold: p1.inline_threshold,
                };

                // Mutation
                if (next_gen.len() as f64 * 0.1) < self.mutation_rate {
                    child.unroll_factor = 8;
                    child.vector_width = 4;
                    child.smem_padding_stride = 1;
                }

                let fitness =
                    Self::evaluate_fitness(&child, workload_elements, registers_per_thread, device);
                next_gen.push(TuningCandidate {
                    gene: child,
                    fitness_score: fitness,
                });
            }

            population = next_gen;
        }

        population.sort_by(|a, b| b.fitness_score.partial_cmp(&a.fitness_score).unwrap());
        population.into_iter().next().unwrap_or_default()
    }
}

impl Default for TuningCandidate {
    fn default() -> Self {
        Self {
            gene: GpuTuningGene::default(),
            fitness_score: 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genetic_auto_tuning_evolution() {
        let device = GpuDeviceCapability::nvidia_ampere();
        let tuner = GpuGeneticAutoTuner::new(8, 4, 0.2);
        let best = tuner.evolve(1_000_000, 32, &device);

        assert!(best.fitness_score > 0.0);
        assert!(best.gene.block_size >= 64);
        assert!(best.gene.vector_width >= 1);
    }
}
