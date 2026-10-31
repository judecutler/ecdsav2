//! v2-proxy evaluator for QCB's ecdsa.fail v2 work.
//!
//! Differences from the v1 `eval_circuit`:
//! - Test inputs come from a fixed seed (`--seed`), not from a hash of the op
//!   stream, so a circuit cannot pick its own test set by editing its gates.
//! - Larger sample (`--shots`, default 20,000) plus edge-case classes, each
//!   reported separately with its failure count.
//! - Reports worst-case Toffolis (every CCX/CCZ op, executed or not) next to
//!   the v1 average, and how many ops are conditioned on classical bits.
//! - Supports other curves (`--curve p256`) to check that a circuit is not
//!   specialized to secp256k1.
//!
//! Usage: eval_v2 [--ops ops.bin] [--curve secp256k1|p256] [--shots N] [--seed S]
//!                [--threads T] [--json out.json]

use alloy_primitives::U256;
use quantum_ecc::circuit::{analyze_ops, BitId, Op, OperationType, QubitId, QubitOrBit, RegisterId, NO_BIT};
use quantum_ecc::sim::Simulator;
use quantum_ecc::weierstrass_elliptic_curve::WeierstrassEllipticCurve;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake256;
use std::io::Read;

const MAGIC: &[u8; 8] = b"QECCOPSZ";
const OP_BYTES: usize = 56;

fn kind(v: u32) -> OperationType {
    use OperationType::*;
    [Neg, Register, AppendToRegister, BitInvert, BitStore0, BitStore1, X, Z, CX, CZ, Swap, R, Hmr, CCX, CCZ, PushCondition, PopCondition, DebugPrint]
        [v as usize]
}

fn load_ops(path: &str) -> Vec<Op> {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let mut r = std::io::BufReader::new(f);
    let mut header = [0u8; 16];
    r.read_exact(&mut header).unwrap();
    assert_eq!(&header[..8], MAGIC, "bad magic");
    let n = u64::from_le_bytes(header[8..].try_into().unwrap()) as usize;
    let mut dec = zstd::stream::read::Decoder::new(r).unwrap();
    let mut ops = Vec::with_capacity(n);
    let mut rec = [0u8; OP_BYTES];
    let u = |rec: &[u8; OP_BYTES], o: usize| u64::from_le_bytes(rec[o..o + 8].try_into().unwrap());
    for _ in 0..n {
        dec.read_exact(&mut rec).unwrap();
        let op = Op {
            kind: kind(u32::from_le_bytes(rec[0..4].try_into().unwrap())),
            q_control2: QubitId(u(&rec, 8)),
            q_control1: QubitId(u(&rec, 16)),
            q_target: QubitId(u(&rec, 24)),
            c_target: BitId(u(&rec, 32)),
            c_condition: BitId(u(&rec, 40)),
            r_target: RegisterId(u(&rec, 48)),
        };
        op.validate();
        ops.push(op);
    }
    ops
}

fn hex(s: &str) -> U256 {
    U256::from_str_radix(s, 16).unwrap()
}

fn curve(name: &str) -> WeierstrassEllipticCurve {
    match name {
        "secp256k1" => WeierstrassEllipticCurve {
            modulus: hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F"),
            a: U256::from(0u64),
            b: U256::from(7u64),
            gx: hex("79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798"),
            gy: hex("483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8"),
            order: hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141"),
        },
        "p256" => WeierstrassEllipticCurve {
            modulus: hex("FFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF"),
            a: hex("FFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFC"),
            b: hex("5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B"),
            gx: hex("6B17D1F2E12C4247F8BCE6E563A440F277037D812DEB33A0F4A13945D898C296"),
            gy: hex("4FE342E2FE1A7F9B8EE7EB4A7C0F9E162BCE33576B315ECECBB6406837BF51F5"),
            order: hex("FFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551"),
        },
        other => panic!("unknown curve {other}"),
    }
}

type Pt = (U256, U256);

struct Case {
    t: Pt,
    o: Pt,
    want: Pt,
    /// T = -2·O, so the sum is -O and x3 equals the addend's x.
    t_is_minus_2o: bool,
}

fn neg(c: &WeierstrassEllipticCurve, p: Pt) -> Pt {
    (p.0, if p.1.is_zero() { p.1 } else { c.modulus - p.1 })
}

/// Pairs (T, O) the harness contract covers: neither is the identity and T.x != O.x.
fn case(c: &WeierstrassEllipticCurve, t: Pt, o: Pt) -> Option<Case> {
    let id = |p: Pt| p.0.is_zero() && p.1.is_zero();
    if id(t) || id(o) || t.0 == o.0 {
        return None;
    }
    let want = c.add(t.0, t.1, o.0, o.1);
    Some(Case { t, o, want, t_is_minus_2o: want == neg(c, o) })
}

fn scalar(xof: &mut sha3::Shake256Reader) -> U256 {
    let mut b = [0u8; 32];
    xof.read(&mut b);
    U256::from_le_bytes(b)
}

fn mul(c: &WeierstrassEllipticCurve, k: U256) -> Pt {
    c.mul(c.gx, c.gy, k)
}

#[derive(Default, Clone, Copy)]
struct Tally {
    shots: usize,
    lanes: usize,
    wrong: usize,
    wrong_minus_2o: usize,
    phase_batches: usize,
    ancilla_batches: usize,
    toffoli: u64,
}

fn run(ops: &[Op], regs: &[Vec<QubitOrBit>], nq: u64, nb: u64, cases: &[Case], seed: &[u8]) -> Tally {
    let mut h = Shake256::default();
    h.update(b"qcb-v2-proxy-sim");
    h.update(seed);
    let mut xof = h.finalize_xof();
    let mut sim = Simulator::new(nq as usize, nb as usize, &mut xof);
    let mut t = Tally::default();
    for chunk in cases.chunks(64) {
        let bs = chunk.len();
        let mask = if bs == 64 { u64::MAX } else { (1u64 << bs) - 1 };
        sim.clear_for_shot();
        // Idle lanes rerun the first case so the Toffoli tally stays per shot.
        for s in 0..64 {
            let c = &chunk[if s < bs { s } else { 0 }];
            sim.set_register(&regs[0], c.t.0, s);
            sim.set_register(&regs[1], c.t.1, s);
            sim.set_register(&regs[2], c.o.0, s);
            sim.set_register(&regs[3], c.o.1, s);
        }
        let before = sim.stats.toffoli_gates;
        sim.apply_iter(ops.iter());
        t.toffoli += sim.stats.toffoli_gates - before;
        for (s, c) in chunk.iter().enumerate() {
            let got = (sim.get_register(&regs[0], s), sim.get_register(&regs[1], s));
            if got != c.want {
                t.wrong += 1;
                if c.t_is_minus_2o {
                    t.wrong_minus_2o += 1;
                }
            }
        }
        if sim.phase & mask != 0 {
            t.phase_batches += 1;
        }
        for r in &regs[..2] {
            for m in r {
                if let QubitOrBit::Qubit(q) = *m {
                    *sim.qubit_mut(q) = 0;
                }
            }
        }
        if (0..nq).any(|q| sim.qubit(QubitId(q)) & mask != 0) {
            t.ancilla_batches += 1;
        }
        t.shots += bs;
        t.lanes += 64;
    }
    t
}

fn run_parallel(ops: &[Op], regs: &[Vec<QubitOrBit>], nq: u64, nb: u64, cases: &[Case], threads: usize, seed: &[u8]) -> Tally {
    let per = cases.len().div_ceil(threads).div_ceil(64) * 64;
    let parts: Vec<Tally> = std::thread::scope(|s| {
        let hs: Vec<_> = cases
            .chunks(per.max(64))
            .enumerate()
            .map(|(i, ch)| {
                let mut sd = seed.to_vec();
                sd.push(i as u8);
                s.spawn(move || run(ops, regs, nq, nb, ch, &sd))
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    parts.iter().fold(Tally::default(), |a, b| Tally {
        shots: a.shots + b.shots,
        lanes: a.lanes + b.lanes,
        wrong: a.wrong + b.wrong,
        wrong_minus_2o: a.wrong_minus_2o + b.wrong_minus_2o,
        phase_batches: a.phase_batches + b.phase_batches,
        ancilla_batches: a.ancilla_batches + b.ancilla_batches,
        toffoli: a.toffoli + b.toffoli,
    })
}

struct Census {
    toffoli_ops: u64,
    conditioned_toffoli_ops: u64,
    conditioned_ops: u64,
    bit_logic_ops: u64,
    offset_conditioned_ops: u64,
}

/// Static facts about the op stream. Bit-logic ops compute on classical bits
/// for free; ops conditioned on offset bits use the classical addend directly.
fn census(ops: &[Op], regs: &[Vec<QubitOrBit>]) -> Census {
    let offset_bits: std::collections::HashSet<u64> = regs[2..4]
        .iter()
        .flatten()
        .filter_map(|m| if let QubitOrBit::Bit(b) = m { Some(b.0) } else { None })
        .collect();
    let mut c = Census { toffoli_ops: 0, conditioned_toffoli_ops: 0, conditioned_ops: 0, bit_logic_ops: 0, offset_conditioned_ops: 0 };
    let mut depth = 0i64;
    for op in ops {
        use OperationType::*;
        let conditioned = op.c_condition != NO_BIT || depth > 0;
        match op.kind {
            CCX | CCZ => {
                c.toffoli_ops += 1;
                if conditioned {
                    c.conditioned_toffoli_ops += 1;
                }
            }
            BitInvert | BitStore0 | BitStore1 => c.bit_logic_ops += 1,
            PushCondition => depth += 1,
            PopCondition => depth -= 1,
            _ => {}
        }
        if conditioned && !matches!(op.kind, PushCondition | PopCondition) {
            c.conditioned_ops += 1;
        }
        if op.c_condition != NO_BIT && offset_bits.contains(&op.c_condition.0) {
            c.offset_conditioned_ops += 1;
        }
    }
    c
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| -> String {
        args.iter().position(|a| a == k).map(|i| args[i + 1].clone()).unwrap_or_else(|| d.to_string())
    };
    let ops_path = arg("--ops", "ops.bin");
    let curve_name = arg("--curve", "secp256k1");
    let shots: usize = arg("--shots", "20000").parse().unwrap();
    let seed = arg("--seed", "qcb-v2-proxy-2026-10-31");
    let threads: usize = arg("--threads", "2").parse().unwrap();
    let json_path = arg("--json", "");

    let c = curve(&curve_name);
    assert!(c.is_on_curve(c.gx, c.gy), "generator not on curve");
    let ops = load_ops(&ops_path);
    let (nq, nb, _, regs) = analyze_ops(ops.iter());
    assert_eq!(regs.len(), 4, "expected 4 registers");
    let cen = census(&ops, &regs);

    // Case classes.
    let mut h = Shake256::default();
    h.update(b"qcb-v2-proxy-inputs");
    h.update(seed.as_bytes());
    h.update(curve_name.as_bytes());
    let mut xof = h.finalize_xof();
    let mut random = Vec::with_capacity(shots);
    while random.len() < shots {
        let (t, o) = (mul(&c, scalar(&mut xof)), mul(&c, scalar(&mut xof)));
        if let Some(k) = case(&c, t, o) {
            random.push(k);
        }
    }
    let g = (c.gx, c.gy);
    let mut small = Vec::new();
    let multiples: Vec<Pt> = (1..=12u64).map(|k| mul(&c, U256::from(k))).collect();
    let negs: Vec<Pt> = multiples.iter().map(|&p| neg(&c, p)).collect();
    for a in multiples.iter().chain(&negs) {
        for b in multiples.iter().chain(&negs) {
            if let Some(k) = case(&c, *a, *b) {
                small.push(k);
            }
        }
    }
    let mut near_order = Vec::new();
    for d in 1..=32u64 {
        let k = c.order - U256::from(d);
        let t = mul(&c, k);
        for o in [g, multiples[1], multiples[6]] {
            if let Some(cs) = case(&c, t, o) {
                near_order.push(cs);
            }
        }
    }
    // T = -2·O: the sum is -O, so x3 equals the addend's x.
    let mut exceptional = Vec::new();
    for _ in 0..64 {
        let o = mul(&c, scalar(&mut xof));
        let t = neg(&c, c.add(o.0, o.1, o.0, o.1));
        if let Some(k) = case(&c, t, o) {
            exceptional.push(k);
        }
    }

    println!("=== eval_v2 (QCB v2 proxy) ===");
    println!("curve            : {curve_name}");
    println!("qubits (width)   : {nq}");
    println!("ops              : {}", ops.len());
    println!("Toffoli ops      : {} (worst case per shot)", cen.toffoli_ops);
    println!("  conditioned    : {}", cen.conditioned_toffoli_ops);
    println!("conditioned ops  : {} ({} on offset bits)", cen.conditioned_ops, cen.offset_conditioned_ops);
    println!("bit-logic ops    : {}", cen.bit_logic_ops);

    let classes: [(&str, &Vec<Case>); 4] =
        [("random", &random), ("small-multiples", &small), ("near-order", &near_order), ("exceptional T=-2O", &exceptional)];
    let mut rows = Vec::new();
    let mut random_tally = Tally::default();
    for (name, cases) in classes {
        let t = run_parallel(&ops, &regs, nq, nb, cases, threads, seed.as_bytes());
        println!(
            "{name:<18}: {:>6} shots, {:>6} wrong ({} with T = -2O), {} phase-dirty batches, {} ancilla-dirty batches, avg Toffoli {:.1}",
            t.shots,
            t.wrong,
            t.wrong_minus_2o,
            t.phase_batches,
            t.ancilla_batches,
            t.toffoli as f64 / t.lanes.max(1) as f64
        );
        if name == "random" {
            random_tally = t;
        }
        rows.push(format!(
            "{{\"class\":\"{name}\",\"shots\":{},\"wrong\":{},\"wrong_t_minus_2o\":{},\"phase_dirty_batches\":{},\"ancilla_dirty_batches\":{}}}",
            t.shots, t.wrong, t.wrong_minus_2o, t.phase_batches, t.ancilla_batches
        ));
    }
    let avg = random_tally.toffoli as f64 / random_tally.lanes as f64;
    let clean = random_tally.phase_batches == 0 && random_tally.ancilla_batches == 0;
    let p_hat = if clean { 1.0 - random_tally.wrong as f64 / random_tally.shots as f64 } else { 0.0 };
    let qxt_avg = nq as f64 * avg;
    let qxt_worst = nq as f64 * cen.toffoli_ops as f64;
    println!("Q x T (v1 avg)   : {:.4e}", qxt_avg);
    println!("Q x T (worst)    : {:.4e}", qxt_worst);
    println!("p-hat (random)   : {:.6}", p_hat);
    println!("Q x T_worst / p  : {:.4e}", if p_hat > 0.0 { qxt_worst / p_hat } else { f64::INFINITY });
    if !json_path.is_empty() {
        let body = format!(
            "{{\"curve\":\"{curve_name}\",\"seed\":\"{seed}\",\"qubits\":{nq},\"ops\":{},\"toffoli_worst\":{},\"toffoli_avg\":{avg:.3},\"conditioned_toffoli_ops\":{},\"offset_conditioned_ops\":{},\"bit_logic_ops\":{},\"p_hat\":{p_hat},\"qxt_avg\":{qxt_avg:.0},\"qxt_worst\":{qxt_worst:.0},\"classes\":[{}]}}\n",
            ops.len(),
            cen.toffoli_ops,
            cen.conditioned_toffoli_ops,
            cen.offset_conditioned_ops,
            cen.bit_logic_ops,
            rows.join(",")
        );
        std::fs::write(&json_path, body).unwrap();
    }
}
