# Phase T5-advanced-math-primitives — Advanced Mathematical & Combinatorial Primitives

## Phase Focus

Integration of high-dimensional geometric bounds, Fourier-analytic sign uncertainty, Hankel moment systems, and saturated-matrix Ramsey covers into `agam_std::math`, `agam_std::numerical`, and `agam_std::code`.

## Key Capabilities & Algorithms

1. **Fourier Sign-Uncertainty & Sphere Packing (`agam_std::math::packing`)**:
   - `cohn_elkies_bound(d: usize) -> f64`: Compute high-dimensional Cohn-Elkies LP density bounds.
   - `fourier_sign_uncertainty_radius(d: usize, sign: Sign) -> f64`: Compute $\frac{1}{\pi}\sqrt{d}$ Fourier eigenfunction sign-change thresholds.
   - Mellin-Hankel functional equation solver for radial Schwartz profiles ($M_g(z) = \pi^{\lambda-z} \frac{\Gamma(z/2)}{\Gamma((d-z)/2)} M_g(d-z)$).

2. **Hankel Moment Systems & Reed-Solomon Recovery (`agam_std::code::hankel`)**:
   - `solve_hankel_system(moments: &[FieldElement]) -> Result<Vec<FieldElement>, HankelError>`: Algebraic root set recovery from low-degree power sums.
   - Bounded polynomial-moment family reconstruction for CVP / nearest-codeword decoding.

3. **Ehrhart Convex Body Volume Bounds (`agam_std::numerical::ehrhart`)**:
   - `barycentric_convex_volume_upper_bound(dim: usize) -> f64`: Compute sharp Ehrhart volume bound $\frac{(n+1)^n}{n!}$.
   - Bergman potential ray evaluation $\psi_t$ and initial slope $\dot{\psi}_{0+}$.

4. **Saturated-Matrix Ramsey Covering (`agam_std::combinatorics::ramsey`)**:
   - `saturated_matrix_cover(h: usize, m: usize) -> MatrixCover`: Two-sided coordinate cover generator for $H$-colored $s \times H^m$ saturated matrices.
   - Superexponential Ramsey lower bound generator $R_k(3) = k^{\Theta(k)}$.

## Verification Plan

- Unit tests in `agam_std` for Cohn-Elkies radius limits as $d \to \infty$.
- Test Hankel system root-reconstruction against known Reed-Solomon evaluation sets.
- Test Ehrhart volume bound against $n$-simplices.
