"""Bit-sliced simulator. Each wire is a Python int; bit j is that wire's value in lane j."""


def run(ops, num_wires, init, lanes):
    """init: {wire: mask}. Unlisted wires start 0. Returns the final state (list of masks)."""
    full = (1 << lanes) - 1
    s = [0] * num_wires
    for w, m in init.items():
        s[w] = m
    for g in ops:
        op = g[0]
        if op == "CCX":
            s[g[3]] ^= s[g[1]] & s[g[2]]
        elif op == "CX":
            s[g[2]] ^= s[g[1]]
        elif op == "X":
            s[g[1]] ^= full
        elif op == "SWAP":
            s[g[1]], s[g[2]] = s[g[2]], s[g[1]]
        else:
            raise ValueError(f"unknown gate {g}")
    return s


def pack(values, wires):
    """values[j] is lane j's integer; returns {wire: mask} for the little-endian reg `wires`."""
    return {w: sum(((v >> i) & 1) << j for j, v in enumerate(values))
            for i, w in enumerate(wires)}


def unpack(state, wires, lanes):
    """Return the per-lane integers held by reg `wires`."""
    return [sum(((state[w] >> j) & 1) << i for i, w in enumerate(wires))
            for j in range(lanes)]
