# RESUME (crash-recovery) — v3 compressed inversion-free ECDLP sampler

A bot or person resuming this work: read **Status** and **Next step** first. Everything here is on branch `v3-research` in `judecutler/ecdsav2`, under `research/v3-sampler/`. Decision trail: `.audit/v3-sampler.tsv`.

## Goal
Build and validate the ECDSA.fail v3 research direction: a discrete-log sampler that (a) never computes the elliptic-curve point in affine form, (b) uses no modular inversion, (c) reads out a single projective-invariant bit. If this works it changes the benchmark's whole cost basis (no point register, no inverter). Build step 1 = prove the representational core on a toy curve before any circuit.

## Status (2026-11-21)
**Phase A COMPLETE — PASS. Go for Phase B.**

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
**Phase B: the reversible circuit.** Express the inversion-free projective point addition (and a windowed scalar multiplication) as a reversible circuit in the challenge gate set (X/CX/CCX), and verify it against `sampler_toy.py`'s `proj_add`/`proj_mul`/`g_projective` with a bit-sliced simulator, exhaustively over inputs on a toy curve. Named data shape: the circuit state is the projective triple `(X:Y:Z)` over `F_q` in RNS residue channels; the readout is one `legendre` circuit (inversion-free, CFS-style). Delegate to the feature model in its own worktree. Gate: the circuit's simulated output must equal `g_projective` for every `k`.

Open design choices for Phase B (settle by prototype, not by asking):
- Standard projective vs Jacobian vs CFS's exact RNS integer-lift. Start with standard projective (formulas already in `sampler_toy.py`), add RNS only when the toy works.
- One output bit vs k bits (trades circuit size for run count; Phase A shows the curve).
- Whether to window the scalar mult now or after a single projective add is verified.

## Files
- `sampler_toy.py` — Phase A validation. The reference implementation of every piece Phase B must reproduce reversibly. Self-checking (asserts).
- `RESUME.md` — this file.
- `UPDATE-BOARD.md` — human-readable status for the group.
- `.audit/v3-sampler.tsv` — decision trail (one row per decision).

## Hazards / notes for a resuming bot
- The sandbox is ephemeral between sessions; the branch on the fork is the source of truth. Re-clone `judecutler/ecdsav2`, `git checkout v3-research`.
- Toy curves use tiny prime-order r (3..23). That is intentional: it lets us compute the *exact* distribution, not sample it. Do not "scale up r" to feel more confident; the derivation is general and the small-r exactness is the stronger evidence.
- Keep the affine path in `sampler_toy.py` as the oracle. Never let the inversion-free path drift from it without the per-k equality check.
