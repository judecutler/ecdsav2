"""Reversible inversion-free projective point addition (generic case only).

Mirrors sampler_toy.proj_add's non-exceptional branch line for line:
  U1=Y2*Z1  U2=Y1*Z2  V1=X2*Z1  V2=X1*Z2  V=V1-V2  U=U1-U2  W=Z1*Z2
  Vsq=V^2  Vcu=Vsq*V  A=U^2*W - Vcu - 2*Vsq*V2
  X3=V*A   Y3=U*(Vsq*V2 - A) - Vcu*U2   Z3=Vcu*W

DEFERRED (not handled, loudly): the exceptional branches of the reference, namely
P1 or P2 = infinity (Z=0) and V1 == V2 (P2 = +-P1, doubling or P + -P). A fixed gate list
has no classical branch, so those need a reversible select (compare + controlled swap) that
a later phase must add. Valid domain here: Z1, Z2 != 0 and x1 != x2.
"""
from builder import Circuit, Reg
from modarith import modadd, modsub, modmul


def proj_add(C: Circuit, P1, P2, out, q: int) -> None:
    """out ^= P1 + P2 (projective, generic inputs). P1, P2 preserved; out starts (0,0,0)."""
    X1, Y1, Z1 = P1
    X2, Y2, Z2 = P2
    w = len(X1)
    sc, start = C.scope(), C.mark()

    def new(op, a, b):
        r = C.alloc(w)
        op(C, a, b, r, q)
        return r

    mul = lambda a, b: new(modmul, a, b)
    add = lambda a, b: new(modadd, a, b)
    sub = lambda a, b: new(modsub, a, b)

    U1, U2 = mul(Y2, Z1), mul(Y1, Z2)
    V1, V2 = mul(X2, Z1), mul(X1, Z2)
    V, U = sub(V1, V2), sub(U1, U2)
    W = mul(Z1, Z2)
    Vsq = mul(V, V)
    Vcu = mul(Vsq, V)
    VsqV2 = mul(Vsq, V2)
    A = sub(sub(mul(mul(U, U), W), Vcu), add(VsqV2, VsqV2))
    X3 = mul(V, A)
    Y3 = sub(mul(U, sub(VsqV2, A)), mul(Vcu, U2))
    Z3 = mul(Vcu, W)
    end = C.mark()
    for src, dst in zip((X3, Y3, Z3), out):
        for x, y in zip(src, dst):
            C.cx(x, y)
    C.uncompute(start, end)
    C.end_scope(sc)
