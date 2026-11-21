# v3 Sampler — Group Update Board

Plain-language status for the group. Newest update on top. The technical detail is in `RESUME.md`; the resumable log is `.audit/v3-sampler.tsv`.

---

## 2026-11-21 — Phase B: the math is now a real reversible circuit (the part the benchmark scores)

**Headline: the inversion-free projective arithmetic now runs as an actual reversible circuit in the challenge's gate set, and it matches the Phase A reference exactly on toy curves.**

What this adds. Phase A proved the idea works as math. Phase B turns the core arithmetic into the thing a quantum computer would actually run: a fixed list of X / CX / CCX gates, no inversion anywhere. We built reversible modular add, subtract and multiply (checked against every possible input for four small fields, with all scratch wires returning to zero), then composed them into a full projective point addition. That point-add was checked against the Phase A reference on two toy curves, over ten thousand input pairs, with the inputs deliberately scrambled into different projective representatives to make sure the circuit handles the non-unique coordinates correctly. Zero mismatches.

We also confirmed the test can catch a bug: deliberately breaking the modular reduction makes it fail loudly, so a pass means something.

Honest scope. The point-add covers the normal case. The special cases (a point at infinity, or adding a point to itself or its negative) are not in the circuit yet; they need a reversible if-then-select and are the next job. The single-bit readout circuit is also still to come. Neither is a surprise; both are on the plan.

**See it yourself:**
```
git checkout v3-research
cd research/v3-sampler
python3 -I circuit/test_modarith.py   # exhaustive, prints PASS
python3 -I circuit/test_projadd.py    # 10k+ cases vs reference, prints PASS
```

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
1. [DONE] Prove the compressed inversion-free sampler recovers `d` (toy, exact).
2. [DONE, generic case] Reversible circuit for the inversion-free projective arithmetic, verified vs the reference. ← we are here
   [NEXT] exceptional cases + the single-bit readout circuit, then scalar multiplication.
3. Whole-attack cost script: score this against the archived board circuits on the new metric.
4. The v3 challenge reference: metric, exactness contract, hardware-profile tracks.
5. The paper.
