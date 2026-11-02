//! Exact reversible integer and mod-p arithmetic. Registers are little-endian
//! qubit lists. Every function leaves its temporaries at |0⟩ and freed.

use super::gates::{Builder, Q};
use alloy_primitives::U256;

/// One addend bit: a known constant or a qubit. A constant multiple of a
/// control qubit is written with `Src::Q(ctl)` at every set bit.
#[derive(Clone, Copy, PartialEq)]
pub enum Src {
    Zero,
    One,
    Q(Q),
}

pub fn qs(a: &[Q]) -> Vec<Src> {
    a.iter().map(|&q| Src::Q(q)).collect()
}

/// The low n bits of k, each set bit replaced by `ctl` when a control is given.
pub fn konst(k: U256, n: usize, ctl: Option<Q>) -> Vec<Src> {
    (0..n)
        .map(|i| match (k.bit(i), ctl) {
            (false, _) => Src::Zero,
            (true, None) => Src::One,
            (true, Some(c)) => Src::Q(c),
        })
        .collect()
}

fn toggle(b: &mut Builder, s: Src, t: Q) {
    match s {
        Src::Zero => {}
        Src::One => b.x(t),
        Src::Q(q) => b.cx(q, t),
    }
}

/// Carry chain of a + t over m bits. Afterwards t[i] holds t_i ^ c_i for
/// i >= 1 and c[i] holds the carry into bit i+1. The addend is never
/// modified, so one control qubit may appear at many positions.
fn carries(b: &mut Builder, a: &[Src], t: &[Q], m: usize) -> Vec<Q> {
    let c = b.alloc_n(m);
    for i in 0..m {
        if i == 0 {
            match a[0] {
                Src::Zero => {}
                Src::One => b.cx(t[0], c[0]),
                Src::Q(q) => b.and(q, t[0], c[0]),
            }
        } else {
            // c_{i+1} = (t ^ c)(a ^ c) ^ c
            b.cx(c[i - 1], t[i]);
            toggle(b, a[i], c[i - 1]);
            b.and(t[i], c[i - 1], c[i]);
            toggle(b, a[i], c[i - 1]);
            b.cx(c[i - 1], c[i]);
        }
    }
    c
}

/// Inverse of `carries`; with `sum` it leaves t = t + a on the way down.
fn uncarries(b: &mut Builder, a: &[Src], t: &[Q], c: Vec<Q>, sum: bool) {
    for i in (0..c.len()).rev() {
        if i == 0 {
            match a[0] {
                Src::Zero => {}
                Src::One => b.cx(t[0], c[0]),
                Src::Q(q) => b.unand(q, t[0], c[0]),
            }
        } else {
            b.cx(c[i - 1], c[i]);
            toggle(b, a[i], c[i - 1]);
            b.unand(t[i], c[i - 1], c[i]);
            toggle(b, a[i], c[i - 1]);
            if !sum {
                b.cx(c[i - 1], t[i]);
            }
        }
        if sum {
            toggle(b, a[i], t[i]);
        }
    }
    b.free_n(&c);
}

/// t += a (mod 2^n). If `z` is given, z ^= carry out. One Toffoli per bit
/// below the top, carries cleared by measurement.
pub fn add(b: &mut Builder, a: &[Src], t: &[Q], z: Option<Q>) {
    let n = t.len();
    assert_eq!(n, a.len());
    let m = if z.is_some() { n } else { n - 1 };
    let c = carries(b, a, t, m);
    match z {
        Some(z) => b.cx(c[n - 1], z),
        None => {
            toggle(b, a[n - 1], t[n - 1]);
            if n > 1 {
                b.cx(c[n - 2], t[n - 1]);
            }
        }
    }
    uncarries(b, a, t, c, true);
}

/// t -= a (mod 2^n). If `z` is given, z ^= [t < a] (borrow).
pub fn sub(b: &mut Builder, a: &[Src], t: &[Q], z: Option<Q>) {
    b.inverse_of(|b| add(b, a, t, z));
}

/// z ^= ctl·carry_out(a + t), t unchanged. n Toffolis (+1 if controlled).
pub fn carry_out(b: &mut Builder, a: &[Src], t: &[Q], z: Q, ctl: Option<Q>) {
    let n = t.len();
    let c = carries(b, a, t, n);
    match ctl {
        Some(k) => b.ccx(k, c[n - 1], z),
        None => b.cx(c[n - 1], z),
    }
    uncarries(b, a, t, c, false);
}

/// z ^= ctl·[t < a] for quantum a and t, both unchanged.
pub fn less_than(b: &mut Builder, a: &[Q], t: &[Q], z: Q, ctl: Option<Q>) {
    // [t < a] is the carry out of a + (2^n - 1 - t).
    for &q in t {
        b.x(q);
    }
    carry_out(b, &qs(a), t, z, ctl);
    for &q in t {
        b.x(q);
    }
}

/// Writes the constant k into fresh |0⟩ qubits (X gates only).
pub fn load_const(b: &mut Builder, k: U256, n: usize) -> Vec<Q> {
    let r = b.alloc_n(n);
    for (i, &q) in r.iter().enumerate() {
        if k.bit(i) {
            b.x(q);
        }
    }
    r
}

pub fn unload_const(b: &mut Builder, r: Vec<Q>, k: U256) {
    for (i, &q) in r.iter().enumerate() {
        if k.bit(i) {
            b.x(q);
        }
    }
    b.free_n(&r);
}

/// Controlled swap of two equal-length registers. n Toffolis.
pub fn cswap(b: &mut Builder, ctl: Q, x: &[Q], y: &[Q]) {
    for (&p, &q) in x.iter().zip(y) {
        b.cx(q, p);
        b.ccx(ctl, p, q);
        b.cx(q, p);
    }
}

/// t ^= ctl·s.
fn ctl_toggle(b: &mut Builder, ctl: Q, s: Src, t: Q) {
    match s {
        Src::Zero => {}
        Src::One => b.cx(ctl, t),
        Src::Q(q) => b.ccx(ctl, q, t),
    }
}

/// t ^= ctl·(s ^ c).
fn ctl_toggle_sum(b: &mut Builder, ctl: Q, s: Src, c: Q, t: Q) {
    toggle(b, s, c);
    b.ccx(ctl, c, t);
    toggle(b, s, c);
}

/// t += ctl·a (mod 2^n). If `z` is given, z ^= ctl·carry_out(a + t).
/// The carries of a + t are computed unconditionally and only the sum bits are
/// written under the control, one Toffoli per bit.
pub fn ctl_add(b: &mut Builder, ctl: Q, a: &[Src], t: &[Q], z: Option<Q>) {
    let n = t.len();
    assert_eq!(n, a.len());
    let m = if z.is_some() { n } else { n - 1 };
    let c = carries(b, a, t, m);
    match z {
        Some(z) => b.ccx(ctl, c[n - 1], z),
        None if n == 1 => ctl_toggle(b, ctl, a[0], t[0]),
        None => ctl_toggle_sum(b, ctl, a[n - 1], c[n - 2], t[n - 1]),
    }
    for i in (0..m).rev() {
        if i == 0 {
            match a[0] {
                Src::Zero => {}
                Src::One => b.cx(t[0], c[0]),
                Src::Q(q) => b.unand(q, t[0], c[0]),
            }
            ctl_toggle(b, ctl, a[0], t[0]);
        } else {
            b.cx(c[i - 1], c[i]);
            toggle(b, a[i], c[i - 1]);
            b.unand(t[i], c[i - 1], c[i]);
            toggle(b, a[i], c[i - 1]);
            b.cx(c[i - 1], t[i]);
            ctl_toggle_sum(b, ctl, a[i], c[i - 1], t[i]);
        }
    }
    b.free_n(&c);
}

/// t -= ctl·a (mod 2^n).
pub fn ctl_sub(b: &mut Builder, ctl: Q, a: &[Q], t: &[Q]) {
    b.inverse_of(|b| ctl_add(b, ctl, &qs(a), t, None));
}

/// Arithmetic modulo an odd prime p < 2^n. Inputs and outputs are reduced.
#[derive(Clone, Copy)]
pub struct Field {
    pub p: U256,
    pub n: usize,
}

impl Field {
    /// t = t + a mod p. About 4n Toffolis.
    pub fn add(&self, b: &mut Builder, a: &[Q], t: &[Q]) {
        let n = self.n;
        let z = b.alloc();
        add(b, &qs(a), t, Some(z));
        // (t,z) -= p as an (n+1)-bit number; z ends set iff a + t < p.
        let mut wide = t.to_vec();
        wide.push(z);
        sub(b, &konst(self.p, n + 1, None), &wide, None);
        // Add p back when the subtraction went negative.
        add(b, &konst(self.p, n, Some(z)), t, None);
        // z = [a + t < p] = not [result < a].
        less_than(b, a, t, z, None);
        b.x(z);
        b.free(z);
    }

    /// t = t - a mod p.
    pub fn sub(&self, b: &mut Builder, a: &[Q], t: &[Q]) {
        b.inverse_of(|b| self.add(b, a, t));
    }

    /// t = t + ctl·a mod p.
    fn ctl_add(&self, b: &mut Builder, ctl: Q, a: &[Q], t: &[Q]) {
        let n = self.n;
        let z = b.alloc();
        ctl_add(b, ctl, &qs(a), t, Some(z));
        let mut wide = t.to_vec();
        wide.push(z);
        sub(b, &konst(self.p, n + 1, None), &wide, None);
        add(b, &konst(self.p, n, Some(z)), t, None);
        // z = [ctl·a + t < p], which is 1 when ctl = 0 (t < p) and
        // not [result < a] when ctl = 1, so z = not (ctl·[result < a]).
        less_than(b, a, t, z, Some(ctl));
        b.x(z);
        b.free(z);
    }

    /// t = t - ctl·a mod p.
    pub fn ctl_sub(&self, b: &mut Builder, ctl: Q, a: &[Q], t: &[Q]) {
        b.inverse_of(|b| self.ctl_add(b, ctl, a, t));
    }

    /// Halving with the carry qubit supplied: in = t (n qubits), z |0⟩.
    /// Afterwards [t[1..], z] holds in/2 mod p and t[0] is |0⟩.
    fn halve_into(&self, b: &mut Builder, t: &[Q], z: Q) {
        let n = self.n;
        let odd = b.alloc();
        b.cx(t[0], odd);
        add(b, &konst(self.p, n, Some(odd)), t, Some(z));
        let out: Vec<Q> = t[1..].iter().copied().chain([z]).collect();
        // The input was odd iff the result exceeds h = (p-1)/2, which is the
        // carry out of out + (2^n - 1 - h).
        let not_h = (U256::MAX >> (256 - n)) - (self.p >> 1);
        carry_out(b, &konst(not_h, n, None), &out, odd, None);
        b.free(odd);
    }

    /// t = t/2 mod p. Relabels t in place. About 2n Toffolis.
    pub fn halve(&self, b: &mut Builder, t: &mut Vec<Q>) {
        let z = b.alloc();
        self.halve_into(b, t, z);
        let low = t.remove(0);
        t.push(z);
        b.free(low);
    }

    /// t = 2t mod p. Relabels t in place.
    pub fn double(&self, b: &mut Builder, t: &mut Vec<Q>) {
        let low = b.alloc();
        let z = t.pop().unwrap();
        t.insert(0, low);
        let full = t.clone();
        b.inverse_of(|b| self.halve_into(b, &full, z));
        b.free(z);
    }

    /// t = t - a^2 mod p, by shift-and-subtract. About 9n^2 Toffolis.
    pub fn sub_square(&self, b: &mut Builder, a: &[Q], t: &[Q]) {
        let n = self.n;
        let mut s = b.alloc_n(n);
        for (&q, &r) in a.iter().zip(&s) {
            b.cx(q, r);
        }
        for i in 0..n {
            self.ctl_sub(b, a[i], &s, t);
            if i + 1 < n {
                self.double(b, &mut s);
            }
        }
        for _ in 0..n - 1 {
            self.halve(b, &mut s);
        }
        for (&q, &r) in a.iter().zip(&s) {
            b.cx(q, r);
        }
        b.free_n(&s);
    }
}
