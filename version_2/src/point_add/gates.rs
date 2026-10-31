use crate::circuit::{BitId, Op, OperationType, QubitId};
use std::collections::BTreeSet;

pub type Q = QubitId;

/// Logical gate. `And` writes c1·c2 into a target known to be |0⟩ (one Toffoli).
/// `Unand` clears a target known to hold c1·c2 by X-basis measurement plus a
/// classically controlled CZ (no Toffoli). Reversing a gate list swaps the two,
/// so any block can be inverted by replaying it backwards.
#[derive(Clone, Copy, Debug)]
pub enum G {
    X(Q),
    Cx(Q, Q),
    Ccx(Q, Q, Q),
    Swap(Q, Q),
    And(Q, Q, Q),
    Unand(Q, Q, Q),
    XIfBit(Q, BitId),
}

impl G {
    fn inverse(self) -> G {
        match self {
            G::And(a, b, t) => G::Unand(a, b, t),
            G::Unand(a, b, t) => G::And(a, b, t),
            g => g,
        }
    }
}

pub struct Builder {
    pub gates: Vec<G>,
    free: BTreeSet<u64>,
    next: u64,
    live: u64,
    pub peak_live: u64,
}

impl Builder {
    pub fn new() -> Self {
        Builder { gates: Vec::new(), free: BTreeSet::new(), next: 0, live: 0, peak_live: 0 }
    }

    pub fn width(&self) -> u64 {
        self.next
    }

    pub fn alloc(&mut self) -> Q {
        self.live += 1;
        self.peak_live = self.peak_live.max(self.live);
        if let Some(q) = self.free.pop_first() {
            return QubitId(q);
        }
        self.next += 1;
        QubitId(self.next - 1)
    }

    /// Like `alloc_n`, but never returns any of `avoid`.
    pub fn alloc_n_avoiding(&mut self, n: usize, avoid: &[Q]) -> Vec<Q> {
        let avoid: BTreeSet<u64> = avoid.iter().map(|q| q.0).collect();
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let pick = self.free.iter().copied().find(|q| !avoid.contains(q));
            let q = match pick {
                Some(q) => {
                    self.free.remove(&q);
                    q
                }
                None => {
                    self.next += 1;
                    self.next - 1
                }
            };
            self.live += 1;
            self.peak_live = self.peak_live.max(self.live);
            out.push(QubitId(q));
        }
        out
    }

    pub fn alloc_n(&mut self, n: usize) -> Vec<Q> {
        (0..n).map(|_| self.alloc()).collect()
    }

    /// Caller guarantees the qubit is |0⟩ on every branch.
    pub fn free(&mut self, q: Q) {
        assert!(self.free.insert(q.0), "double free of q{}", q.0);
        self.live -= 1;
    }

    /// Marks a currently free qubit as live again (it must be |0⟩ or about to
    /// receive a value from a replayed block).
    pub fn realloc(&mut self, q: Q) {
        assert!(self.free.remove(&q.0), "q{} is not free", q.0);
        self.live += 1;
        self.peak_live = self.peak_live.max(self.live);
    }

    pub fn free_n(&mut self, qs: &[Q]) {
        for &q in qs {
            self.free(q);
        }
    }

    pub fn push(&mut self, g: G) {
        self.gates.push(g);
    }

    pub fn mark(&self) -> usize {
        self.gates.len()
    }

    /// Appends the inverse of gates[start..end]. The block must leave every
    /// qubit it temporarily used free again, so replaying it later is safe as
    /// long as those qubits are still free.
    pub fn replay_inverse(&mut self, start: usize, end: usize) {
        for i in (start..end).rev() {
            let g = self.gates[i].inverse();
            self.gates.push(g);
        }
    }

    /// Emits the inverse of whatever `f` emits.
    pub fn inverse_of(&mut self, f: impl FnOnce(&mut Builder)) {
        let start = self.mark();
        f(self);
        self.gates[start..].reverse();
        for g in &mut self.gates[start..] {
            *g = g.inverse();
        }
    }

    pub fn x(&mut self, q: Q) {
        self.push(G::X(q));
    }
    pub fn cx(&mut self, c: Q, t: Q) {
        self.push(G::Cx(c, t));
    }
    pub fn ccx(&mut self, a: Q, b: Q, t: Q) {
        self.push(G::Ccx(a, b, t));
    }
    pub fn and(&mut self, a: Q, b: Q, t: Q) {
        self.push(G::And(a, b, t));
    }
    pub fn unand(&mut self, a: Q, b: Q, t: Q) {
        self.push(G::Unand(a, b, t));
    }
    pub fn swap(&mut self, a: Q, b: Q) {
        self.push(G::Swap(a, b));
    }
    pub fn x_if_bit(&mut self, q: Q, b: BitId) {
        self.push(G::XIfBit(q, b));
    }
}

fn op(kind: OperationType) -> Op {
    let mut o = Op::empty();
    o.kind = kind;
    o
}

/// Lowers logical gates to harness ops. `scratch` is the classical bit that
/// receives X-basis measurement outcomes for `Unand`. With `coherent`, no
/// measurement is emitted: `Unand` becomes a Toffoli.
pub fn lower(gates: &[G], scratch: BitId, coherent: bool, out: &mut Vec<Op>) {
    out.reserve(gates.len() + gates.len() / 8);
    for &g in gates {
        match g {
            G::X(q) => {
                let mut o = op(OperationType::X);
                o.q_target = q;
                out.push(o);
            }
            G::Cx(c, t) => {
                let mut o = op(OperationType::CX);
                o.q_control1 = c;
                o.q_target = t;
                out.push(o);
            }
            G::Swap(a, b) => {
                let mut o = op(OperationType::Swap);
                o.q_control1 = a;
                o.q_target = b;
                out.push(o);
            }
            G::Ccx(a, b, t) | G::And(a, b, t) => {
                let mut o = op(OperationType::CCX);
                o.q_control2 = a;
                o.q_control1 = b;
                o.q_target = t;
                out.push(o);
            }
            G::Unand(a, b, t) if coherent => {
                let mut o = op(OperationType::CCX);
                o.q_control2 = a;
                o.q_control1 = b;
                o.q_target = t;
                out.push(o);
            }
            G::Unand(a, b, t) => {
                let mut m = op(OperationType::Hmr);
                m.q_target = t;
                m.c_target = scratch;
                out.push(m);
                let mut z = op(OperationType::CZ);
                z.q_control1 = a;
                z.q_target = b;
                z.c_condition = scratch;
                out.push(z);
            }
            G::XIfBit(q, b) => {
                let mut o = op(OperationType::X);
                o.q_target = q;
                o.c_condition = b;
                out.push(o);
            }
        }
    }
}
