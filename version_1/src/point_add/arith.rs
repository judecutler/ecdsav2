//! Exact reversible integer and mod-p arithmetic. Registers are little-endian
//! qubit lists. Every function leaves its temporaries at |0⟩ and freed.

use super::gates::{Builder, Q};
use alloy_primitives::U256;

/// t += a (mod 2^n). If `z` is given, z ^= carry out. Gidney's adder: n-1
/// Toffolis (n with carry out), carries cleared by measurement.
pub fn add(b: &mut Builder, a: &[Q], t: &[Q], z: Option<Q>) {
    let n = a.len();
    assert_eq!(n, t.len());
    let m = if z.is_some() { n } else { n - 1 };
    let c = b.alloc_n(m);
    for i in 0..m {
        if i == 0 {
            b.and(a[0], t[0], c[0]);
        } else {
            b.cx(c[i - 1], a[i]);
            b.cx(c[i - 1], t[i]);
            b.and(a[i], t[i], c[i]);
            b.cx(c[i - 1], c[i]);
        }
    }
    if let Some(z) = z {
        b.cx(c[n - 1], z);
    } else {
        b.cx(a[n - 1], t[n - 1]);
        if n > 1 {
            b.cx(c[n - 2], t[n - 1]);
        }
    }
    for i in (0..m).rev() {
        if i == 0 {
            b.unand(a[0], t[0], c[0]);
            b.cx(a[0], t[0]);
        } else {
            b.cx(c[i - 1], c[i]);
            b.unand(a[i], t[i], c[i]);
            b.cx(c[i - 1], a[i]);
            b.cx(a[i], t[i]);
        }
    }
    b.free_n(&c);
}

/// t -= a (mod 2^n). If `z` is given, z ^= [t < a] (borrow).
pub fn sub(b: &mut Builder, a: &[Q], t: &[Q], z: Option<Q>) {
    b.inverse_of(|b| add(b, a, t, z));
}

/// z ^= [t < a], or z ^= ctl·[t < a]. Leaves a and t unchanged. n Toffolis.
pub fn less_than(b: &mut Builder, a: &[Q], t: &[Q], z: Q, ctl: Option<Q>) {
    let n = a.len();
    assert_eq!(n, t.len());
    // [t < a] is the carry out of a + (2^n - 1 - t).
    for &q in t {
        b.x(q);
    }
    let c = b.alloc_n(n);
    for i in 0..n {
        if i == 0 {
            b.and(a[0], t[0], c[0]);
        } else {
            b.cx(c[i - 1], a[i]);
            b.cx(c[i - 1], t[i]);
            b.and(a[i], t[i], c[i]);
            b.cx(c[i - 1], c[i]);
        }
    }
    match ctl {
        Some(k) => b.ccx(k, c[n - 1], z),
        None => b.cx(c[n - 1], z),
    }
    for i in (0..n).rev() {
        if i == 0 {
            b.unand(a[0], t[0], c[0]);
        } else {
            b.cx(c[i - 1], c[i]);
            b.unand(a[i], t[i], c[i]);
            b.cx(c[i - 1], a[i]);
            b.cx(c[i - 1], t[i]);
        }
    }
    b.free_n(&c);
    for &q in t {
        b.x(q);
    }
}

/// Writes the constant k into fresh |0⟩ qubits (Clifford only), optionally
/// controlled by `ctl`. Undo with `unload_const` using the same arguments.
pub fn load_const(b: &mut Builder, k: U256, n: usize, ctl: Option<Q>) -> Vec<Q> {
    let r = b.alloc_n(n);
    toggle_const(b, &r, k, ctl);
    r
}

pub fn unload_const(b: &mut Builder, r: Vec<Q>, k: U256, ctl: Option<Q>) {
    toggle_const(b, &r, k, ctl);
    b.free_n(&r);
}

fn toggle_const(b: &mut Builder, r: &[Q], k: U256, ctl: Option<Q>) {
    for (i, &q) in r.iter().enumerate() {
        if k.bit(i) {
            match ctl {
                Some(c) => b.cx(c, q),
                None => b.x(q),
            }
        }
    }
}

/// Controlled swap of two equal-length registers. n Toffolis.
pub fn cswap(b: &mut Builder, ctl: Q, x: &[Q], y: &[Q]) {
    for (&p, &q) in x.iter().zip(y) {
        b.cx(q, p);
        b.ccx(ctl, p, q);
        b.cx(q, p);
    }
}

/// Runs `f` on ctl·a, materialized in a scratch register (n Toffolis, cleared by measurement).
fn with_masked(b: &mut Builder, ctl: Q, a: &[Q], f: impl FnOnce(&mut Builder, &[Q])) {
    let m = b.alloc_n(a.len());
    for (&q, &r) in a.iter().zip(&m) {
        b.and(ctl, q, r);
    }
    f(b, &m);
    for (&q, &r) in a.iter().zip(&m) {
        b.unand(ctl, q, r);
    }
    b.free_n(&m);
}

/// t -= ctl·a (mod 2^n).
pub fn ctl_sub(b: &mut Builder, ctl: Q, a: &[Q], t: &[Q]) {
    with_masked(b, ctl, a, |b, m| sub(b, m, t, None));
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
        add(b, a, t, Some(z));
        // (t,z) -= p as an (n+1)-bit number; z ends set iff a + t < p.
        let mut wide = t.to_vec();
        wide.push(z);
        let k = load_const(b, self.p, n + 1, None);
        sub(b, &k, &wide, None);
        unload_const(b, k, self.p, None);
        // Add p back when the subtraction went negative.
        let k = load_const(b, self.p, n, Some(z));
        add(b, &k, t, None);
        unload_const(b, k, self.p, Some(z));
        // z = [a + t < p] = not [result < a].
        less_than(b, a, t, z, None);
        b.x(z);
        b.free(z);
    }

    /// t = t - a mod p.
    pub fn sub(&self, b: &mut Builder, a: &[Q], t: &[Q]) {
        b.inverse_of(|b| self.add(b, a, t));
    }

    /// t = t - ctl·a mod p.
    pub fn ctl_sub(&self, b: &mut Builder, ctl: Q, a: &[Q], t: &[Q]) {
        with_masked(b, ctl, a, |b, m| self.sub(b, m, t));
    }

    /// Halving with the carry qubit supplied: in = t (n qubits), z |0⟩.
    /// Afterwards [t[1..], z] holds in/2 mod p and t[0] is |0⟩.
    fn halve_into(&self, b: &mut Builder, t: &[Q], z: Q) {
        let n = self.n;
        let odd = b.alloc();
        b.cx(t[0], odd);
        let k = load_const(b, self.p, n, Some(odd));
        add(b, &k, t, Some(z));
        unload_const(b, k, self.p, Some(odd));
        let out: Vec<Q> = t[1..].iter().copied().chain([z]).collect();
        // The input was odd iff the result is above (p-1)/2.
        let h = load_const(b, self.p >> 1, n, None);
        less_than(b, &out, &h, odd, None);
        unload_const(b, h, self.p >> 1, None);
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
