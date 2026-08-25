# Phase T6-algebraic-complexity-verification — Permanent Lower Bounds, Baur-Strassen AD & Entangled Game Verification

## Phase Focus

IR-level Baur-Strassen reverse-mode automatic differentiation, division-free matrix permanent GPU lowering, quantum correlated sampling simulators, and non-sofic group verification.

## Key Capabilities & Algorithms

1. **Baur-Strassen AD Pass in MIR (`agam_mir::optimize::baur_strassen`)**:
   - `lower_reverse_mode_ad(mir: &MirFunction) -> MirFunction`: Transform homogeneous polynomial circuits into combined function and gradient evaluators with $\le 3\times$ multiplication gate cost.
   - Slicing and affine specialization avoiding critical locus $\text{Crit}(P) = \{x : \nabla P(x) = 0\}$.

2. **Division-Free Permanent GPU Acceleration (`agam_codegen::nvptx::permanent`)**:
   - Ryser / Glynn / Square-zero algebra block permanent kernels (`@gpu` targets).
   - Fast $O(n 2^n)$ parallel CUDA lowering for minor-sum polynomials $M_{t,s,d}(X)$.

3. **Quantum Correlated Sampling & Entangled Game Verification (`agam_std::quantum`)**:
   - `quantum_correlated_sampling(state: &QuantumState, alpha: f64) -> TargetState`: Simulates postselection-stable quantum state alignment.
   - Entangled value decay evaluator $\omega^*(G^{\otimes n}) \le \exp\left(-c_{qs} \frac{\varepsilon^{13}}{\varepsilon + \log(|A||B|)} n\right)$.

4. **Non-Sofic & Property-(T) Group Testing (`agam_std::group_theory`)**:
   - Unit group generators for binary Leavitt algebra $L_{\mathbb{F}_2}(1, 2)^\times$.
   - Ershov–Jaikin-Zapirain property-(T) Kazhdan constant bounds.

## Verification Plan

- MIR pass tests verifying $\le 3\times$ multiplication gate expansion on AD transformations.
- GPU kernel validation for $4\times 4$ and $8\times 8$ permanent evaluation matching expected scalar block permanents.
- Quantum correlated sampling total variation distance tests.
