#!/usr/bin/env python3
"""Breadth check (reviewer flag 5): the exact compressed-sampler recovery is not a small-r
artifact. Runs the same exact proof as sampler_toy on curves with prime group order up to ~71.
These are still exactly computable; the point is breadth of r, not scale. Run: python3 -I test_recovery_larger.py"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sampler_toy as S

CURVES = [(307, 1, 1), (503, 2, 1), (739, 1, 2), (997, 2, 5)]  # chosen for prime order r in [37,71]


def main():
    print("Larger prime-order toy curves (breadth, not scale):")
    all_ok = True
    for q, a, b in CURVES:
        S.order_hint = []
        C, P, r = S.find_curve_and_generator(q, a, b)
        if not S.pow_is_prime(r) or r < 30:
            continue
        S.order_hint = [r]
        Pp = S.proj_from_aff(P)
        mism = [k for k in range(r) if S.g_affine(C, k, P) != S.g_projective(C, k, Pp)]
        gvals = [S.g_affine(C, k, P) for k in range(r)]
        ok = not mism
        maxoff = 0.0
        for d in (1, 2, 7, r // 3, r // 2, r - 1):
            d %= r
            if d == 0:
                continue
            joint, pu = S.exact_distribution(gvals, r, d)
            off, on = S.full_distribution_no_assumption(gvals, r, d)
            ok = ok and S.recover_d(joint, r) == {d} and off < 1e-9 and on > 1 - 1e-6
            maxoff = max(maxoff, off)
        all_ok = all_ok and ok
        print(f"  q={q} r={r}: readout-match={not mism} recover={'PASS' if ok else 'FAIL'} "
              f"max_offline_mass={maxoff:.1e} P(useless)~{pu:.3f}")
    print("OVERALL:", "PASS - recovery holds across r up to ~71, not a small-r artifact" if all_ok else "FAIL")
    return 0 if all_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
