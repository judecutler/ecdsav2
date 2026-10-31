# version_2 — plan and progress log

Crash-recovery file. Read **Status** and **Next step** first. version_1 is frozen; its log is `../version_1/PROGRESS.md`.

## Goal

Keep version_1's guarantees and cut Q×T, while matching where QCB's v2 benchmark is heading:

- **Exact** on every input in the task domain. Any shortcut must come with a proof, never a sampled budget.
- **Field-generic.** No step depends on the shape of the prime.
- **Quantum-addend ready (Track Q).** It runs with A_x, A_y as qubit registers and restores them.
- **Coherent-profile ready.** It can be lowered without R/HMR measurements, and both costs are reported.

## What v2 is likely to require

Sources: Elliot's repo (`2elliot/qcb-ecdsafail`, the `#verified-claims` catalogue of Oct 29, and teammates' agent docs). I could not read the newer Discord threads; there is no Discord access from here. All entries below are **source-read** from teammates' proposals; none is a decided rule.

| Proposal | Source | Effect on us |
| --- | --- | --- |
| Inputs independent of the op stream, fixed or secret corpus | R002 (Mike), C-TAIL (joel), assurance architecture | `eval_v2` already uses a fixed seed |
| Amplitude-priced error: Score = T̄·Q / [(1−ε_W−ε_A)^56 (1−2ε_P)^56], gate ε_W+ε_A+2ε_P ≤ 0.188% | C-AMP, R_P003 (Aodhfionn) | 0 failures measured on random inputs, so no penalty |
| Track Q: A_x, A_y are 256-qubit registers, must be restored; legacy Profile A = (Q+16, T+131,068) | Track Q synthesis (joel) | our addend is only ever read, so native |
| Coherent profile rejects R/HMR | C-03 (Jack) | we use HMR for AND uncomputation; must report a measurement-free lowering |
| Stress tests at R = A, −A, −2A; "no complete-adder mandate" | assurance architecture (joel) | we fail all three; document |
| Track G: other primes | C-MOD (Jude), R003 (Mike), Track G synthesis | already generic; tested on P-256 |
| Forward∘reverse identity check | C-TAIL, README | only meaningful without measurements; add to `eval_v2` |

## Design changes (decided)

1. **In-place GCD on x** (done, v2a). The target pass never reads x, so the GCD uses x itself as v. The reverse GCD restores it. Saves the 256-qubit copy.
2. **Proven width schedule** (done, v2a). Before step k, u·v < 2^(2n−k) and u ≥ 1, so for k ≥ n both fit in 2n−k bits. GCD steps act only on that prefix. This is exact: the bits above it are |0⟩ on every input. Pornin (eprint 2020/972) shows 2·len−1 steps is tight for binary GCD, so the 512-step count itself cannot be cut for this GCD.
3. **Lazy transcript and window freeing** (next). Transcript bits are allocated when written and freed when cleared, and the u/v bits above the window are released. The expected peak moves from the GCD phase to the target phase: about 2,049.
4. **Transcript packing** (planned). Steps have 3 possible symbols, so 5 steps fit in 8 bits (243 ≤ 256): 1,024 → 820 qubits, at about +3% Toffolis.

## Status

| Build | Qubits | Toffolis | Q×T | Tests |
| --- | --- | --- | --- | --- |
| version_1c (start) | 2,559 | 4,787,186 | 1.225e10 | all |
| v2a: in-place GCD + width schedule | 2,303 | 4,264,946 | 9.82e9 | n=8 all pass; 256-bit scoring pending |

## Next step

Implement item 3 in `src/point_add/divide.rs`, run `cargo test`, then rebuild and score:

```
cd version_2/offline && export RUSTUP_TOOLCHAIN=stable
cargo test --release --offline --bin build_circuit
mkdir -p runN && cd runN && ../target/release/build_circuit && ../target/release/eval_circuit && ../target/release/eval_v2 --shots 2000
```

## Log

- 2026-10-31 15:45 PT. Started version_2 as a copy of version_1c (harness files unchanged, offline twin included).
- 2026-10-31 16:05 PT. Research digest (background agent) and Elliot's repo read. Discord threads unreadable (robots-blocked, no connector); asked the user to paste them.
- 2026-10-31 16:20 PT. **v2a.** In-place GCD + width schedule. Builder: 2,303 qubits (−256), 4,264,946 Toffolis (−522,240). Both matched the pre-build predictions exactly. All n = 8 tests pass, including division by every divisor mod 251.
