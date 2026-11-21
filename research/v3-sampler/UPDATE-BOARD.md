# v3 Sampler — Group Update Board

Plain-language status for the group. Newest update on top. The technical detail is in `RESUME.md`; the resumable log is `.audit/v3-sampler.tsv`.

---

## 2026-11-21 — Phase A done: the core idea works on paper and in code

**Headline: the "don't compute the point, don't invert, read one bit" sampler recovers the secret key `d` on toy curves. The risky part of the whole plan is validated.**

Why this matters for the paper. Every circuit on the current board spends most of its qubits holding a full elliptic-curve point and most of its gates on a modular inversion. The v3 idea is to do neither: compute the point in projective coordinates (no inversion), and read out a single yes/no bit instead of the whole point. The open question was whether that single bit still carries enough information to recover the key. 

**Answer: yes, exactly.** We computed the full quantum measurement distribution (not a simulation estimate, the exact thing) on four toy curves. Every possible measurement outcome lands on a single clean line that reveals `d`. The one-bit readout loses nothing in *correctness*; it only means you need roughly twice as many runs as a full-point readout, because about half the runs land on a useless "zero" outcome. That run-count cost is a dial we can turn (read a few bits instead of one) and it's a nice figure for the paper.

We also proved the inversion-free readout gives the identical bit to the normal affine one, for every input — so the projective shortcut is exact, not approximate.

**What's next:** turn the validated math into an actual reversible circuit (the thing the benchmark scores) and check it gate-for-gate against this reference on a toy curve.

**How to see it yourself:**
```
git checkout v3-research
cd research/v3-sampler && python3 -I sampler_toy.py
```
Prints `OVERALL: PASS` in about a second. No setup, no dependencies.

---

## Roadmap (where this sits)
1. [DONE] Prove the compressed inversion-free sampler recovers `d` (toy, exact). ← we are here
2. [NEXT] Reversible circuit for the inversion-free projective arithmetic, verified vs the reference.
3. Whole-attack cost script: score this against the archived board circuits on the new metric.
4. The v3 challenge reference: metric, exactness contract, hardware-profile tracks.
5. The paper.
