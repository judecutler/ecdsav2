//! Packed GCD transcript. A step's choice (odd, swap) has only three values,
//! because swap implies odd, so it is the trit t = odd + swap. Five steps share
//! one 8-bit code C = sum of t_j 3^j (3^5 = 243 <= 256), which stores 1.6
//! bits per step instead of 2.
//!
//! `pack` folds step j's bits into C and clears them. It relies on the digits
//! above j still being 0, so that t_j >= 1 iff C >= 3^j and t_j = 2 iff
//! C >= 2·3^j. Its inverse `unpack` restores the bits and removes the digit.

use super::arith::{add, carry_out, konst};
use super::gates::{Builder, Q};
use alloy_primitives::U256;

pub const GROUP: usize = 5;

/// Bits needed for a group of m steps.
pub fn code_bits(m: usize) -> usize {
    let states = 3u64.pow(m as u32);
    (64 - (states - 1).leading_zeros() as usize).max(1)
}

fn ge_const(b: &mut Builder, code: &[Q], k: u64, z: Q) {
    let w = code.len();
    carry_out(b, &konst(U256::from((1u64 << w) - k), w, None), code, z, None);
}

/// C += 3^j·(odd + swap), then odd and swap are cleared from C.
pub fn pack(b: &mut Builder, code: &[Q], j: usize, odd: Q, swap: Q) {
    let w = code.len();
    let d = 3u64.pow(j as u32);
    add(b, &konst(U256::from(d), w, Some(odd)), code, None);
    add(b, &konst(U256::from(d), w, Some(swap)), code, None);
    ge_const(b, code, d, odd);
    ge_const(b, code, 2 * d, swap);
}

pub fn unpack(b: &mut Builder, code: &[Q], j: usize, odd: Q, swap: Q) {
    b.inverse_of(|b| pack(b, code, j, odd, swap));
}
