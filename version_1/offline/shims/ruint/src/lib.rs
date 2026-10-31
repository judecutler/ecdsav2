//! Minimal stand-in for `ruint::aliases::U256`: only the operations the
//! harness and the point_add code use, implemented exactly.

pub mod aliases {
    pub use crate::U256;
}

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct U256([u64; 4]);

impl U256 {
    pub const ZERO: U256 = U256([0; 4]);
    pub const MAX: U256 = U256([u64::MAX; 4]);

    pub const fn from_limbs(l: [u64; 4]) -> Self {
        U256(l)
    }

    pub fn from_le_bytes(b: [u8; 32]) -> Self {
        let mut l = [0u64; 4];
        for (i, limb) in l.iter_mut().enumerate() {
            *limb = u64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap());
        }
        U256(l)
    }

    pub fn from_str_radix(s: &str, radix: u64) -> Result<Self, String> {
        let mut r = U256::ZERO;
        for ch in s.chars() {
            if ch == '_' {
                continue;
            }
            let d = ch.to_digit(radix as u32).ok_or_else(|| format!("bad digit {ch}"))? as u64;
            let (m, over) = r.mul_small(radix);
            let (a, c) = m.add_carry(U256::from(d));
            if over != 0 || c {
                return Err("overflow".into());
            }
            r = a;
        }
        Ok(r)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }

    pub fn bit(&self, i: usize) -> bool {
        i < 256 && (self.0[i / 64] >> (i % 64)) & 1 == 1
    }

    pub fn set_bit(&mut self, i: usize, v: bool) {
        let m = 1u64 << (i % 64);
        if v {
            self.0[i / 64] |= m;
        } else {
            self.0[i / 64] &= !m;
        }
    }

    pub fn to<T: FromU256>(&self) -> T {
        T::from_u256(*self)
    }

    pub fn as_limbs(&self) -> &[u64; 4] {
        &self.0
    }

    fn add_carry(self, o: U256) -> (U256, bool) {
        let mut r = [0u64; 4];
        let mut c = false;
        for i in 0..4 {
            let (s1, c1) = self.0[i].overflowing_add(o.0[i]);
            let (s2, c2) = s1.overflowing_add(c as u64);
            r[i] = s2;
            c = c1 || c2;
        }
        (U256(r), c)
    }

    fn sub_borrow(self, o: U256) -> (U256, bool) {
        let mut r = [0u64; 4];
        let mut b = false;
        for i in 0..4 {
            let (s1, b1) = self.0[i].overflowing_sub(o.0[i]);
            let (s2, b2) = s1.overflowing_sub(b as u64);
            r[i] = s2;
            b = b1 || b2;
        }
        (U256(r), b)
    }

    fn mul_small(self, k: u64) -> (U256, u64) {
        let mut r = [0u64; 4];
        let mut c = 0u128;
        for i in 0..4 {
            let t = self.0[i] as u128 * k as u128 + c;
            r[i] = t as u64;
            c = t >> 64;
        }
        (U256(r), c as u64)
    }

    fn shl1(self) -> (U256, bool) {
        let top = self.bit(255);
        let mut r = [0u64; 4];
        for i in 0..4 {
            r[i] = (self.0[i] << 1) | if i > 0 { self.0[i - 1] >> 63 } else { 0 };
        }
        (U256(r), top)
    }

    fn shr_n(self, n: usize) -> U256 {
        if n >= 256 {
            return U256::ZERO;
        }
        let (w, s) = (n / 64, n % 64);
        let mut r = [0u64; 4];
        for i in 0..4 - w {
            r[i] = self.0[i + w] >> s;
            if s > 0 && i + w + 1 < 4 {
                r[i] |= self.0[i + w + 1] << (64 - s);
            }
        }
        U256(r)
    }

    fn bit_len(&self) -> usize {
        for i in (0..4).rev() {
            if self.0[i] != 0 {
                return 64 * i + 64 - self.0[i].leading_zeros() as usize;
            }
        }
        0
    }

    fn rem(self, m: U256) -> U256 {
        assert!(!m.is_zero(), "division by zero");
        if self < m {
            return self;
        }
        let mut r = U256::ZERO;
        for i in (0..self.bit_len()).rev() {
            let (s, over) = r.shl1();
            r = s;
            r.set_bit(0, self.bit(i));
            if over || r >= m {
                r = r.sub_borrow(m).0;
            }
        }
        r
    }

    pub fn add_mod(self, o: U256, m: U256) -> U256 {
        let (a, b) = (self.rem(m), o.rem(m));
        let (s, c) = a.add_carry(b);
        if c || s >= m {
            s.sub_borrow(m).0
        } else {
            s
        }
    }

    pub fn mul_mod(self, o: U256, m: U256) -> U256 {
        let a = self.rem(m);
        let b = o.rem(m);
        let mut r = U256::ZERO;
        for i in (0..b.bit_len()).rev() {
            r = r.add_mod(r, m);
            if b.bit(i) {
                r = r.add_mod(a, m);
            }
        }
        r
    }

    /// Inverse modulo an odd m, if it exists.
    pub fn inv_mod(self, m: U256) -> Option<U256> {
        let a = self.rem(m);
        if a.is_zero() {
            return None;
        }
        let (mut u, mut v) = (a, m);
        let (mut x1, mut x2) = (U256::from(1u64), U256::ZERO);
        let half = |x: U256| -> U256 {
            if x.bit(0) {
                let (s, c) = x.add_carry(m);
                let mut h = s.shr_n(1);
                h.set_bit(255, c);
                h
            } else {
                x.shr_n(1)
            }
        };
        while u != U256::from(1u64) && v != U256::from(1u64) {
            if u.is_zero() || v.is_zero() {
                return None;
            }
            while !u.bit(0) {
                u = u.shr_n(1);
                x1 = half(x1);
            }
            while !v.bit(0) {
                v = v.shr_n(1);
                x2 = half(x2);
            }
            if u >= v {
                u = u.sub_borrow(v).0;
                x1 = sub_mod(x1, x2, m);
            } else {
                v = v.sub_borrow(u).0;
                x2 = sub_mod(x2, x1, m);
            }
        }
        Some(if u == U256::from(1u64) { x1 } else { x2 })
    }
}

fn sub_mod(a: U256, b: U256, m: U256) -> U256 {
    let (d, borrow) = a.sub_borrow(b);
    if borrow {
        d.add_carry(m).0
    } else {
        d
    }
}

pub trait FromU256 {
    fn from_u256(v: U256) -> Self;
}
impl FromU256 for u64 {
    fn from_u256(v: U256) -> Self {
        v.0[0]
    }
}
impl FromU256 for u128 {
    fn from_u256(v: U256) -> Self {
        v.0[0] as u128 | (v.0[1] as u128) << 64
    }
}

macro_rules! from_int {
    ($($t:ty),*) => {$(
        impl From<$t> for U256 {
            fn from(v: $t) -> Self {
                assert!(v as i128 >= 0, "negative U256");
                U256([v as u64, 0, 0, 0])
            }
        }
    )*};
}
from_int!(u8, u16, u32, u64, usize, i32, i64);

impl From<u128> for U256 {
    fn from(v: u128) -> Self {
        U256([v as u64, (v >> 64) as u64, 0, 0])
    }
}

impl Ord for U256 {
    fn cmp(&self, o: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&o.0[i]) {
                Ordering::Equal => continue,
                x => return x,
            }
        }
        Ordering::Equal
    }
}
impl PartialOrd for U256 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl std::ops::Rem for U256 {
    type Output = U256;
    fn rem(self, m: U256) -> U256 {
        U256::rem(self, m)
    }
}
impl std::ops::Sub for U256 {
    type Output = U256;
    fn sub(self, o: U256) -> U256 {
        let (d, b) = self.sub_borrow(o);
        assert!(!b, "U256 subtraction underflow");
        d
    }
}
impl std::ops::Add for U256 {
    type Output = U256;
    fn add(self, o: U256) -> U256 {
        let (s, c) = self.add_carry(o);
        assert!(!c, "U256 addition overflow");
        s
    }
}
impl std::ops::Shr<usize> for U256 {
    type Output = U256;
    fn shr(self, n: usize) -> U256 {
        self.shr_n(n)
    }
}
impl std::ops::Shr<i32> for U256 {
    type Output = U256;
    fn shr(self, n: i32) -> U256 {
        self.shr_n(n as usize)
    }
}
impl std::ops::ShrAssign<usize> for U256 {
    fn shr_assign(&mut self, n: usize) {
        *self = self.shr_n(n);
    }
}
impl std::ops::ShrAssign<i32> for U256 {
    fn shr_assign(&mut self, n: i32) {
        *self = self.shr_n(n as usize);
    }
}

impl fmt::LowerHex for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for i in (0..4).rev() {
            s.push_str(&format!("{:016x}", self.0[i]));
        }
        let t = s.trim_start_matches('0');
        f.pad_integral(true, "0x", if t.is_empty() { "0" } else { t })
    }
}
impl fmt::Debug for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self)
    }
}
impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self)
    }
}
