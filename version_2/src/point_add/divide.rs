//! Exact in-place modular division (y, x) -> (y/x, 0) for any odd prime p.
//!
//! A binary GCD runs on (u, v) = (p, x). Each step is one of
//!   v even:            v = v/2
//!   v odd, v >= u:     v = (v-u)/2
//!   v odd, v <  u:     (u, v) = (v, (u-v)/2)
//! u stays odd, and while v != 0 each step at least halves u·v, which starts
//! below 4^n. So 2n steps always reach v = 0, u = gcd = 1. A finished state is
//! a fixed point of the first case, so no done flag is needed.
//! Applying the same steps mod p to (alpha, beta) = (0, y), where u = alpha·x and
//! v = beta·x mod p, ends at (y/x, 0). The step choices are kept as a 2-bit
//! transcript, and the GCD is run backwards afterwards to clear it.
//!
//! Width schedule. Before step k, u·v < 2^(2n-k) (it starts below p·x < 4^n
//! and at least halves per step), and u >= 1 always. So once k >= n, both u
//! and v fit in 2n - k bits, and the GCD step only touches that prefix. The
//! bits above it are |0⟩ on every input, so this is exact, not a sampled cut.

use super::arith::{cswap, ctl_sub, less_than, load_const, unload_const, Field};
use super::gates::{Builder, Q};

pub fn steps(f: &Field) -> usize {
    2 * f.n
}

/// Bits of u and v that can be nonzero before step k.
pub fn width(f: &Field, k: usize) -> usize {
    f.n.min(2 * f.n - k)
}

fn gcd_step(b: &mut Builder, u: &[Q], v: &mut Vec<Q>, odd: Q, swap: Q, w: usize) {
    let (uw, vw) = (&u[..w], &v[..w]);
    b.cx(vw[0], odd);
    less_than(b, uw, vw, swap, Some(odd));
    cswap(b, swap, uw, vw);
    ctl_sub(b, odd, uw, vw);
    // v is even now; dividing by 2 is a relabel of the full register.
    v.rotate_left(1);
}

fn target_step(f: &Field, b: &mut Builder, alpha: &mut Vec<Q>, beta: &mut Vec<Q>, odd: Q, swap: Q) {
    cswap(b, swap, alpha, beta);
    f.ctl_sub(b, odd, alpha, beta);
    f.halve(b, beta);
}

/// Consumes beta = y and returns a fresh register holding y/x, with x
/// unchanged at the end. Requires x != 0 mod p. beta ends at |0⟩ and is freed.
///
/// The GCD runs in place on x, which the target pass does not read. It
/// provably ends at (u, v) = (1, 0), so u and x are released during the target
/// pass and rebuilt by the reverse GCD, and alpha is only allocated once the
/// forward GCD's scratch is free.
pub fn divide(f: &Field, b: &mut Builder, x: &[Q], beta: Vec<Q>) -> Vec<Q> {
    let mut beta = beta;
    let n = f.n;
    let u = load_const(b, f.p, n);
    let k = steps(f);
    let transcript = b.alloc_n(2 * k);
    let gcd = |b: &mut Builder| {
        let mut v = x.to_vec();
        for i in 0..k {
            gcd_step(b, &u, &mut v, transcript[2 * i], transcript[2 * i + 1], width(f, i));
        }
        v
    };
    let v = gcd(b);
    // alpha takes the forward GCD's freed scratch; u and v are freed after it
    // so the target pass's scratch can use them.
    let mut alpha = b.alloc_n(n);
    b.x(u[0]);
    b.free_n(&u);
    b.free_n(&v);
    for i in 0..k {
        target_step(f, b, &mut alpha, &mut beta, transcript[2 * i], transcript[2 * i + 1]);
    }
    // beta is 0 now; halving may have moved it onto u's or x's qubits.
    b.free_n(&beta);

    for &q in u.iter().chain(&v) {
        b.realloc(q);
    }
    b.x(u[0]);
    // Regenerated rather than replayed, so its scratch avoids alpha.
    b.inverse_of(|b| {
        gcd(b);
    });
    b.free_n(&transcript);
    unload_const(b, u, f.p);
    alpha
}

#[cfg(test)]
pub fn gcd_step_for_cost(b: &mut Builder, u: &[Q], v: &mut Vec<Q>, odd: Q, swap: Q) {
    gcd_step(b, u, v, odd, swap, u.len())
}

#[cfg(test)]
pub fn target_step_for_cost(f: &Field, b: &mut Builder, a: &mut Vec<Q>, be: &mut Vec<Q>, odd: Q, swap: Q) {
    target_step(f, b, a, be, odd, swap)
}
