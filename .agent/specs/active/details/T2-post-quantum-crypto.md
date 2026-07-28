# Phase T2-post-quantum-crypto — Post-Quantum Cryptography and Formal Verification

## Goal

Modernize the `agam-crypto` library with NIST-standardized Post-Quantum Cryptography (PQC) and establish a formal verification pipeline to mathematically prove the correctness of these critical implementations.

## Background

As quantum computing advances, traditional public-key cryptography (like RSA and ECC) becomes vulnerable to Shor's algorithm. Apple's recent open-sourcing of their quantum-resistant encryption (used in `corecrypto`) highlights two critical industry shifts:
1. The adoption of ML-KEM (FIPS 203) and ML-DSA (FIPS 204).
2. The necessity of using formal verification (via tools like Cryptol and Isabelle) to mathematically prove cryptographic correctness, rather than relying solely on traditional unit testing.

Agam-Lang, aiming to be a premier systems language with robust safety guarantees, must adopt both the algorithms and the formal verification methodology. 

## Implementation Checklist

- [ ] **PQC Algorithm Implementation:** Implement ML-KEM (FIPS 203) for key encapsulation and ML-DSA (FIPS 204) for digital signatures natively within the `agam-crypto` library.
- [ ] **Formal Verification Pipeline:** Utilize the experimental `agam_smt` crate (or build translators to external provers like Isabelle) to establish a formal verification pipeline.
- [ ] **Mathematical Proofs:** Write formal specifications for the `agam-crypto` PQC implementations and execute the verification pipeline to mathematically guarantee their correctness against the FIPS standards, preventing subtle implementation bugs or side-channel vulnerabilities.
- [ ] **Supply Chain Security:** Integrate ML-DSA into the `agam_pkg` package manager for signing and verifying packages, ensuring the Agam ecosystem's supply chain is quantum-resistant.

## Dependencies

- Phase T2-os-sandbox (Runtime Hardening)
- Phase T1-resolver-lockfile (Deterministic Resolver and Lockfile - for package integration)
