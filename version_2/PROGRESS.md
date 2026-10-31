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
3. **Lazy transcript and window freeing** (done, v2b). Transcript bits are allocated when written and freed when the reverse GCD clears them, and the u/v bits above the proven window are released. GCD-phase live count falls to about 1,790, and the peak moves to the target pass (α, β, transcript, scratch).
4. **Transcript packing** (done, v2c). The step choice is a trit t = odd + swap, because swap implies odd. Five steps share an 8-bit code C = Σ t_j 3^j (243 ≤ 256). Packing a digit is two controlled constant adds plus two constant compares on 8 bits, which clear the step bits. The target pass unpacks one group at a time into 10 temporary bits and repacks after. The transcript shrinks from 1,024 to 820 qubits.
5. **Track Q interface** (done). `V2_ADDEND=quantum` makes registers 2 and 3 qubits. The addend is used in place and only read, so the circuit restores it with no change in Toffolis. The cost is the 512 addend qubits.
6. **Coherent lowering** (done). `V2_PROFILE=coherent` emits no HMR: each measurement-uncomputed AND becomes a Toffoli. It is reported, not optimized.
7. **eval_v2 upgrades** (done). Per-shot W/A/P classes (C-AMP R1), Clopper–Pearson upper bounds, C-AMP amplitude pricing with k = 28 and the 90% gate, a Track Q restore check, and a forward∘reverse identity check whenever the circuit has no measurements.

## Status

| Build | Qubits | Toffolis | Q×T | Tests |
| --- | --- | --- | --- | --- |
| version_1c (start) | 2,559 | 4,787,186 | 1.225e10 | all |
| v2a: in-place GCD + width schedule | 2,303 | 4,264,946 | 9.82e9 | small-field tests pass (superseded before scoring) |
| v2b: lazy transcript + window freeing | 2,049 | 4,264,946 | 8.74e9 | small-field tests pass (superseded before scoring) |
| **v2c: packed transcript** | **1,855** | **4,379,378** | **8.12e9** | small-field tests pass; v1 harness + eval_v2 scoring in `offline/run3` |
| v2c, Track Q (quantum addend) | 2,367 | 4,379,378 | 1.037e10 | builder only so far |
| v2c, Track Q, coherent (no HMR) | 2,367 | 8,100,836 | 1.917e10 | builder only so far |

## Next step

Read `offline/run3/eval_circuit.txt` and `eval_v2.txt` (v2c). Then run `eval_v2` on `offline/run5` (Track Q) and `offline/run4` (Track Q coherent, forward∘reverse checked), and copy the outputs into `results/`. Commands:

```
cd version_2/offline && export RUSTUP_TOOLCHAIN=stable
cargo test --release --offline --bin build_circuit
mkdir -p runN && cd runN && ../target/release/build_circuit && ../target/release/eval_circuit && ../target/release/eval_v2 --shots 2000
```

## Log

- 2026-10-31 15:45 PT. Started version_2 as a copy of version_1c (harness files unchanged, offline twin included).
- 2026-10-31 16:05 PT. Research digest (background agent) and Elliot's repo read. Discord threads unreadable (robots-blocked, no connector); asked the user to paste them.
- 2026-10-31 16:20 PT. **v2a.** In-place GCD + width schedule. Builder: 2,303 qubits (−256), 4,264,946 Toffolis (−522,240). Both matched the pre-build predictions exactly. All n = 8 tests pass, including division by every divisor mod 251.
- 2026-10-31 16:35 PT. **v2b.** Lazy transcript allocation, plus freeing u/v bits above the proven window (`window(k)` in `divide.rs`). alpha is allocated away from u and x ids (`alloc_n_avoiding`), and the reverse GCD is regenerated step by step, reallocating each window before the step it undoes. Width 2,303 → 2,049, Toffolis unchanged. Added `divide_every_divisor_many_primes`: every divisor for p in {11, 13, 131, 193, 241, 257, 311, 509, 1021} at n = 4, 8, 9, 10, all exact with clean ancillas and zero phase.
- 2026-10-31 16:50 PT. **v2c.** Packed transcript (`transcript.rs`). Width 2,049 → 1,855, Toffolis +114,432 (+2.7%). Q×T 8.12e9. Tests cover partial groups (16 steps = 5+5+5+1, 18 = 5+5+5+3, 8 = 5+3).
- 2026-10-31 17:05 PT. Track Q and coherent switches; eval_v2 per-shot classes, C-AMP pricing and the forward∘reverse check. The small-curve test now runs all four combinations (classical or quantum addend × measured or coherent) and checks that a quantum addend is restored.
