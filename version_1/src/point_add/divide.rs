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

use super::arith::{cswap, ctl_sub, less_than, load_const, unload_const, Field};
use super::gates::{Builder, Q};

pub fn steps(f: &Field) -> usize {
    2 * f.n
}

fn gcd_step(b: &mut Builder, u: &[Q], v: &mut Vec<Q>, odd: Q, swap: Q) {
    b.cx(v[0], odd);
    less_than(b, u, v, swap, Some(odd));
    cswap(b, swap, u, v);
    ctl_sub(b, odd, u, v);
    // v is even now; dividing by 2 is a relabel.
    v.rotate_left(1);
}

fn target_step(f: &Field, b: &mut Builder, alpha: &mut Vec<Q>, beta: &mut Vec<Q>, odd: Q, swap: Q) {
    cswap(b, swap, alpha, beta);
    f.ctl_sub(b, odd, alpha, beta);
    f.halve(b, beta);
}

/// Maps (alpha = 0, beta = y) to (alpha = y/x, beta = 0) with x unchanged.
/// Requires x != 0 mod p. Relabels alpha and beta.
pub fn divide(f: &Field, b: &mut Builder, x: &[Q], alpha: &mut Vec<Q>, beta: &mut Vec<Q>) {
    let n = f.n;
    let u = load_const(b, f.p, n);
    let v0 = b.alloc_n(n);
    for (&s, &d) in x.iter().zip(&v0) {
        b.cx(s, d);
    }
    let k = steps(f);
    let transcript = b.alloc_n(2 * k);
    let mut v = v0.clone();
    let start = b.mark();
    for i in 0..k {
        gcd_step(b, &u, &mut v, transcript[2 * i], transcript[2 * i + 1]);
    }
    let end = b.mark();
    for i in 0..k {
        target_step(f, b, alpha, beta, transcript[2 * i], transcript[2 * i + 1]);
    }
    b.replay_inverse(start, end);
    b.free_n(&transcript);
    for (&s, &d) in x.iter().zip(&v0) {
        b.cx(s, d);
    }
    b.free_n(&v0);
    unload_const(b, u, f.p);
}
