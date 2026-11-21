"""Reversible circuit builder.

Gate set: X(a), CX(a,b), CCX(a,b,c), SWAP(a,b). Every gate is its own inverse,
so the inverse of a gate list is the same list reversed.

Data shape (one obvious form):
  Reg  = list[int]            little-endian wire indices (Reg[0] is the LSB)
  Gate = tuple                ("X",a) | ("CX",a,b) | ("CCX",a,b,c) | ("SWAP",a,b)
  ops  = list[Gate]
"""

Reg = list
Gate = tuple


class Circuit:
    def __init__(self):
        self.ops: list[Gate] = []
        self.num_wires = 0          # high-water mark; wire ids are 0..num_wires-1
        self._free: list[int] = []  # wires returned clean (value 0) and reusable
        self._live: list[int] = []  # wires allocated and not yet freed, in order

    # -- wires ------------------------------------------------------------
    def alloc(self, n: int) -> Reg:
        """Return n wires that currently hold 0."""
        reg = []
        for _ in range(n):
            if self._free:
                reg.append(self._free.pop())
            else:
                reg.append(self.num_wires)
                self.num_wires += 1
        self._live.extend(reg)
        return reg

    def free(self, reg: Reg) -> None:
        """Give wires back. Caller promises they hold 0 at this point."""
        for w in reg:
            self._live.remove(w)
        self._free.extend(reg)

    def scope(self) -> int:
        """Token for end_scope: remembers which wires were live."""
        return len(self._live)

    def end_scope(self, token: int) -> None:
        """Free every wire allocated since scope(). Caller promises they are clean
        (the block's gates were uncomputed)."""
        self.free(self._live[token:])

    # -- gates ------------------------------------------------------------
    def x(self, a):
        self.ops.append(("X", a))

    def cx(self, a, b):
        assert a != b
        self.ops.append(("CX", a, b))

    def ccx(self, a, b, c):
        assert len({a, b, c}) == 3, "CCX needs three distinct wires"
        self.ops.append(("CCX", a, b, c))

    def swap(self, a, b):
        assert a != b
        self.ops.append(("SWAP", a, b))

    # -- compute / uncompute ----------------------------------------------
    def mark(self) -> int:
        return len(self.ops)

    def uncompute(self, start: int, end: int) -> None:
        """Append the inverse of ops[start:end] (the same gates, reversed)."""
        self.ops.extend(reversed(self.ops[start:end]))

    def ccx_count(self) -> int:
        return sum(1 for g in self.ops if g[0] == "CCX")
