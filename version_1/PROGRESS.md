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
| 4. Point addition + v1 score | small-curve test passes; full 256-bit build next |
| 5. v2-proxy evaluator | pending |
| 6. Reflection | pending |

## Next step

Build the full secp256k1 circuit and score it with the unchanged eval_circuit (offline twin): `cd version_1/offline && RUSTUP_TOOLCHAIN=stable cargo run --release --offline --bin build_circuit && cargo run --release --offline --bin eval_circuit`.

## Log

- 2026-10-31 01:40 PT. Copied the upstream harness from Layr-Labs/ecdsafail-challenge@5ba2d2d (lib.rs, circuit.rs, sim.rs, weierstrass_elliptic_curve.rs, bin/build_circuit.rs, bin/eval_circuit.rs, Cargo files). Those trusted files are unchanged. Everything we write lives in `src/point_add/` and `src/bin/eval_v2.rs`.
- 2026-10-31 02:10 PT. Sandbox blocks crates.io and static.rust-lang.org (403 from the egress proxy), so the pinned 1.93.0 toolchain and the four registry crates cannot be fetched. Added `offline/`, a twin Cargo package that compiles the same unchanged `../src` against local shims: an exact U256 (ruint/alloy), a real SHAKE256 (checked against the FIPS 202 empty-input vector), and a pass-through stand-in for zstd framing. Scores from the twin match a real build because the Fiat-Shamir seed and simulator do not depend on compression. Teammates with crates.io access can build `version_1/` directly.
- 2026-10-31 02:20 PT. Wrote `src/point_add/{gates,arith,divide,point,mod,tests}.rs`. Logical gates: AND (1 Toffoli) and UNAND (X-basis measurement + classically controlled CZ, 0 Toffolis) swap under inversion, so any block can be inverted by replay. Tests: integer add/sub/compare, field add/halve/double/controlled-sub, square-subtract, division for all 250 divisors mod 251, and full point addition on y^2 = x^3 + 7 over F_251, all with clean ancillas and zero phase. One failure was a test bug (input labels captured after relabeling), not a circuit bug.
