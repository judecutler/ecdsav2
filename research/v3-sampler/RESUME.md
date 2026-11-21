# RESUME (crash-recovery) — v3 compressed inversion-free ECDLP sampler

A bot or person resuming this work: read **Status** and **Next step** first. Everything here is on branch `v3-research` in `judecutler/ecdsav2`, under `research/v3-sampler/`. Decision trail: `.audit/v3-sampler.tsv`.

## Goal
Build and validate the ECDSA.fail v3 research direction: a discrete-log sampler that (a) never computes the elliptic-curve point in affine form, (b) uses no modular inversion, (c) reads out a single projective-invariant bit. If this works it changes the benchmark's whole cost basis (no point register, no inverter). Build step 1 = prove the representational core on a toy curve before any circuit.

## Status (2026-11-21)
**Phase A COMPLETE (PASS). Phase A+ COMPLETE. Phase B PARTIAL (generic domain verified; exceptional branches + readout deferred).**

Next unit = Phase B2: reversible exceptional-case handling + the inversion-free Legendre readout (see "Next step").

`sampler_toy.py` validated the core claim on 4 toy curves (F23, F41, F97, F211). Run it:
```
cd research/v3-sampler && python3 -I sampler_toy.py
```
Expected: `OVERALL: PASS`. Pure Python, no deps, ~1 s.

What is proven (exactly, by exhaustive/analytic computation, not sampling):
1. **Inversion-free readout == affine readout.** `g(k) = [legendre_q(X*Z) == -1]` computed from projective coords `(X:Y:Z)` of `[k]P` equals `[legendre_q(x) == -1]` from the affine point, for every `k` in `[0,r)`. The trick: `legendre(X/Z) = legendre(X*Z)`, and under projective scaling `(X:Y:Z)->(lX:lY:lZ)` the product `X*Z -> l^2 X*Z`, a square, so the Legendre symbol is invariant to the representative. This is why projective coordinates (normally broken in Shor by non-uniqueness) work once the output is compressed.
2. **Exact d recovery.** With control registers of size r and output bit `g`, all measurement mass lies on the line `v == d*u (mod r)`. Verified two ways: a derivation (mass = `[v==du] * |g_hat_m(u)|^2`) and an independent full `r*r` distribution that assumes no line and finds off-line mass ~1e-33. So any run with `gcd(u,r)=1` yields `d = v * u^{-1} mod r`.
3. **Compression cost, quantified.** A balanced single-bit readout puts ~1/2 the probability on the useless `u=0` outcome, vs `1/r` for a full point register. So single-bit compression costs ~2x in run count. Reducible by reading out a few bits instead of one (k-bit compression) — a Phase B+ knob.

## Next step
**Phase B2: close the circuit to a full readout.** Two pieces, each verified against `sampler_toy.py` with the bit-sliced simulator before moving on (principle-sequence-verifiable-units):
1. **Exceptional cases, reversibly.** `proj_add` today covers the generic branch only. Add the reference's exceptional branches (either input at infinity Z=0; P2 = +-P1, i.e. doubling and P + -P) as a FIXED gate list, using a reversible compare (V1==V2, U1==U2, Z==0) plus controlled select between the add-formula and the double-formula outputs. Then build `proj_dbl` and compose `proj_mul` (windowed or double-and-add). Gate: matches `sampler_toy.proj_mul` for all k on a toy curve, clean ancillas.
2. **Inversion-free Legendre readout.** One circuit computing the bit `g(k) = [legendre_q((X-cZ)Z) == -1]` from the projective triple, with no inversion. For the toy, `legendre` via modular exponentiation (square-and-multiply = a chain of `modmul`) is fine and stays inversion-free. Gate: the circuit's output bit equals `sampler_toy.g_projective` (and `compression_tradeoff.readout_bits`) for every k.

Open design choices (settle by prototype, not by asking):
- Standard projective (done) vs CFS's exact RNS integer-lift. Add RNS only when the standard toy readout works end to end; RNS is the space-optimization, not the correctness core.
- One output bit vs k bits: `compression_tradeoff.py` already gives the run-count curve; pick k from the target.
- Window the scalar mult vs double-and-add: double-and-add first (simpler oracle), window later.

Phase B results so far (branch merged into v3-research):
- `circuit/modarith.py`: out-of-place `modadd/modsub/modmul` mod q, inversion-free (shift-add), exhaustively verified q in {5,7,11,13}, clean ancillas. CCX: modadd 72-96, modsub 48-64, modmul 882-1568.
- `circuit/projadd.py`: generic projective add, verified projectively vs `sampler_toy.proj_add` (F23: 676 cases; F97: 9604 cases; 0 mismatches; inputs preserved; clean). Fixed gate list, no classical branching.
- Harness has teeth: a mutation breaking the mod reduction makes the exhaustive test report errors (checked this session).

## Files
- `sampler_toy.py` — Phase A validation. The reference implementation of every piece Phase B must reproduce reversibly. Self-checking (asserts).
- `RESUME.md` — this file.
- `UPDATE-BOARD.md` — human-readable status for the group.
- `.audit/v3-sampler.tsv` — decision trail (one row per decision).

## Hazards / notes for a resuming bot
- The sandbox is ephemeral between sessions; the branch on the fork is the source of truth. Re-clone `judecutler/ecdsav2`, `git checkout v3-research`.
- Toy curves use tiny prime-order r (3..23). That is intentional: it lets us compute the *exact* distribution, not sample it. Do not "scale up r" to feel more confident; the derivation is general and the small-r exactness is the stronger evidence.
- Keep the affine path in `sampler_toy.py` as the oracle. Never let the inversion-free path drift from it without the per-k equality check.

## Phase B (reversible circuit prototype, branch v3-phaseB-circuit)
`circuit/` holds a reversible circuit builder (X/CX/CCX/SWAP), a bit-sliced simulator, and
out-of-place mod-q `modadd`/`modsub`/`modmul` (no inversion), exhaustively verified for
q in {5,7,11,13}. `proj_add` reproduces `sampler_toy.proj_add` projectively on F_23 and F_97.
**DEFERRED (not done):** the exceptional branches (infinity inputs, P2 = +-P1 i.e. doubling and
P + -P). `proj_add` is verified on the GENERIC domain only. Also not done: `proj_dbl`, `proj_mul`,
Legendre readout. Run: `python3 -I circuit/test_modarith.py && python3 -I circuit/test_projadd.py`.
