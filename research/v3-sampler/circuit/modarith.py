"""Out-of-place reversible modular arithmetic mod a prime parameter q.

Convention: f(C, a, b, out) does out ^= f(a, b) mod q. out is a w-bit Reg that starts 0.
a and b are only read, and every scratch wire is returned to 0.
Each op is compute -> CX copy into out -> uncompute (the compute gates reversed).
No modular inversion appears anywhere.
"""
from builder import Circuit, Reg


def width(q: int) -> int:
    """Bits per field element: smallest w with 2**w >= q."""
    return (q - 1).bit_length()


# -- n-bit ripple-carry adder (Vedral-Barenco-Ekert), b += a mod 2**n ------------
def _carry(g, c, a, b, cn):
    g += [("CCX", a, b, cn), ("CX", a, b), ("CCX", c, b, cn)]


def _carry_inv(g, c, a, b, cn):
    g += [("CCX", c, b, cn), ("CX", a, b), ("CCX", a, b, cn)]


def add_inplace(C: Circuit, a: Reg, b: Reg, subtract: bool = False) -> None:
    """b += a (or b -= a) mod 2**n. a and b have equal length n. a is preserved."""
    n = len(a)
    assert len(b) == n
    c = C.alloc(n)  # carries; c[0] stays 0
    g: list = []
    for i in range(n - 1):
        _carry(g, c[i], a[i], b[i], c[i + 1])
    g += [("CX", a[n - 1], b[n - 1]), ("CX", c[n - 1], b[n - 1])]
    for i in range(n - 2, -1, -1):
        _carry_inv(g, c[i], a[i], b[i], c[i + 1])
        g += [("CX", a[i], b[i]), ("CX", c[i], b[i])]
    C.ops.extend(reversed(g) if subtract else g)
    C.free(c)


def add_const(C: Circuit, k: int, t: Reg, ctrl: int | None = None, subtract: bool = False) -> None:
    """t += k (or -= k) mod 2**len(t), only when wire ctrl is 1 (always if ctrl is None)."""
    kr = C.alloc(len(t))
    bits = [i for i in range(len(t)) if (k >> i) & 1]
    for i in bits:
        C.x(kr[i]) if ctrl is None else C.cx(ctrl, kr[i])
    add_inplace(C, kr, t, subtract)
    for i in bits:
        C.x(kr[i]) if ctrl is None else C.cx(ctrl, kr[i])
    C.free(kr)


def _copy_padded(C: Circuit, a: Reg) -> Reg:
    """Fresh (w+1)-bit register holding a, top bit 0."""
    t = C.alloc(len(a) + 1)
    for x, y in zip(a, t):
        C.cx(x, y)
    return t


def _pad(C: Circuit, b: Reg):
    """b zero-extended by one fresh 0 wire. Returns (padded, wire to free afterwards)."""
    z = C.alloc(1)
    return b + z, z


def _fold(C: Circuit, t: Reg, q: int) -> None:
    """t holds a value in [-q, 2q-2] as (w+1)-bit two's complement. Make it t mod q:
    the top bit is 1 exactly when the value is negative."""
    f = C.alloc(1)
    C.cx(t[-1], f[0])
    add_const(C, q, t, ctrl=f[0])
    # f is left set; the caller's uncompute clears it.


def _finish(C: Circuit, start: int, t: Reg, out: Reg) -> None:
    """Copy t's low bits to out, uncompute everything since `start` (except the copy)."""
    end = C.mark()
    for x, y in zip(t, out):
        C.cx(x, y)
    C.uncompute(start, end)


def modadd(C: Circuit, a: Reg, b: Reg, out: Reg, q: int) -> None:
    """out ^= (a + b) mod q."""
    sc, start = C.scope(), C.mark()
    t = _copy_padded(C, a)
    bp, z = _pad(C, b)
    add_inplace(C, bp, t)                    # t = a + b in [0, 2q-2]
    add_const(C, q, t, subtract=True)        # t = a + b - q in [-q, q-2]
    _fold(C, t, q)                           # t = (a + b) mod q
    _finish(C, start, t, out)
    C.end_scope(sc)


def modsub(C: Circuit, a: Reg, b: Reg, out: Reg, q: int) -> None:
    """out ^= (a - b) mod q."""
    sc, start = C.scope(), C.mark()
    t = _copy_padded(C, a)
    bp, z = _pad(C, b)
    add_inplace(C, bp, t, subtract=True)     # t = a - b in [-(q-1), q-1]
    _fold(C, t, q)                           # t = (a - b) mod q
    _finish(C, start, t, out)
    C.end_scope(sc)


def modmul(C: Circuit, a: Reg, b: Reg, out: Reg, q: int) -> None:
    """out ^= (a * b) mod q. Schoolbook MSB-first: acc <- (2*acc + b_i * a) mod q."""
    if set(a) & set(b):                      # squaring: CCX needs distinct wires
        b2 = C.alloc(len(b))
        for x, y in zip(b, b2):
            C.cx(x, y)
        modmul(C, a, b2, out, q)
        for x, y in zip(b, b2):
            C.cx(x, y)
        C.free(b2)
        return
    w = len(a)
    sc, start = C.scope(), C.mark()
    acc = C.alloc(w)                         # acc = 0
    for i in range(len(b) - 1, -1, -1):
        dup = C.alloc(w)
        for x, y in zip(acc, dup):
            C.cx(x, y)
        dbl = C.alloc(w)
        modadd(C, acc, dup, dbl, q)          # 2*acc mod q
        masked = C.alloc(w)
        for x, y in zip(a, masked):
            C.ccx(b[i], x, y)                # b_i * a
        nxt = C.alloc(w)
        modadd(C, dbl, masked, nxt, q)
        acc = nxt                            # old regs stay (garbage) until the uncompute
    end = C.mark()
    for x, y in zip(acc, out):
        C.cx(x, y)
    C.uncompute(start, end)
    C.end_scope(sc)
