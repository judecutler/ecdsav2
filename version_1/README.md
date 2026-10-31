# version_1: an exact, field-generic secp256k1 point adder

QCB's first ecdsa.fail v2 submission, built from scratch rather than forked from the leaderboard lineage. It follows the v2 outline: correctness first, no test-set grinding, no use of the classical addend beyond what a windowed-Shor lookup provides, and no dependence on the shape of the secp256k1 prime.

`PROGRESS.md` holds the plan, the measured results, the run log, and the reflection.

## Layout

| Path | What it is |
| --- | --- |
| `src/point_add/` | Our circuit. `gates.rs` (builder, AND/UNAND, replay-inverse), `arith.rs` (adders, comparators, mod-p add/sub/halve/double/square), `divide.rs` (exact binary-GCD division), `point.rs` (point addition), `tests.rs` (small-field tests). |
| `src/bin/eval_v2.rs` | v2-proxy evaluator: fixed seeds, large sample, edge-case classes, worst-case Toffolis, classical-conditioning census, other curves. |
| `src/{lib,circuit,sim,weierstrass_elliptic_curve}.rs`, `src/bin/{build,eval}_circuit.rs` | Upstream harness from Layr-Labs/ecdsafail-challenge@5ba2d2d, unchanged. |
| `offline/` | Twin Cargo package that builds the same sources against local shims, for machines without crates.io access. |

## Run it

With crates.io access (needs the pinned 1.93.0 toolchain):

```
cd version_1
cargo run --release --bin build_circuit
cargo run --release --bin eval_circuit -- --note "version_1"
cargo run --release --bin eval_v2 -- --shots 20000
```

Without crates.io (what the build sandbox had):

```
cd version_1/offline
export RUSTUP_TOOLCHAIN=stable
cargo test  --release --offline --bin build_circuit     # small-field tests
cargo run   --release --offline --bin build_circuit
cargo run   --release --offline --bin eval_circuit -- --note "version_1"
cargo run   --release --offline --bin eval_v2 -- --shots 20000
V1_CURVE=p256 cargo run --release --offline --bin build_circuit   # same code, NIST P-256 prime
cargo run   --release --offline --bin eval_v2 -- --curve p256 --shots 2000
```

The offline twin writes `ops.bin` uncompressed (about 2 GB), because its zstd stand-in passes bytes through. Scores are unaffected.
