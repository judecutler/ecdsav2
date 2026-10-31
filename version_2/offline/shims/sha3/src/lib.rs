//! Minimal SHAKE256 (FIPS 202) with the `sha3`/`digest` trait names the
//! harness imports.

pub mod digest {
    pub trait Update {
        fn update(&mut self, data: &[u8]);
    }
    pub trait ExtendableOutput {
        type Reader: XofReader;
        fn finalize_xof(self) -> Self::Reader;
    }
    pub trait XofReader {
        fn read(&mut self, buf: &mut [u8]);
    }
}

use digest::{ExtendableOutput, Update, XofReader};

const RATE: usize = 136;

const RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];
const ROT: [u32; 25] = [0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14];

fn keccak_f(a: &mut [u64; 25]) {
    for rc in RC {
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        for x in 0..5 {
            let d = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                a[x + 5 * y] ^= d;
            }
        }
        let mut b = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                b[y + 5 * ((2 * x + 3 * y) % 5)] = a[x + 5 * y].rotate_left(ROT[x + 5 * y]);
            }
        }
        for x in 0..5 {
            for y in 0..5 {
                a[x + 5 * y] = b[x + 5 * y] ^ (!b[(x + 1) % 5 + 5 * y] & b[(x + 2) % 5 + 5 * y]);
            }
        }
        a[0] ^= rc;
    }
}

fn xor_byte(s: &mut [u64; 25], i: usize, v: u8) {
    s[i / 8] ^= (v as u64) << (8 * (i % 8));
}

#[derive(Clone, Default)]
pub struct Shake256 {
    state: [u64; 25],
    pos: usize,
}

impl Update for Shake256 {
    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            xor_byte(&mut self.state, self.pos, byte);
            self.pos += 1;
            if self.pos == RATE {
                keccak_f(&mut self.state);
                self.pos = 0;
            }
        }
    }
}

impl ExtendableOutput for Shake256 {
    type Reader = Shake256Reader;
    fn finalize_xof(mut self) -> Shake256Reader {
        xor_byte(&mut self.state, self.pos, 0x1f);
        xor_byte(&mut self.state, RATE - 1, 0x80);
        keccak_f(&mut self.state);
        Shake256Reader { state: self.state, pos: 0 }
    }
}

pub struct Shake256Reader {
    state: [u64; 25],
    pos: usize,
}

impl XofReader for Shake256Reader {
    fn read(&mut self, buf: &mut [u8]) {
        for out in buf {
            if self.pos == RATE {
                keccak_f(&mut self.state);
                self.pos = 0;
            }
            *out = (self.state[self.pos / 8] >> (8 * (self.pos % 8))) as u8;
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_input_vector() {
        let mut r = Shake256::default().finalize_xof();
        let mut out = [0u8; 32];
        r.read(&mut out);
        let hex: String = out.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f");
    }
}
