#!/usr/bin/env python3 -I
"""Exhaustive test of modadd/modsub/modmul over all (a,b) in F_q^2 for q in {5,7,11,13}.
One bit-sliced lane per (a,b). Checks: out == expected, a and b preserved, every other wire 0."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from builder import Circuit
from modarith import modadd, modsub, modmul, width
from sim import run, pack, unpack

OPS = {
    "modadd": (modadd, lambda a, b, q: (a + b) % q),
    "modsub": (modsub, lambda a, b, q: (a - b) % q),
    "modmul": (modmul, lambda a, b, q: (a * b) % q),
}


def check(name, q):
    fn, ref = OPS[name]
    w = width(q)
    C = Circuit()
    a, b, out = C.alloc(w), C.alloc(w), C.alloc(w)
    fn(C, a, b, out, q)
    pairs = [(x, y) for x in range(q) for y in range(q)]
    lanes = len(pairs)
    init = {**pack([p[0] for p in pairs], a), **pack([p[1] for p in pairs], b)}
    s = run(C.ops, C.num_wires, init, lanes)
    errs = 0
    for lane, got in enumerate(unpack(s, out, lanes)):
        if got != ref(*pairs[lane], q):
            errs += 1
    errs += sum(x != y for x, y in zip(unpack(s, a, lanes), [p[0] for p in pairs]))
    errs += sum(x != y for x, y in zip(unpack(s, b, lanes), [p[1] for p in pairs]))
    dirty = [i for i in range(C.num_wires) if i not in set(a + b + out) and s[i] != 0]
    print(f"{name} q={q:2d}: {lanes:3d} cases, CCX={C.ccx_count():5d}, gates={len(C.ops):6d}, "
          f"wires={C.num_wires:3d}, errors={errs}, dirty_wires={len(dirty)}")
    return errs == 0 and not dirty


if __name__ == "__main__":
    ok = all([check(n, q) for q in (5, 7, 11, 13) for n in OPS])
    print("PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)
