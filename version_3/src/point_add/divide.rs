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
use super::transcript::{code_bits, pack, unpack, GROUP};
use crate::circuit::QubitId;

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
/// The GCD runs in place on x, which the target pass does not read. Qubits
/// are live only while they can hold data: each step's two transcript bits
/// are packed into a shared code as they are written (see `transcript.rs`) and
/// the codes are freed when the reverse GCD clears them, and
/// u/v bits above the proven width are released. At the end of the forward
/// GCD only u = 1 remains, so the target pass runs beside the transcript alone.
pub fn divide(f: &Field, b: &mut Builder, x: &[Q], beta: Vec<Q>) -> Vec<Q> {
    let mut beta = beta;
    let n = f.n;
    let k_max = steps(f);
    let u = load_const(b, f.p, n);
    // Label vector of v before each step: halving rotates the labels.
    let mut vs = vec![x.to_vec()];
    for _ in 0..k_max {
        let mut v = vs.last().unwrap().clone();
        v.rotate_left(1);
        vs.push(v);
    }
    // Ids of u and v that can be nonzero before step k (after the last step,
    // u = 1 and v = 0).
    let window = |k: usize| -> std::collections::BTreeSet<u64> {
        let (wu, wv) = if k < k_max { (width(f, k), width(f, k)) } else { (1, 0) };
        u[..wu].iter().chain(&vs[k][..wv]).map(|q| q.0).collect()
    };

    let groups: Vec<(usize, usize)> =
        (0..k_max).step_by(GROUP).map(|s| (s, GROUP.min(k_max - s))).collect();
    let mut codes = Vec::with_capacity(groups.len());
    for &(s, m) in &groups {
        let code = b.alloc_n(code_bits(m));
        for j in 0..m {
            let k = s + j;
            let (odd, swap) = (b.alloc(), b.alloc());
            let mut v = vs[k].clone();
            gcd_step(b, &u, &mut v, odd, swap, width(f, k));
            pack(b, &code, j, odd, swap);
            b.free(odd);
            b.free(swap);
            for id in window(k).difference(&window(k + 1)) {
                b.free(QubitId(*id));
            }
        }
        codes.push(code);
    }
    // alpha must not sit on u or x, which the reverse GCD rebuilds.
    let avoid: Vec<Q> = u.iter().chain(x).copied().collect();
    let mut alpha = b.alloc_n_avoiding(n, &avoid);
    b.x(u[0]);
    b.free(u[0]);
    for (&(_, m), code) in groups.iter().zip(&codes) {
        let bits = b.alloc_n(2 * m);
        for j in (0..m).rev() {
            unpack(b, code, j, bits[2 * j], bits[2 * j + 1]);
        }
        for j in 0..m {
            target_step(f, b, &mut alpha, &mut beta, bits[2 * j], bits[2 * j + 1]);
        }
        for j in 0..m {
            pack(b, code, j, bits[2 * j], bits[2 * j + 1]);
        }
        b.free_n(&bits);
    }
    // beta is 0 now; halving may have moved it onto u's or x's qubits.
    b.free_n(&beta);

    b.realloc(u[0]);
    b.x(u[0]);
    for (&(s, m), code) in groups.iter().zip(&codes).rev() {
        for j in (0..m).rev() {
            let k = s + j;
            for id in window(k).difference(&window(k + 1)) {
                b.realloc(QubitId(*id));
            }
            b.inverse_of(|b| {
                let (odd, swap) = (b.alloc(), b.alloc());
                let mut v = vs[k].clone();
                gcd_step(b, &u, &mut v, odd, swap, width(f, k));
                pack(b, code, j, odd, swap);
                b.free(odd);
                b.free(swap);
            });
        }
        b.free_n(code);
    }
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
