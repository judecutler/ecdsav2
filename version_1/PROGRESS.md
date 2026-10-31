# version_1 — plan and progress log

Crash-recovery file. Each work session updates this file and pushes it. If a run dies, read **Status** and **Next step** first.

## Goal

Build a from-scratch ECDSA.fail point-addition submission that follows the QCB v2 outline:

- **Exact.** Correct for every valid input, not just for a hash-selected test set. No tuned iteration budgets.
- **Coherent-ready.** No Toffoli is conditioned on classical offset bits, and no free classical computation runs on the offset. Each offset-dependent value is a constant that a windowed-Shor lookup table could hold.
- **Field-generic.** The builder takes the prime as a parameter. Nothing depends on the shape of 2^256 − 2^32 − 977.
- **Measured honestly.** Scored with the unchanged upstream harness (v1 score), and with a v2-proxy evaluator that uses fixed seeds, a large sample, and edge cases.

## Design (decided)

Affine mixed addition with two in-place divisions and one square:

1. `X -= cx`, `Y -= cy` (dx, dy)
2. Division D with divisor X: `(A=0, Y=dy) -> (A=λ, Y=0)`
3. `X += 3cx`, `X -= λ²` → `X = cx - x3`
4. Inverse division D† with divisor X: `(A=λ, Y=0) -> (A=0, Y=λ(cx-x3)) = (0, y3+cy)`
5. `Y -= cy`, `X = cx - X`

Division D runs a binary GCD on `(u, v) = (p, x)` with three cases (v even: halve; v odd and v ≥ u: v = (v−u)/2; v odd and v < u: swap, then the same). `log2(u·v)` drops by at least 1 per step, so 2n steps always finish (v = 0, u = 1). A finished state (v = 0) is a fixed point of the "v even" case, so no done flag is needed. The same steps applied mod p to `(α, β) = (0, y)` give `(y/x, 0)`. The forward GCD writes a transcript (2 bits/step), the target pass reads it, and the reverse GCD clears it, returning `(u, v)` to `(p, x)`.

## Status

| Phase | State |
| --- | --- |
| 1. Scaffold + harness copy | done |
| 2. Arithmetic primitives + tests | done (n=8, p=251 tests pass) |
| 3. Exact divider | done (every divisor mod 251 passes) |
| 4. Point addition + v1 score | done. v1c: 2,559 q x 4,787,186 T = **1.225e10**, 9,024/9,024 OK |
| 5. v2-proxy evaluator | done. 0 failures on 20,000 random secp256k1 inputs and 2,000 P-256 inputs; fails only on T = -2O |
| 6. Reflection | done (below) |

## Next step

Run complete. For version_2, start from **Suggestions** below. The first item is Kaliski-style integer cofactors in the target pass.

## Results

All numbers are **measured** in this sandbox with the offline twin. Raw outputs are in `results/`.

| Build | Qubits | Toffoli / shot | Q x T | v1 harness (9,024 shots) |
| --- | --- | --- | --- | --- |
| v1a: first full build | 3,074 | 4,790,008 | 1.472e10 | all OK |
| v1b: unified constant adder | 2,817 | 4,787,186 | 1.349e10 | all OK |
| **v1c: (u,v) released during target pass** | **2,559** | **4,787,186** | **1.225e10** | **all OK** |

v2 proxy on v1c (`eval_v2`, fixed seed `qcb-v2-proxy-2026-10-31`):

| Class | secp256k1 | P-256 (same code, `V1_CURVE=p256`) |
| --- | --- | --- |
| Random pairs | 0 wrong of 20,000 (and of a separate 2,000) | 0 wrong of 2,000 |
| Small multiples (±1..12)·G | 12 wrong of 528, all T = -2O | 12 of 528, all T = -2O |
| Near-order multiples | 3 wrong of 93, all T = -2O | 3 of 93, all T = -2O |
| T = -2O class | 64 wrong of 64 | 64 of 64 |
| Toffolis conditioned on classical bits | 0 | 0 |
| Ops conditioned on offset bits | 2,560 (X loads of cx/cy only) | 2,560 |
| Bit-logic ops (free classical compute) | 0 | 0 |

Worst-case Toffolis equal the average because no Toffoli is conditioned on anything. The 20,000-shot JSON reports an average of 4,794,845.5. That figure is an evaluator artifact: idle lanes in the last partial 64-shot batch were counted. It was fixed (idle lanes rerun a real case) before the 2,000-shot reruns, which report 4,787,186.0 exactly.

Measured Toffoli breakdown (`cargo test --ignored cost_breakdown`, n = 256):

| Component | Toffolis | Share |
| --- | --- | --- |
| GCD passes (4 x 512 steps x 1,024) | 2,097,152 | 43.8% |
| Target passes (2 x 512 steps x 2,045) | 2,094,080 | 43.7% |
| Square-subtract (shift-and-subtract) | 587,778 | 12.3% |
| 8 field adds/subs | 8,176 | 0.2% |
| **Total** | **4,787,186** | |

Peak width 2,559 is set during the GCD passes: x (256) + y (256) + u, v (512) + transcript (1,024) + step scratch (~511). The transcript alone is 40% of the peak.

## Reflection

**How it performs.** version_1 does what the v2 outline asked for. It is exact on every input class except one documented case, worst-case equals average, it uses no classical computation on the addend, and it runs unchanged on another curve. On cost it is not competitive. Its Q x T of 1.225e10 is 1.14x the challenge's original 1.07e10 baseline and about 13x the 17 October leaderboard leader (1,234 q x 761,096 T, about 9.39e8). It is also 6.2x the organizers' coherent-addend variant (1.96e9 retry-adjusted) and 4.1x Google's low-gate bound (3.0e9). The gap is 2.1x in width and 6.3x in Toffolis versus the leader. Those comparisons are under different accounting: the leader uses a classical addend and accepts small failure rates, so part of the gap is the price of exactness, and part is that version_1 is a first, unoptimized architecture.

**What the gap is made of.**
- **Step count.** Every division runs the proven 2n = 512 GCD steps, 4 passes per point addition. Leaders run shorter budgets tuned to average-case data and accept the residual failures. A tighter *proven* bound (Bernstein-Yang style, computer-checked) is the honest way to close this. Every 10% off the step bound removes about 8.7% of all Toffolis.
- **Target pass.** Each step does a full mod-p controlled subtract plus a mod-p halving (8n Toffolis). Kaliski's almost-inverse keeps the cofactors as integers below p without per-step reduction, which would bring a step to roughly cswap + add + shift (about 3n). That is the largest single lever, about -27% of all Toffolis if it works out.
- **Transcript width.** 2 qubits per step for 3 possible step types. Packing 3 steps into 5 bits saves about 170 qubits, at some Toffoli cost.
- **Squaring.** Shift-and-subtract costs about 9n^2. A schoolbook or Karatsuba square plus a reduction would roughly halve it.

**The one failing case.** T = -2O (the result is -O, so x3 equals the addend's x) breaks the inverse division, because its divisor cx - x3 is 0. In Shor this has negligible probability per addition, but a v2 adversarial set will include it. The clean fix is a per-addend table word (the tangent slope at O), which a windowed lookup can carry. In this v1 harness, computing it from the classical offset bits would be exactly the free-classical-compute shortcut we ruled out, so it was left unhandled and is reported explicitly.

**What the evaluator taught us.** Running the same circuit through fixed seeds and edge-case classes immediately separated "passes the hash-chosen 9,024" from "correct". The upstream reference submission at Layr-Labs@5ba2d2d appends 96 X gates plus a `TAIL_NONCE` in `src/point_add/mod.rs::build()` (source-read). That is the op-stream grinding that `eval_v2`'s fixed seeds neutralize. Two evaluator lessons: per-shot metrics must not count idle simulator lanes, and the reference curve arithmetic dominates runtime at 20k+ shots with the shim U256 (about 8 of 10 minutes).

**Process notes.** The n = 8, p = 251 tests caught both allocator bugs in v1c before any 256-bit run, which took 5 minutes each. Inverting a block by replaying gate ids is only safe when nothing else is live on that block's scratch qubits; regenerating the inverse with fresh allocation is the safe default. The offline twin made the harness buildable without crates.io while keeping every trusted harness file byte-identical.

## Suggestions

1. **Kaliski integer cofactors in the target pass** (largest Toffoli lever, about -27% if per-step reduction can be dropped).
2. **Prove a tighter step bound** for this GCD variant, or switch to Bernstein-Yang divsteps, whose bound is already published. Adopt it only with a proof or a machine-checked bound, never a sampled one.
3. **Handle T = -2O** via a lookup-table word, and give the v2 harness a coherent-addend mode so table words are charged at QROM cost instead of computed for free.
4. **Transcript packing** (3 steps in 5 bits) and Karatsuba squaring for width and the remaining 12%.
5. **Run `eval_v2` on the current leaderboard circuits** (needs a machine with crates.io and roughly 5 minutes per 10k shots each). It turns the v2 outline's "what v2 punishes" table from inferred to measured.
6. **Speed up `eval_v2`'s reference arithmetic** (projective coordinates or a fast mul_mod), so 100k+ shots is routine.


## Log

- 2026-10-31 01:40 PT. Copied the upstream harness from Layr-Labs/ecdsafail-challenge@5ba2d2d (lib.rs, circuit.rs, sim.rs, weierstrass_elliptic_curve.rs, bin/build_circuit.rs, bin/eval_circuit.rs, Cargo files). Those trusted files are unchanged. Everything we write lives in `src/point_add/` and `src/bin/eval_v2.rs`.
- 2026-10-31 02:10 PT. Sandbox blocks crates.io and static.rust-lang.org (403 from the egress proxy), so the pinned 1.93.0 toolchain and the four registry crates cannot be fetched. Added `offline/`, a twin Cargo package that compiles the same unchanged `../src` against local shims: an exact U256 (ruint/alloy), a real SHAKE256 (checked against the FIPS 202 empty-input vector), and a pass-through stand-in for zstd framing. Scores from the twin match a real build because the Fiat-Shamir seed and simulator do not depend on compression. Teammates with crates.io access can build `version_1/` directly.
- 2026-10-31 02:20 PT. Wrote `src/point_add/{gates,arith,divide,point,mod,tests}.rs`. Logical gates: AND (1 Toffoli) and UNAND (X-basis measurement + classically controlled CZ, 0 Toffolis) swap under inversion, so any block can be inverted by replay. Tests: integer add/sub/compare, field add/halve/double/controlled-sub, square-subtract, division for all 250 divisors mod 251, and full point addition on y^2 = x^3 + 7 over F_251, all with clean ancillas and zero phase. One failure was a test bug (input labels captured after relabeling), not a circuit bug.
- 2026-10-31 02:55 PT. **v1a scored.** Full secp256k1 build: 3,074 qubits, 38.5M ops. Unchanged `eval_circuit`: 9,024/9,024 shots correct, 0 phase-garbage, 0 ancilla-garbage batches, avg Toffoli 4,790,008 (constant per shot), **score 14,724,484,592 (1.47e10)**. That is 1.37x the challenge's original 1.07e10 baseline. Exact, but not competitive yet.
- 2026-10-31 03:05 PT. **v1b.** Replaced loaded constant registers with a unified adder whose addend bits are Zero / One / qubit (a control qubit stands in for every set bit of a controlled constant). Width 3,074 -> 2,817 (-257, as predicted), Toffolis 4,787,186. Builder-predicted Q x T = 1.349e10. All small-field tests still pass.
- 2026-10-31 03:20 PT. **v1b scored** with unchanged `eval_circuit`: 9,024/9,024 OK, 2,817 qubits x 4,787,186 Toffolis, **score 13,485,502,962 (1.349e10)**.
- 2026-10-31 03:35 PT. **v1c.** The forward GCD provably ends at (u, v) = (1, 0), so u and v are now released during the target pass and rebuilt before the reverse GCD. beta (provably 0 after the target pass) is freed inside `divide`, and alpha is allocated only after the forward GCD's scratch is free. Two bugs on the way, both caught by the n=8 tests: (1) alpha landed on u's qubits, which the reverse pass needs; (2) the reverse GCD was an id-for-id replay whose scratch collided with alpha (phase garbage). Fix for (2): regenerate the reverse GCD with `inverse_of` so its scratch is allocated fresh. Width 2,817 -> 2,559, Toffolis unchanged (4,787,186). Builder-predicted Q x T = 1.225e10. Scoring in `offline/run4/eval_log.txt`.
- 2026-10-31 03:50 PT. **v1c scored** with unchanged `eval_circuit`: 9,024/9,024 OK, **score 12,250,408,974 (1.225e10)**. `eval_v2` (20,000 random secp256k1 shots): 0 wrong. Edge classes: 15 wrong, all of the T = -2O form (the evaluator now attributes them).
- 2026-10-31 04:05 PT. Fixed an `eval_v2` artifact: idle lanes in partial batches were counted in the Toffoli average. Re-ran 2,000-shot classes on secp256k1 and P-256: identical profile, 4,787,186 Toffolis per shot exactly. Wrote the results, reflection and suggestions above.
