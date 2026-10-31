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
| 4. Point addition + v1 score | v1a 1.47e10, v1b 1.349e10 (both 9,024/9,024 OK); v1c (2,559 q) scoring |
| 5. v2-proxy evaluator | written (`src/bin/eval_v2.rs`); first run on v1b in progress |
| 6. Reflection | pending |

## Next step

Read `version_1/offline/run4/eval_log.txt` for the v1 and v2-proxy scores of v1c, then run the P-256 genericity check (`V1_CURVE=p256` build + `eval_v2 --curve p256`), then write the reflection.

## Log

- 2026-10-31 01:40 PT. Copied the upstream harness from Layr-Labs/ecdsafail-challenge@5ba2d2d (lib.rs, circuit.rs, sim.rs, weierstrass_elliptic_curve.rs, bin/build_circuit.rs, bin/eval_circuit.rs, Cargo files). Those trusted files are unchanged. Everything we write lives in `src/point_add/` and `src/bin/eval_v2.rs`.
- 2026-10-31 02:10 PT. Sandbox blocks crates.io and static.rust-lang.org (403 from the egress proxy), so the pinned 1.93.0 toolchain and the four registry crates cannot be fetched. Added `offline/`, a twin Cargo package that compiles the same unchanged `../src` against local shims: an exact U256 (ruint/alloy), a real SHAKE256 (checked against the FIPS 202 empty-input vector), and a pass-through stand-in for zstd framing. Scores from the twin match a real build because the Fiat-Shamir seed and simulator do not depend on compression. Teammates with crates.io access can build `version_1/` directly.
- 2026-10-31 02:20 PT. Wrote `src/point_add/{gates,arith,divide,point,mod,tests}.rs`. Logical gates: AND (1 Toffoli) and UNAND (X-basis measurement + classically controlled CZ, 0 Toffolis) swap under inversion, so any block can be inverted by replay. Tests: integer add/sub/compare, field add/halve/double/controlled-sub, square-subtract, division for all 250 divisors mod 251, and full point addition on y^2 = x^3 + 7 over F_251, all with clean ancillas and zero phase. One failure was a test bug (input labels captured after relabeling), not a circuit bug.
- 2026-10-31 02:55 PT. **v1a scored.** Full secp256k1 build: 3,074 qubits, 38.5M ops. Unchanged `eval_circuit`: 9,024/9,024 shots correct, 0 phase-garbage, 0 ancilla-garbage batches, avg Toffoli 4,790,008 (constant per shot), **score 14,724,484,592 (1.47e10)**. That is 1.37x the challenge's original 1.07e10 baseline. Exact, but not competitive yet.
- 2026-10-31 03:05 PT. **v1b.** Replaced loaded constant registers with a unified adder whose addend bits are Zero / One / qubit (a control qubit stands in for every set bit of a controlled constant). Width 3,074 -> 2,817 (-257, as predicted), Toffolis 4,787,186. Builder-predicted Q x T = 1.349e10. All small-field tests still pass.
- 2026-10-31 03:20 PT. **v1b scored** with unchanged `eval_circuit`: 9,024/9,024 OK, 2,817 qubits x 4,787,186 Toffolis, **score 13,485,502,962 (1.349e10)**.
- 2026-10-31 03:35 PT. **v1c.** The forward GCD provably ends at (u, v) = (1, 0), so u and v are now released during the target pass and rebuilt before the reverse GCD. beta (provably 0 after the target pass) is freed inside `divide`, and alpha is allocated only after the forward GCD's scratch is free. Two bugs on the way, both caught by the n=8 tests: (1) alpha landed on u's qubits, which the reverse pass needs; (2) the reverse GCD was an id-for-id replay whose scratch collided with alpha (phase garbage). Fix for (2): regenerate the reverse GCD with `inverse_of` so its scratch is allocated fresh. Width 2,817 -> 2,559, Toffolis unchanged (4,787,186). Builder-predicted Q x T = 1.225e10. Scoring in `offline/run4/eval_log.txt`.
