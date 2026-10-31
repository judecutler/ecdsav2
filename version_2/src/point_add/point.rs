//! Affine point addition (x, y) += (cx, cy) with a classical addend.
//!
//! The addend bits are only ever copied into |0⟩ qubits with classically
//! conditioned X gates and copied out again. That is the interface a windowed
//! Shor lookup provides, so no Toffoli here depends on the addend being
//! classical, and no classical computation is done on it.
//!
//!   x -= cx; y -= cy                     (dx, dy)
//!   (a, y) = (0, dy) -> (dy/dx, 0)       lambda
//!   x += 3cx; x -= lambda^2              x = cx - x3
//!   (a, y) = (lambda, 0) -> (0, lambda·x) via the inverse division: y = y3 + cy
//!   y -= cy; x = cx - x
//!
//! Exceptional inputs: dx = 0 (P = ±Q) and x3 = cx (P = -2Q) are not handled.

use super::arith::Field;
use super::divide::divide;
use super::gates::{Builder, Q};
use crate::circuit::BitId;

fn load_bits(b: &mut Builder, bits: &[BitId]) -> Vec<Q> {
    let r = b.alloc_n(bits.len());
    for (&q, &c) in r.iter().zip(bits) {
        b.x_if_bit(q, c);
    }
    r
}

fn unload_bits(b: &mut Builder, r: Vec<Q>, bits: &[BitId]) {
    for (&q, &c) in r.iter().zip(bits) {
        b.x_if_bit(q, c);
    }
    b.free_n(&r);
}

/// Moves the value held on `cur` onto the qubits `target` (all other qubits
/// in `target` are |0⟩ and not live). Swaps only, no Toffolis.
fn route(b: &mut Builder, cur: &mut [Q], target: &[Q]) {
    for i in 0..cur.len() {
        if cur[i] == target[i] {
            continue;
        }
        let (from, to) = (cur[i], target[i]);
        b.swap(from, to);
        for c in cur.iter_mut().skip(i + 1) {
            if *c == to {
                *c = from;
            }
        }
        cur[i] = to;
    }
}

pub fn point_add(f: &Field, b: &mut Builder, x: &[Q], y: &[Q], cx: &[BitId], cy: &[BitId]) {

    let c = load_bits(b, cx);
    f.sub(b, &c, x);
    unload_bits(b, c, cx);
    let c = load_bits(b, cy);
    f.sub(b, &c, y);
    unload_bits(b, c, cy);

    let start = b.mark();
    let a = divide(f, b, x, y.to_vec());
    let end = b.mark();

    let c = load_bits(b, cx);
    for _ in 0..3 {
        f.add(b, &c, x);
    }
    unload_bits(b, c, cx);
    f.sub_square(b, &a, x);

    // Inverse division: same gates backwards, now with x = cx - x3 as divisor.
    // It takes a = lambda back to y = lambda·x and clears a.
    b.replay_inverse(start, end);
    b.free_n(&a);
    for &q in y {
        b.realloc(q);
    }

    let c = load_bits(b, cy);
    f.sub(b, &c, y);
    unload_bits(b, c, cy);

    // x3 = cx - x, computed into a fresh register, then x cleared.
    let mut l = load_bits(b, cx);
    f.sub(b, x, &l);
    f.add(b, &l, x);
    for (&q, &bit) in x.iter().zip(cx) {
        b.x_if_bit(q, bit);
    }
    let l_orig = l.clone();
    route(b, &mut l, x);
    b.free_n(&l_orig);
}
