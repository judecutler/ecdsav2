#!/usr/bin/env python3
"""Phase A+: the compression tradeoff curve.

Phase A showed a single-bit readout recovers d but puts ~1/2 the measurement mass on the
useless u=0 outcome (vs 1/r for a full point register). This file quantifies the dial:
how P(useless run) falls as the readout widens from 1 bit toward a full register.

Derivation (RESUME.md): with a b-bit readout partitioning Z_r into cells of sizes n_m,
  P(u=0) = (1/r^2) * sum_m n_m^2   (by Parseval; all other mass stays on v=d*u).
Minimized when cells are equal: 2^b cells of size r/2^b give P(u=0) = 1/2^b, down to the
1/r floor once 2^b >= r (injective readout = full register). So:

  P(useless run) ~ max(1/2^b, 1/r),   a clean 1-bit -> full-register interpolation.

Each readout bit is a projective-INVARIANT Legendre symbol legendre_q((X - c*Z)*Z): degree-2
homogeneous, so invariant under (X:Y:Z)->(lX:lY:lZ), and inversion-free. Different constants c
give different bits. We confirm the predicted P(useless) empirically and report runs-to-recover.

Reuses sampler_toy.py. Run: python3 -I compression_tradeoff.py
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sampler_toy as S


def readout_bits(C, k, Pp, consts):
    """b-bit projective-invariant inversion-free readout of [k]P: for each c,
    bit = [legendre_q((X - c*Z)*Z) == -1]. Infinity -> all zeros."""
    R = S.proj_mul(C, k, Pp)
    if S.proj_is_inf(R):
        return 0
    X, Y, Z = R
    val = 0
    for i, c in enumerate(consts):
        t = ((X - c * Z) * Z) % C.q
        if S.legendre(t, C.q) == -1:
            val |= (1 << i)
    return val


def p_useless_for_readout(C, Pp, r, consts):
    """Exact P(u=0) = (1/r^2) sum_m n_m^2 for the readout g: Z_r -> cells."""
    cells = {}
    for k in range(r):
        m = readout_bits(C, k, Pp, consts)
        cells[m] = cells.get(m, 0) + 1
    p0 = sum(n * n for n in cells.values()) / (r * r)
    return p0, len(cells)


def run():
    S.order_hint = []
    print("=" * 84)
    print("Compression tradeoff: P(useless run) vs readout width b   (predict max(1/2^b, 1/r))")
    print("=" * 84)
    for q, a, b_, label in [(211, 1, 7, "E4/F211"), (97, 3, 5, "E3/F97"), (23, 1, 1, "E1/F23")]:
        C, P, r = S.find_curve_and_generator(q, a, b_)
        S.order_hint = [r]
        Pp = S.proj_from_aff(P)
        print(f"\n{label}: order r={r}  (full-register floor 1/r = {1/r:.4f})")
        print(f"  bits b | cells used | P(useless) measured | predicted max(1/2^b,1/r) | ~runs to recover")
        consts = list(range(0, 40))  # pool of distinct invariant bits
        for b in range(1, min(r.bit_length() + 1, 8)):
            p0, ncells = p_useless_for_readout(C, Pp, r, consts[:b])
            pred = max(1 / (2**b), 1 / r)
            # expected runs until a useful (gcd(u,r)=1) outcome: ~1/(1-p0) for prime r
            runs = 1 / (1 - p0) if p0 < 1 else float("inf")
            print(f"    {b:3d}   |   {ncells:4d}      |     {p0:.4f}          |      {pred:.4f}            |   {runs:.2f}")
    print("\n" + "=" * 84)
    print("Reading (report the MEASURED column, not the 1/2^b bound): the predicted max(1/2^b,1/r)")
    print("is the best case of equal, independent cells. Real projective-invariant Legendre bits")
    print("are strongly CORRELATED, so measured P(useless) sits well above 1/2^b and does not halve")
    print("per bit (e.g. F97 b=1 and b=2 are identical: a whole extra bit bought nothing). The honest")
    print("claim: widening the readout lowers P(useless) monotonically toward the 1/r full-register")
    print("floor, but the rate is readout-dependent and must be measured, not assumed to be 1/2^b.")
    print("=" * 84)


if __name__ == "__main__":
    run()
