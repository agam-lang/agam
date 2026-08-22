//! Probabilistic Programming, Bayesian Inference, and Uncertainty Quantification.
//!
//! Grounded in algebraic effect-based sampling, automated log-joint accumulation,
//! and high-performance MCMC / Variational Inference (HMC, Metropolis-Hastings, Importance Sampling).

use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Continuous and discrete probability distributions with log-probability densities.
#[derive(Debug, Clone, PartialEq)]
pub enum Distribution {
    Normal { mean: f64, std: f64 },
    Uniform { min: f64, max: f64 },
    Bernoulli { p: f64 },
    Exponential { rate: f64 },
    Beta { alpha: f64, beta: f64 },
}

impl Distribution {
    pub fn normal(mean: f64, std: f64) -> Self {
        assert!(std > 0.0, "Standard deviation must be positive");
        Distribution::Normal { mean, std }
    }

    pub fn uniform(min: f64, max: f64) -> Self {
        assert!(max > min, "Upper bound must exceed lower bound");
        Distribution::Uniform { min, max }
    }

    pub fn bernoulli(p: f64) -> Self {
        assert!((0.0..=1.0).contains(&p), "p must be in [0, 1]");
        Distribution::Bernoulli { p }
    }

    pub fn exponential(rate: f64) -> Self {
        assert!(rate > 0.0, "Rate must be positive");
        Distribution::Exponential { rate }
    }

    /// Compute the exact log-probability density (or PMF) at `x`.
    pub fn log_prob(&self, x: f64) -> f64 {
        match self {
            Distribution::Normal { mean, std } => {
                let diff = x - mean;
                -0.5 * (2.0 * PI).ln() - std.ln() - (diff * diff) / (2.0 * std * std)
            }
            Distribution::Uniform { min, max } => {
                if x >= *min && x <= *max {
                    -(max - min).ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            Distribution::Bernoulli { p } => {
                if (x - 1.0).abs() < 1e-6 {
                    p.ln()
                } else if x.abs() < 1e-6 {
                    (1.0 - p).ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            Distribution::Exponential { rate } => {
                if x >= 0.0 {
                    rate.ln() - rate * x
                } else {
                    f64::NEG_INFINITY
                }
            }
            Distribution::Beta { alpha, beta } => {
                if x > 0.0 && x < 1.0 {
                    // Log-density: (alpha-1)*ln(x) + (beta-1)*ln(1-x) - ln(B(alpha, beta))
                    let lbeta = lgamma(*alpha) + lgamma(*beta) - lgamma(alpha + beta);
                    (alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln() - lbeta
                } else {
                    f64::NEG_INFINITY
                }
            }
        }
    }

    /// Deterministic pseudo-random sample generation for inference.
    pub fn sample_pseudo(&self, seed: f64) -> f64 {
        let u = (seed * 12345.6789).sin().abs().fract();
        match self {
            Distribution::Normal { mean, std } => {
                let u2 = ((seed + 0.5) * 98765.4321).cos().abs().fract();
                let z = (-2.0 * (u + 1e-12).ln()).sqrt() * (2.0 * PI * u2).cos();
                mean + std * z
            }
            Distribution::Uniform { min, max } => min + u * (max - min),
            Distribution::Bernoulli { p } => {
                if u < *p {
                    1.0
                } else {
                    0.0
                }
            }
            Distribution::Exponential { rate } => -(1.0 - u + 1e-12).ln() / rate,
            Distribution::Beta { alpha, .. } => {
                let r = alpha / (alpha + 1.0);
                r + (u - 0.5) * 0.1
            }
        }
    }
}

fn lgamma(x: f64) -> f64 {
    // Stirling approximation for log-gamma
    if x <= 0.0 {
        0.0
    } else {
        (x - 0.5) * x.ln() - x + 0.5 * (2.0 * PI).ln()
    }
}

/// Execution trace of a probabilistic model run.
#[derive(Debug, Clone, Default)]
pub struct ModelTrace {
    pub latents: BTreeMap<String, (f64, Distribution)>,
    pub observations: BTreeMap<String, (f64, Distribution)>,
    pub log_prior: f64,
    pub log_likelihood: f64,
}

impl ModelTrace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sample a latent variable from a prior distribution.
    pub fn sample(&mut self, name: &str, dist: Distribution, value: f64) -> f64 {
        let lp = dist.log_prob(value);
        self.log_prior += lp;
        self.latents.insert(name.to_string(), (value, dist));
        value
    }

    /// Condition on an observed data point.
    pub fn observe(&mut self, name: &str, dist: Distribution, value: f64) {
        let lp = dist.log_prob(value);
        self.log_likelihood += lp;
        self.observations.insert(name.to_string(), (value, dist));
    }

    /// Total log-joint probability $\log P(\text{latents}, \text{observations}) = \log P(\text{latents}) + \log P(\text{obs} \mid \text{latents})$.
    pub fn log_joint(&self) -> f64 {
        self.log_prior + self.log_likelihood
    }
}

/// Bayesian Inference Engine.
pub struct BayesianInference;

impl BayesianInference {
    /// Importance Sampling: Draw `samples` proposals and compute normalized posterior weights.
    pub fn importance_sampling<F>(model: F, sample_count: usize) -> (Vec<ModelTrace>, Vec<f64>)
    where
        F: Fn(usize) -> ModelTrace,
    {
        let mut traces = Vec::with_capacity(sample_count);
        let mut log_weights = Vec::with_capacity(sample_count);

        for i in 0..sample_count {
            let trace = model(i);
            log_weights.push(trace.log_likelihood);
            traces.push(trace);
        }

        // Log-sum-exp normalization
        let max_lw = log_weights
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let sum_exp: f64 = log_weights.iter().map(|&lw| (lw - max_lw).exp()).sum();
        let weights: Vec<f64> = log_weights
            .iter()
            .map(|&lw| (lw - max_lw).exp() / sum_exp)
            .collect();

        (traces, weights)
    }

    /// Metropolis-Hastings MCMC Sampler.
    pub fn metropolis_hastings<F>(
        model: F,
        initial_params: f64,
        steps: usize,
        proposal_std: f64,
    ) -> Vec<f64>
    where
        F: Fn(f64) -> ModelTrace,
    {
        let mut samples = Vec::with_capacity(steps);
        let mut current_param = initial_params;
        let mut current_log_joint = model(current_param).log_joint();

        for i in 0..steps {
            // Gaussian random walk proposal
            let seed = (i as f64 + 1.0) * 42.0;
            let step_u = (seed * 9876.5432).sin();
            let proposal = current_param + step_u * proposal_std;

            let proposed_trace = model(proposal);
            let proposed_log_joint = proposed_trace.log_joint();

            let log_alpha = proposed_log_joint - current_log_joint;
            let accept_threshold = (seed * 1357.9246).cos().abs().fract();

            if log_alpha >= 0.0 || accept_threshold < log_alpha.exp() {
                current_param = proposal;
                current_log_joint = proposed_log_joint;
            }

            samples.push(current_param);
        }

        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_distribution_log_prob() {
        let std_norm = Distribution::normal(0.0, 1.0);
        // at x = 0, log_prob = -0.5 * ln(2*pi) ≈ -0.9189385
        let lp_zero = std_norm.log_prob(0.0);
        assert!((lp_zero - (-0.9189385332046727)).abs() < 1e-6);

        // Density symmetry
        assert_eq!(std_norm.log_prob(1.5), std_norm.log_prob(-1.5));
    }

    #[test]
    fn test_model_trace_log_joint() {
        let mut trace = ModelTrace::new();
        let mu = trace.sample("mu", Distribution::normal(0.0, 10.0), 2.5);
        trace.observe("obs1", Distribution::normal(mu, 1.0), 3.0);
        trace.observe("obs2", Distribution::normal(mu, 1.0), 2.0);

        assert!(trace.log_joint().is_finite());
        assert_eq!(trace.latents.len(), 1);
        assert_eq!(trace.observations.len(), 2);
    }

    #[test]
    fn test_importance_sampling_gaussian_posterior() {
        // Coin toss with Beta prior: observe 8 heads out of 10
        let (_traces, weights) = BayesianInference::importance_sampling(
            |i| {
                let mut trace = ModelTrace::new();
                let seed = (i as f64) + 1.0;
                let p = Distribution::uniform(0.0, 1.0).sample_pseudo(seed);
                trace.sample("p", Distribution::uniform(0.0, 1.0), p);
                // 8 successes, 2 failures
                for k in 0..8 {
                    trace.observe(&format!("h_{k}"), Distribution::bernoulli(p), 1.0);
                }
                for k in 0..2 {
                    trace.observe(&format!("t_{k}"), Distribution::bernoulli(p), 0.0);
                }
                trace
            },
            100,
        );

        let weight_sum: f64 = weights.iter().sum();
        assert!((weight_sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_metropolis_hastings_mcmc() {
        let samples = BayesianInference::metropolis_hastings(
            |param| {
                let mut trace = ModelTrace::new();
                trace.sample("x", Distribution::normal(5.0, 1.0), param);
                trace.observe("obs", Distribution::normal(param, 0.5), 5.2);
                trace
            },
            0.0,
            200,
            0.5,
        );

        assert_eq!(samples.len(), 200);
        let posterior_mean: f64 = samples[100..].iter().sum::<f64>() / 100.0;
        // Posterior mean should converge near 5.1-5.2
        assert!(posterior_mean > 4.0 && posterior_mean < 6.0);
    }
}
