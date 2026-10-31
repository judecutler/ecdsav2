//! QCB ecdsa.fail v2, version_2: an exact, field-generic point adder.
//! See ../../PROGRESS.md for the design and the measured results.
//!
//! Build switches (the scored v1-harness circuit uses the defaults):
//!   V2_CURVE=p256        build for the NIST P-256 prime instead of secp256k1
//!   V2_ADDEND=quantum    Track Q interface: registers 2 and 3 are qubits
//!   V2_PROFILE=coherent  no measurements: AND uncomputation by Toffoli

mod arith;
mod divide;
mod gates;
mod point;
mod transcript;
#[cfg(test)]
mod tests;

use crate::circuit::{BitId, Op, OperationType, QubitOrBit, RegisterId};
use alloy_primitives::U256;
use arith::Field;
use gates::{lower, Builder, G};
use point::Coord;

pub const SECP256K1_P: &str = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F";
pub const P256_P: &str = "FFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF";

/// The prime the circuit is built for. The scored submission always uses
/// secp256k1; `V2_CURVE=p256` exists only to test that nothing is specific to it.
fn field() -> Field {
    let hex = match std::env::var("V2_CURVE").as_deref() {
        Ok("p256") => P256_P,
        _ => SECP256K1_P,
    };
    Field { p: U256::from_str_radix(hex, 16).unwrap(), n: 256 }
}

fn declare(ops: &mut Vec<Op>, r: u64, members: impl Iterator<Item = QubitOrBit>) {
    for m in members {
        let mut o = Op::empty();
        o.kind = OperationType::AppendToRegister;
        match m {
            QubitOrBit::Qubit(q) => o.q_target = q,
            QubitOrBit::Bit(c) => o.c_target = c,
        }
        o.r_target = RegisterId(r);
        ops.push(o);
    }
    let mut o = Op::empty();
    o.kind = OperationType::Register;
    o.r_target = RegisterId(r);
    ops.push(o);
}

fn flag(name: &str, on: &str) -> bool {
    std::env::var(name).map(|v| v == on).unwrap_or(false)
}

/// Builds the circuit for `f`.
pub fn build_for(f: &Field, quantum_addend: bool, coherent: bool) -> Vec<Op> {
    let n = f.n;
    let mut b = Builder::new();
    let x = b.alloc_n(n);
    let y = b.alloc_n(n);
    let (cx, cy, offsets): (Coord, Coord, [Vec<QubitOrBit>; 2]) = if quantum_addend {
        let (ax, ay) = (b.alloc_n(n), b.alloc_n(n));
        let regs = [ax.iter().map(|&q| QubitOrBit::Qubit(q)).collect(), ay.iter().map(|&q| QubitOrBit::Qubit(q)).collect()];
        (Coord::Qubits(ax), Coord::Qubits(ay), regs)
    } else {
        let ax: Vec<BitId> = (0..n as u64).map(BitId).collect();
        let ay: Vec<BitId> = (n as u64..2 * n as u64).map(BitId).collect();
        let regs = [ax.iter().map(|&c| QubitOrBit::Bit(c)).collect(), ay.iter().map(|&c| QubitOrBit::Bit(c)).collect()];
        (Coord::Bits(ax), Coord::Bits(ay), regs)
    };
    let scratch = BitId(2 * n as u64);

    point::point_add(f, &mut b, &x, &y, &cx, &cy);

    let mut ops = Vec::new();
    declare(&mut ops, 0, x.iter().map(|&q| QubitOrBit::Qubit(q)));
    declare(&mut ops, 1, y.iter().map(|&q| QubitOrBit::Qubit(q)));
    let [ox, oy] = offsets;
    declare(&mut ops, 2, ox.into_iter());
    declare(&mut ops, 3, oy.into_iter());
    lower(&b.gates, scratch, coherent, &mut ops);
    let count = |pred: fn(&G) -> bool| b.gates.iter().filter(|g| pred(g)).count();
    let toffolis = count(|g| matches!(g, G::And(..) | G::Ccx(..)));
    let unands = count(|g| matches!(g, G::Unand(..)));
    eprintln!(
        "  builder     : width {} qubits, {} Toffolis ({} with coherent AND uncomputation){}",
        b.width(),
        toffolis,
        toffolis + unands,
        if coherent { ", coherent lowering" } else { "" }
    );
    ops
}

pub fn build() -> Vec<Op> {
    build_for(&field(), flag("V2_ADDEND", "quantum"), flag("V2_PROFILE", "coherent"))
}
