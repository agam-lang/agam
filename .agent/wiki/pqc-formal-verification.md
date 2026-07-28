# Post-Quantum Cryptography & Formal Verification

**Context:** The transition to Post-Quantum Cryptography (PQC) is accelerating, driven by the threat of Shor's algorithm running on future quantum computers. NIST has standardized ML-KEM and ML-DSA.

**Observation:** Apple's open-source release of their PQC implementations in `corecrypto` was notable not just for the algorithms, but because they mathematically *proved* the code's correctness using formal verification tools (Cryptol and Isabelle). Traditional unit testing is insufficient for guaranteeing the absolute absence of vulnerabilities in cryptographic logic.

**Relevance to Agam-Lang:**
- **`agam-crypto` Modernization:** Agam must implement ML-KEM and ML-DSA natively to ensure the standard library is ready for the post-quantum era.
- **Formal Verification as a Standard:** Agam currently has an experimental `agam_smt` crate. We must elevate formal verification to be a core pillar of the language's safety story. The `agam-crypto` PQC primitives should be formally verified—either through internal solver integration or by lowering our cryptographic AST into external provers like Isabelle.
- **Ecosystem Defense:** ML-DSA will be integrated directly into `agam_pkg` to sign and verify packages, making the entire Agam-Lang supply chain quantum-resistant from day one.

*Added in response to the Apple Quantum-Resistant Encryption open-source release evaluation (Phase 33).*
