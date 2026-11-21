#!/usr/bin/env python3 -I
"""Verify the reversible proj_add against sampler_toy.proj_add on a toy curve.

SCOPE (loud): GENERIC DOMAIN ONLY. Exceptional cases (infinity inputs, P2 = +-P1) are
DEFERRED to a later phase; they are excluded here, not special-cased.
Inputs use random nonzero projective scalings, so non-unique representatives are exercised.
"""
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, ".."))
import sampler_toy as S
from builder import Circuit
from modarith import width
from projadd import proj_add
from sim import run, pack, unpack


def check(q, a, b, seed=1):
    cur = S.Curve(q, a, b)
    pts = [(x, y) for x in range(q) for y in range(q) if cur.is_on((x, y))]
    rng = random.Random(seed)
    cases = []
    for P1 in pts:
        for P2 in pts:
            if P1[0] == P2[0]:      # P2 = +-P1: deferred exceptional branch
                continue
            l1, l2 = rng.randrange(1, q), rng.randrange(1, q)
            cases.append(((P1[0] * l1 % q, P1[1] * l1 % q, l1), (P2[0] * l2 % q, P2[1] * l2 % q, l2)))
    w = width(q)
    C = Circuit()
    R1 = [C.alloc(w) for _ in range(3)]
    R2 = [C.alloc(w) for _ in range(3)]
    out = [C.alloc(w) for _ in range(3)]
    proj_add(C, R1, R2, out, q)
    lanes = len(cases)
    init = {}
    for reg, k, which in [(R1[i], i, 0) for i in range(3)] + [(R2[i], i, 1) for i in range(3)]:
        init.update(pack([c[which][k] for c in cases], reg))
    s = run(C.ops, C.num_wires, init, lanes)
    got = [unpack(s, r, lanes) for r in out]
    bad = 0
    for j, (p1, p2) in enumerate(cases):
        Xr, Yr, Zr = S.proj_add(cur, p1, p2)
        X3, Y3, Z3 = got[0][j], got[1][j], got[2][j]
        ok = Z3 != 0 and (X3 * Zr - Xr * Z3) % q == 0 and (Y3 * Zr - Yr * Z3) % q == 0
        bad += not ok
    inputs_ok = all(unpack(s, R1[i], lanes) == [c[0][i] for c in cases] and
                    unpack(s, R2[i], lanes) == [c[1][i] for c in cases] for i in range(3))
    keep = {x for reg in R1 + R2 + out for x in reg}
    dirty = [i for i in range(C.num_wires) if i not in keep and s[i]]
    print(f"proj_add F_{q} y^2=x^3+{a}x+{b}: {lanes} generic cases ({len(pts)} points), "
          f"CCX={C.ccx_count()}, gates={len(C.ops)}, wires={C.num_wires}, "
          f"mismatches={bad}, inputs_preserved={inputs_ok}, dirty_wires={len(dirty)}")
    return bad == 0 and inputs_ok and not dirty


if __name__ == "__main__":
    print("NOTE: generic domain only; infinity / P2=+-P1 branches DEFERRED to a later phase.")
    ok = check(23, 1, 1) and check(97, 3, 5)
    print("PASS (generic domain)" if ok else "FAIL")
    sys.exit(0 if ok else 1)
