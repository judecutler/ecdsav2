use super::arith::{self, Field};
use super::divide::divide;
use super::gates::{lower, Builder, Q};
use super::point::point_add;
use crate::circuit::{analyze_ops, BitId, QubitOrBit};
use crate::sim::Simulator;
use alloy_primitives::U256;
use sha3::digest::{ExtendableOutput, Update};
use sha3::Shake256;

const SCRATCH: BitId = BitId(4000);

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, m: u64) -> u64 {
        self.next() % m
    }
}

fn qreg(q: &[Q]) -> Vec<QubitOrBit> {
    q.iter().map(|&q| QubitOrBit::Qubit(q)).collect()
}
fn breg(b: &[BitId]) -> Vec<QubitOrBit> {
    b.iter().map(|&b| QubitOrBit::Bit(b)).collect()
}

/// Runs the builder's gates on 64 shots. `inputs[k]` gives a register and one
/// value per shot. Returns per-shot values of `outputs` after asserting that
/// every other qubit is |0⟩ and the phase is clean.
fn run(b: &Builder, inputs: &[(Vec<QubitOrBit>, Vec<u64>)], outputs: &[Vec<QubitOrBit>]) -> Vec<Vec<u64>> {
    let mut ops = Vec::new();
    lower(&b.gates, SCRATCH, &mut ops);
    let (nq, nb, _, _) = analyze_ops(ops.iter());
    let nq = nq.max(b.width());
    let mut h = Shake256::default();
    h.update(b"v1-tests");
    let mut xof = h.finalize_xof();
    let mut sim = Simulator::new(nq as usize, (nb as usize).max(4001), &mut xof);
    for (reg, vals) in inputs {
        for (s, &v) in vals.iter().enumerate() {
            sim.set_register(reg, U256::from(v), s);
        }
    }
    sim.apply_iter(ops.iter());
    let shots = inputs[0].1.len();
    let out: Vec<Vec<u64>> = outputs
        .iter()
        .map(|r| (0..shots).map(|s| sim.get_register(r, s).to::<u64>()).collect())
        .collect();
    let mask = if shots == 64 { u64::MAX } else { (1u64 << shots) - 1 };
    assert_eq!(sim.phase & mask, 0, "phase garbage");
    for r in outputs {
        for m in r {
            if let QubitOrBit::Qubit(q) = m {
                *sim.qubit_mut(*q) = 0;
            }
        }
    }
    for q in 0..nq {
        assert_eq!(sim.qubits[q as usize] & mask, 0, "ancilla q{q} dirty");
    }
    out
}

const P8: u64 = 251;
fn f8() -> Field {
    Field { p: U256::from(P8), n: 8 }
}

fn samples(rng: &mut Rng, m: u64) -> Vec<u64> {
    (0..64).map(|_| rng.below(m)).collect()
}

#[test]
fn integer_add_sub_compare() {
    let mut rng = Rng(7);
    let mut b = Builder::new();
    let a = b.alloc_n(8);
    let t = b.alloc_n(8);
    let u = b.alloc_n(8);
    let z = b.alloc();
    let w = b.alloc();
    arith::add(&mut b, &arith::qs(&a), &t, Some(z));
    arith::sub(&mut b, &arith::qs(&a), &u, None);
    arith::less_than(&mut b, &a, &u, w, None);
    let (av, tv, uv) = (samples(&mut rng, 256), samples(&mut rng, 256), samples(&mut rng, 256));
    let out = run(
        &b,
        &[(qreg(&a), av.clone()), (qreg(&t), tv.clone()), (qreg(&u), uv.clone())],
        &[qreg(&a), qreg(&t), qreg(&[z]), qreg(&u), qreg(&[w])],
    );
    for s in 0..64 {
        let d = (uv[s] + 256 - av[s]) % 256;
        assert_eq!(out[0][s], av[s]);
        assert_eq!(out[1][s], (av[s] + tv[s]) % 256);
        assert_eq!(out[2][s], (av[s] + tv[s]) / 256);
        assert_eq!(out[3][s], d);
        assert_eq!(out[4][s], (d < av[s]) as u64);
    }
}

#[test]
fn field_ops() {
    let f = f8();
    let mut rng = Rng(11);
    let mut b = Builder::new();
    let a = b.alloc_n(8);
    let t = b.alloc_n(8);
    let mut h = b.alloc_n(8);
    let mut d = b.alloc_n(8);
    let s = b.alloc_n(8);
    let ctl = b.alloc();
    let (h_in, d_in) = (h.clone(), d.clone());
    f.add(&mut b, &a, &t);
    f.halve(&mut b, &mut h);
    f.double(&mut b, &mut d);
    f.ctl_sub(&mut b, ctl, &a, &s);
    let (av, tv, hv, dv, sv, cv) = (
        samples(&mut rng, P8),
        samples(&mut rng, P8),
        samples(&mut rng, P8),
        samples(&mut rng, P8),
        samples(&mut rng, P8),
        samples(&mut rng, 2),
    );
    let out = run(
        &b,
        &[
            (qreg(&a), av.clone()),
            (qreg(&t), tv.clone()),
            (qreg(&h_in), hv.clone()),
            (qreg(&d_in), dv.clone()),
            (qreg(&s), sv.clone()),
            (qreg(&[ctl]), cv.clone()),
        ],
        &[qreg(&a), qreg(&t), qreg(&h), qreg(&d), qreg(&s), qreg(&[ctl])],
    );
    let inv2 = (P8 + 1) / 2;
    for i in 0..64 {
        assert_eq!(out[1][i], (av[i] + tv[i]) % P8);
        assert_eq!(out[2][i], hv[i] * inv2 % P8);
        assert_eq!(out[3][i], dv[i] * 2 % P8);
        assert_eq!(out[4][i], (sv[i] + P8 - cv[i] * av[i]) % P8);
    }
}

#[test]
fn field_sub_square() {
    let f = f8();
    let mut rng = Rng(5);
    let mut b = Builder::new();
    let a = b.alloc_n(8);
    let t = b.alloc_n(8);
    f.sub_square(&mut b, &a, &t);
    let (av, tv) = (samples(&mut rng, P8), samples(&mut rng, P8));
    let out = run(&b, &[(qreg(&a), av.clone()), (qreg(&t), tv.clone())], &[qreg(&a), qreg(&t)]);
    for i in 0..64 {
        assert_eq!(out[0][i], av[i]);
        assert_eq!(out[1][i], (tv[i] + P8 * P8 - av[i] * av[i]) % P8);
    }
}

fn pow(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1;
    a %= p;
    while e > 0 {
        if e & 1 == 1 {
            r = r * a % p;
        }
        a = a * a % p;
        e >>= 1;
    }
    r
}

#[test]
fn divide_every_divisor() {
    let f = f8();
    let mut rng = Rng(3);
    for chunk in 0..4 {
        let mut b = Builder::new();
        let x = b.alloc_n(8);
        let mut a = b.alloc_n(8);
        let mut y = b.alloc_n(8);
        let (x_in, a_in, y_in) = (x.clone(), a.clone(), y.clone());
        divide(&f, &mut b, &x, &mut a, &mut y);
        let xv: Vec<u64> = (0..64).map(|i| 1 + ((chunk * 64 + i) % (P8 - 1))).collect();
        let yv = samples(&mut rng, P8);
        let out = run(
            &b,
            &[(qreg(&x_in), xv.clone()), (qreg(&a_in), vec![0; 64]), (qreg(&y_in), yv.clone())],
            &[qreg(&x), qreg(&a), qreg(&y)],
        );
        for i in 0..64 {
            assert_eq!(out[0][i], xv[i]);
            assert_eq!(out[1][i], yv[i] * pow(xv[i], P8 - 2, P8) % P8, "y/x for x={}", xv[i]);
            assert_eq!(out[2][i], 0);
        }
    }
}

/// Small curve y^2 = x^3 + 7 over F_251, exhaustively enumerated.
fn curve_points() -> Vec<(u64, u64)> {
    let mut pts = Vec::new();
    for x in 0..P8 {
        for y in 0..P8 {
            if (y * y) % P8 == (x * x % P8 * x + 7) % P8 {
                pts.push((x, y));
            }
        }
    }
    pts
}

fn affine_add(p: (u64, u64), q: (u64, u64)) -> (u64, u64) {
    let l = (p.1 + P8 - q.1) % P8 * pow((p.0 + P8 - q.0) % P8, P8 - 2, P8) % P8;
    let x3 = (l * l + 2 * P8 - p.0 - q.0) % P8;
    let y3 = (l * ((q.0 + P8 - x3) % P8) + P8 - q.1) % P8;
    (x3, y3)
}

#[test]
fn point_add_small_curve() {
    let f = f8();
    let pts = curve_points();
    let mut rng = Rng(9);
    let mut b = Builder::new();
    let x = b.alloc_n(8);
    let y = b.alloc_n(8);
    let cx: Vec<BitId> = (0..8).map(BitId).collect();
    let cy: Vec<BitId> = (8..16).map(BitId).collect();
    point_add(&f, &mut b, &x, &y, &cx, &cy);
    let mut tv = Vec::new();
    let mut ov = Vec::new();
    while tv.len() < 64 {
        let t = pts[rng.below(pts.len() as u64) as usize];
        let o = pts[rng.below(pts.len() as u64) as usize];
        if t.0 == o.0 || affine_add(t, o).0 == o.0 {
            continue;
        }
        tv.push(t);
        ov.push(o);
    }
    let out = run(
        &b,
        &[
            (qreg(&x), tv.iter().map(|p| p.0).collect()),
            (qreg(&y), tv.iter().map(|p| p.1).collect()),
            (breg(&cx), ov.iter().map(|p| p.0).collect()),
            (breg(&cy), ov.iter().map(|p| p.1).collect()),
        ],
        &[qreg(&x), qreg(&y)],
    );
    for i in 0..64 {
        assert_eq!((out[0][i], out[1][i]), affine_add(tv[i], ov[i]), "{:?} + {:?}", tv[i], ov[i]);
    }
}
