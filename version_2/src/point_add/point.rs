//! Affine point addition (x, y) += (cx, cy), with the addend either classical
//! bits (the v1 harness) or qubit registers (Track Q).
//!
//! The addend is only ever read. Classical bits are copied into |0⟩ qubits
//! with conditioned X gates and copied out again; qubit registers are used in
//! place and are unchanged at the end. So no Toffoli depends on the addend
//! being classical, and no classical computation is done on it.
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

/// One addend coordinate.
#[derive(Clone)]
pub enum Coord {
    Bits(Vec<BitId>),
    Qubits(Vec<Q>),
}

impl Coord {
    /// XORs the coordinate into `r`.
    fn xor_into(&self, b: &mut Builder, r: &[Q]) {
        match self {
            Coord::Bits(bits) => r.iter().zip(bits).for_each(|(&q, &c)| b.x_if_bit(q, c)),
            Coord::Qubits(qs) => r.iter().zip(qs).for_each(|(&q, &a)| b.cx(a, q)),
        }
    }

    /// A readable register holding the coordinate: the qubits themselves, or
    /// a fresh copy of the bits.
    fn load(&self, b: &mut Builder) -> Vec<Q> {
        match self {
            Coord::Qubits(qs) => qs.clone(),
            Coord::Bits(bits) => {
                let r = b.alloc_n(bits.len());
                self.xor_into(b, &r);
                r
            }
        }
    }

    fn unload(&self, b: &mut Builder, r: Vec<Q>) {
        if let Coord::Bits(_) = self {
            self.xor_into(b, &r);
            b.free_n(&r);
        }
    }

    /// A fresh register holding a copy of the coordinate.
    fn copy(&self, b: &mut Builder, n: usize) -> Vec<Q> {
        let r = b.alloc_n(n);
        self.xor_into(b, &r);
        r
    }
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

pub fn point_add(f: &Field, b: &mut Builder, x: &[Q], y: &[Q], cx: &Coord, cy: &Coord) {

    let c = cx.load(b);
    f.sub(b, &c, x);
    cx.unload(b, c);
    let c = cy.load(b);
    f.sub(b, &c, y);
    cy.unload(b, c);

    let start = b.mark();
    let a = divide(f, b, x, y.to_vec());
    let end = b.mark();

    let c = cx.load(b);
    for _ in 0..3 {
        f.add(b, &c, x);
    }
    cx.unload(b, c);
    f.sub_square(b, &a, x);

    // Inverse division: same gates backwards, now with x = cx - x3 as divisor.
    // It takes a = lambda back to y = lambda·x and clears a.
    b.replay_inverse(start, end);
    b.free_n(&a);
    for &q in y {
        b.realloc(q);
    }

    let c = cy.load(b);
    f.sub(b, &c, y);
    cy.unload(b, c);

    // x3 = cx - x, computed into a fresh register, then x cleared.
    let mut l = cx.copy(b, f.n);
    f.sub(b, x, &l);
    f.add(b, &l, x);
    cx.xor_into(b, x);
    let l_orig = l.clone();
    route(b, &mut l, x);
    b.free_n(&l_orig);
}
