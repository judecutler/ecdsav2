#!/usr/bin/env python3
"""Phase A: classical validation of the compressed, inversion-free ECDLP sampler.

The claim this file settles (go/no-go for the whole v3 direction):
  A single-bit, projective-invariant, inversion-free readout of [k]P still produces
  a measurement distribution from which the discrete log d (Q = [d]P) is recovered.

We do NOT build a circuit here. We compute the exact quantum measurement distribution
of the two-control-register Shor/Ekera ECDLP sampler, with the output register replaced
by one bit g(k) = chi_q(projective-invariant function of [k]P), and check:

  1. g computed inversion-free from projective (X:Z) equals the affine Legendre readout.
  2. ALL measurement mass lies on the line v == d*u (mod r). (=> exact d recovery.)
  3. d is recovered from any run with gcd(u, r) = 1.
  4. The only cost of compression is the distribution over u (|g_hat(u)|^2): we report
     the "useless-run" probability P(u == 0) for the compressed vs the full-register sampler.

Math (derived in RESUME.md): for control registers of size r with QFT_r, output bit m,
  P(u, v | m)  proportional to  [v == d*u mod r] * |g_hat_m(u)|^2,
  g_hat_m(u) = sum_{k: g(k)=m} omega_r^{k u}.
So the line v == d*u is exact regardless of g; compression only reshapes P(u).

Pure Python + cmath, no third-party deps. Run: python3 -I sampler_toy.py
"""
import cmath
import math


# ---------------------------------------------------------------------------
# Toy elliptic curve over F_q in short Weierstrass form y^2 = x^3 + a x + b.
# Affine for the reference; projective (X:Y:Z), x=X/Z, y=Y/Z, inversion-free, for the readout.
# ---------------------------------------------------------------------------
class Curve:
    def __init__(self, q, a, b):
        assert all(pow_is_prime(q) for _ in [0]), "q must be prime for F_q"
        self.q, self.a, self.b = q, a % q, b % q
        assert (4 * self.a**3 + 27 * self.b**2) % q != 0, "singular curve"

    def is_on(self, P):
        if P is None:
            return True
        x, y = P
        return (y * y - (x * x * x + self.a * x + self.b)) % self.q == 0


def pow_is_prime(n):
    if n < 2:
        return False
    for p in range(2, int(n**0.5) + 1):
        if n % p == 0:
            return False
    return True


def aff_add(C, P, Qp):
    """Affine addition; None is the point at infinity. Uses inversion (reference only)."""
    q = C.q
    if P is None:
        return Qp
    if Qp is None:
        return P
    x1, y1 = P
    x2, y2 = Qp
    if x1 == x2 and (y1 + y2) % q == 0:
        return None
    if P == Qp:
        lam = (3 * x1 * x1 + C.a) * inv(2 * y1 % q, q) % q
    else:
        lam = (y2 - y1) * inv((x2 - x1) % q, q) % q
    x3 = (lam * lam - x1 - x2) % q
    y3 = (lam * (x1 - x3) - y1) % q
    return (x3, y3)


def aff_mul(C, k, P):
    R = None
    k %= order_hint[0] if order_hint else (1 << 62)
    base = P
    while k:
        if k & 1:
            R = aff_add(C, R, base)
        base = aff_add(C, base, base)
        k >>= 1
    return R


order_hint = []


def inv(a, q):
    return pow(a % q, q - 2, q)


# --- Projective, inversion-free (standard projective coordinates, x=X/Z, y=Y/Z) ---
# Addition/doubling formulas for y^2 = x^3 + a x + b in homogeneous projective coords.
# Source: standard; no field inversion anywhere. Identity = (0:1:0).
def proj_from_aff(P):
    if P is None:
        return (0, 1, 0)
    return (P[0], P[1], 1)


def proj_is_inf(Pp):
    return Pp[2] == 0


def proj_dbl(C, Pp):
    q, a = C.q, C.a
    X1, Y1, Z1 = Pp
    if Y1 % q == 0 or Z1 % q == 0:
        return (0, 1, 0)
    W = (a * Z1 * Z1 + 3 * X1 * X1) % q
    S = (Y1 * Z1) % q
    B = (X1 * Y1 * S) % q
    H = (W * W - 8 * B) % q
    X3 = (2 * H * S) % q
    Y3 = (W * (4 * B - H) - 8 * Y1 * Y1 * S * S) % q
    Z3 = (8 * S * S * S) % q
    return (X3 % q, Y3 % q, Z3 % q)


def proj_add(C, Pp, Qp):
    q, a = C.q, C.a
    if proj_is_inf(Pp):
        return Qp
    if proj_is_inf(Qp):
        return Pp
    X1, Y1, Z1 = Pp
    X2, Y2, Z2 = Qp
    U1 = (Y2 * Z1) % q
    U2 = (Y1 * Z2) % q
    V1 = (X2 * Z1) % q
    V2 = (X1 * Z2) % q
    if V1 == V2:
        if U1 == U2:
            return proj_dbl(C, Pp)
        return (0, 1, 0)  # P + (-P) = inf
    V = (V1 - V2) % q
    U = (U1 - U2) % q
    W = (Z1 * Z2) % q
    Vsq = (V * V) % q
    Vcu = (Vsq * V) % q
    A_ = (U * U * W - Vcu - 2 * Vsq * V2) % q
    X3 = (V * A_) % q
    Y3 = (U * (Vsq * V2 - A_) - Vcu * U2) % q
    Z3 = (Vcu * W) % q
    return (X3 % q, Y3 % q, Z3 % q)


def proj_mul(C, k, Pp):
    R = (0, 1, 0)
    base = Pp
    while k:
        if k & 1:
            R = proj_add(C, R, base)
        base = proj_dbl(C, base)
        k >>= 1
    return R


def proj_to_aff(C, Pp):
    if proj_is_inf(Pp):
        return None
    q = C.q
    zi = inv(Pp[2], q)
    return ((Pp[0] * zi) % q, (Pp[1] * zi) % q)


# ---------------------------------------------------------------------------
# Legendre symbol and the compressed readout.
# g(k) = [ legendre_q( f(point) ) == -1 ]  as a bit in {0,1}; infinity -> fixed 0.
# Affine f: f = x. Projective-invariant f: X*Z, since legendre(X/Z) = legendre(X*Z)
# and under scaling (lX:lY:lZ), X*Z -> l^2 X*Z (a square), so the symbol is invariant.
# This is the inversion-free readout: only a product and a Legendre symbol, no 1/Z.
# ---------------------------------------------------------------------------
def legendre(a, q):
    a %= q
    if a == 0:
        return 0
    return 1 if pow(a, (q - 1) // 2, q) == 1 else -1


def g_affine(C, k, P):
    Pt = aff_mul(C, k, P)
    if Pt is None:
        return 0
    return 1 if legendre(Pt[0], C.q) == -1 else 0


def g_projective(C, k, Pp):
    R = proj_mul(C, k, Pp)
    if proj_is_inf(R):
        return 0
    X, Y, Z = R
    return 1 if legendre((X * Z) % C.q, C.q) == -1 else 0


# ---------------------------------------------------------------------------
# Exact sampler distribution over (u, v) in Z_r^2 with output bit m.
# Returns dict (u,v)->prob (joint over m already summed), plus per-u marginal.
# ---------------------------------------------------------------------------
def exact_distribution(gvals, r, d):
    """gvals[k] in {0,1} for k in range(r). Returns (joint P(u,v), P_useless, line_ok)."""
    w = [cmath.exp(2j * math.pi * t / r) for t in range(r)]
    # g_hat_m(u) = sum_{k: g(k)=m} w^{k u}
    joint = {}
    total = 0.0
    line_ok = True
    for m in (0, 1):
        ks = [k for k in range(r) if gvals[k] == m]
        if not ks:
            continue
        for u in range(r):
            gh = sum(w[(k * u) % r] for k in ks)
            amp2 = abs(gh) ** 2
            if amp2 < 1e-9:
                continue
            v = (d * u) % r  # the derived line
            joint[(u, v)] = joint.get((u, v), 0.0) + amp2
            total += amp2
            # verify no mass off the line by checking a few off-line (u,v')
    for key in joint:
        joint[key] /= total
    # useless runs: u == 0 (v also 0), gcd(u,r)!=1 can't invert
    p_useless = sum(p for (u, v), p in joint.items() if math.gcd(u, r) != 1)
    return joint, p_useless


def full_distribution_no_assumption(gvals, r, d):
    """Independent check: compute P(u,v) over the FULL r x r grid directly from the
    sampler state, assuming nothing about a line. Returns (max off-line mass, on-line mass).
    State (post-measure output m): uniform over {(a,b): g(a+db mod r)=m}. QFT both registers.
    P(u,v|m) = (1/(r^2 N_m)) |sum_{a,b in S_m} w^{au+bv}|^2 ; we sum over m weighted by N_m/r^2."""
    w = [cmath.exp(2j * math.pi * t / r) for t in range(r)]
    off = 0.0
    on = 0.0
    for m in (0, 1):
        S = [(a, b) for a in range(r) for b in range(r) if gvals[(a + d * b) % r] == m]
        if not S:
            continue
        weight = len(S) / (r * r)  # P(measure output = m)
        norm = len(S)
        for u in range(r):
            for v in range(r):
                amp = sum(w[(a * u + b * v) % r] for a, b in S)
                p = weight * (abs(amp) ** 2) / (r * r * norm)
                if (v - d * u) % r == 0:
                    on += p
                else:
                    off = max(off, p)
    return off, on


def recover_d(joint, r):
    """From any (u,v) with gcd(u,r)=1, d = v * u^{-1} mod r. Check all agree."""
    ds = set()
    for (u, v), p in joint.items():
        if math.gcd(u, r) == 1:
            ds.add((v * inv(u, r)) % r)
    return ds


def full_register_useless(r):
    """Uncompressed sampler (full point register, injective on Z_r): P(u=0) = 1/r.
    With full output, g_hat is a delta; every u equally likely, only u=0 useless."""
    return 1.0 / r


# ---------------------------------------------------------------------------
def find_curve_and_generator(q, a, b):
    C = Curve(q, a, b)
    # find a point of large prime order
    pts = [None]
    for x in range(q):
        rhs = (x * x * x + a * x + b) % q
        for y in range(q):
            if (y * y) % q == rhs:
                pts.append((x, y))
    best = None
    for P in pts:
        if P is None:
            continue
        # order of P
        o = 1
        R = P
        while R is not None:
            R = aff_add(C, R, P)
            o += 1
            if o > 2 * q:
                break
        if pow_is_prime(o) and (best is None or o > best[1]):
            best = (P, o)
    return C, best[0], best[1]


def run_instance(q, a, b, label):
    global order_hint
    C, P, r = find_curve_and_generator(q, a, b)
    order_hint = [r]
    Pp = proj_from_aff(P)
    # 1. inversion-free readout equals affine readout, for every k
    mism = [k for k in range(r) if g_affine(C, k, P) != g_projective(C, k, Pp)]
    assert not mism, f"{label}: proj/affine readout mismatch at {mism[:5]}"
    gvals = [g_affine(C, k, P) for k in range(r)]
    nonconst = len(set(gvals)) > 1
    # 2,3,4 for a few planted d
    results = []
    for d in (1, 2, 3, 5, r // 2, r - 1):
        d %= r
        if d == 0:
            continue
        joint, p_useless = exact_distribution(gvals, r, d)
        ds = recover_d(joint, r)
        off, on = full_distribution_no_assumption(gvals, r, d)
        ok = ds == {d} and off < 1e-9 and on > 1 - 1e-6
        results.append((d, ok, p_useless, off, on))
    balance = sum(gvals) / r
    return {
        "label": label, "q": q, "a": a, "b": b, "P": P, "r": r,
        "readout_inversion_free_matches_affine": True,
        "g_nonconstant": nonconst, "g_balance": balance,
        "recover": results, "full_register_useless": full_register_useless(r),
    }


def main():
    instances = [
        (23, 1, 1, "E1/F23"),
        (41, 2, 3, "E2/F41"),
        (97, 3, 5, "E3/F97"),
        (211, 1, 7, "E4/F211"),
    ]
    print("=" * 78)
    print("Phase A: compressed inversion-free ECDLP sampler, exact toy validation")
    print("=" * 78)
    all_ok = True
    for q, a, b, label in instances:
        R = run_instance(q, a, b, label)
        every = all(ok for ok in (row[1] for row in R["recover"]))
        all_ok = all_ok and every and R["g_nonconstant"]
        pu = R["recover"][0][2]
        max_off = max(row[3] for row in R["recover"])
        print(f"\n{label}: q={q} y^2=x^3+{a}x+{b}  P={R['P']} order r={R['r']}")
        print(f"  inversion-free readout == affine readout: PASS (all k)")
        print(f"  g non-constant: {R['g_nonconstant']}  balance(P[g=1])={R['g_balance']:.3f}")
        print(f"  off-line mass (independent full r*r check, must be ~0): {max_off:.2e}")
        print(f"  d recovery (planted d -> recovered set == {{d}}):")
        for d, ok, p_useless, off, on in R["recover"]:
            print(f"    d={d:3d}: {'PASS' if ok else 'FAIL'}   P(useless run)={p_useless:.4f}  on-line mass={on:.6f}")
        print(f"  compression cost: compressed P(useless)~{pu:.4f} vs full-register P(useless)={R['full_register_useless']:.4f}")
    print("\n" + "=" * 78)
    print(f"OVERALL: {'PASS - compressed inversion-free sampler recovers d' if all_ok else 'FAIL'}")
    print("=" * 78)
    return 0 if all_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
