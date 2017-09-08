// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The ISAAC random number generator.

#![allow(non_camel_case_types)]

use core::slice;
use core::iter::repeat;
use core::num::Wrapping as w;
use core::fmt;

use {Rng, FromRng, SeedableRng, CryptoError};

/// Select 32- or 64-bit variant dependent on pointer size.
#[cfg(target_pointer_width = "32")]
pub use prng::IsaacRng as IsaacWordRng;
#[cfg(target_pointer_width = "64")]
pub use prng::Isaac64Rng as IsaacWordRng;

#[allow(bad_style)]
type w64 = w<u64>;
#[allow(bad_style)]
type w32 = w<u32>;

const RAND_SIZE_LEN: usize = 8;
const RAND_SIZE: u32 = 1 << RAND_SIZE_LEN;
const RAND_SIZE_USIZE: usize = 1 << RAND_SIZE_LEN;

/// A random number generator that uses the ISAAC algorithm[1].
///
/// The ISAAC algorithm is generally accepted as suitable for
/// cryptographic purposes, but this implementation has not be
/// verified as such. Prefer a generator like `OsRng` that defers to
/// the operating system for cases that need high security.
///
/// [1]: Bob Jenkins, [*ISAAC: A fast cryptographic random number
/// generator*](http://www.burtleburtle.net/bob/rand/isaacafa.html)
#[derive(Copy)]
pub struct IsaacRng {
    cnt: u32,
    rsl: [w32; RAND_SIZE_USIZE],
    mem: [w32; RAND_SIZE_USIZE],
    a: w32,
    b: w32,
    c: w32,
}

static EMPTY: IsaacRng = IsaacRng {
    cnt: 0,
    rsl: [w(0); RAND_SIZE_USIZE],
    mem: [w(0); RAND_SIZE_USIZE],
    a: w(0), b: w(0), c: w(0),
};

impl IsaacRng {
    /// Create an ISAAC random number generator using the default
    /// fixed seed.
    pub fn new_unseeded() -> IsaacRng {
        let mut rng = EMPTY;
        rng.init(false);
        rng
    }

    /// Initialises `self`. If `use_rsl` is true, then use the current value
    /// of `rsl` as a seed, otherwise construct one algorithmically (not
    /// randomly).
    fn init(&mut self, use_rsl: bool) {
        let mut a = w(0x9e3779b9);
        let mut b = a;
        let mut c = a;
        let mut d = a;
        let mut e = a;
        let mut f = a;
        let mut g = a;
        let mut h = a;

        macro_rules! mix {
            () => {{
                a=a^(b<<11); d=d+a; b=b+c;
                b=b^(c>>2);  e=e+b; c=c+d;
                c=c^(d<<8);  f=f+c; d=d+e;
                d=d^(e>>16); g=g+d; e=e+f;
                e=e^(f<<10); h=h+e; f=f+g;
                f=f^(g>>4);  a=a+f; g=g+h;
                g=g^(h<<8);  b=b+g; h=h+a;
                h=h^(a>>9);  c=c+h; a=a+b;
            }}
        }

        for _ in 0..4 {
            mix!();
        }

        if use_rsl {
            macro_rules! memloop {
                ($arr:expr) => {{
                    for i in (0..RAND_SIZE_USIZE/8).map(|i| i * 8) {
                        a=a+$arr[i  ]; b=b+$arr[i+1];
                        c=c+$arr[i+2]; d=d+$arr[i+3];
                        e=e+$arr[i+4]; f=f+$arr[i+5];
                        g=g+$arr[i+6]; h=h+$arr[i+7];
                        mix!();
                        self.mem[i  ]=a; self.mem[i+1]=b;
                        self.mem[i+2]=c; self.mem[i+3]=d;
                        self.mem[i+4]=e; self.mem[i+5]=f;
                        self.mem[i+6]=g; self.mem[i+7]=h;
                    }
                }}
            }

            memloop!(self.rsl);
            memloop!(self.mem);
        } else {
            for i in (0..RAND_SIZE_USIZE/8).map(|i| i * 8) {
                mix!();
                self.mem[i  ]=a; self.mem[i+1]=b;
                self.mem[i+2]=c; self.mem[i+3]=d;
                self.mem[i+4]=e; self.mem[i+5]=f;
                self.mem[i+6]=g; self.mem[i+7]=h;
            }
        }

        self.isaac();
    }

    /// Refills the output buffer (`self.rsl`)
    #[inline]
    fn isaac(&mut self) {
        self.c = self.c + w(1);
        // abbreviations
        let mut a = self.a;
        let mut b = self.b + self.c;

        const MIDPOINT: usize = RAND_SIZE_USIZE / 2;

        macro_rules! ind {
            ($x:expr) => ( self.mem[($x >> 2usize).0 as usize & (RAND_SIZE_USIZE - 1)] )
        }

        let r = [(0, MIDPOINT), (MIDPOINT, 0)];
        for &(mr_offset, m2_offset) in r.iter() {

            macro_rules! rngstepp {
                ($j:expr, $shift:expr) => {{
                    let base = $j;
                    let mix = a << $shift;

                    let x = self.mem[base  + mr_offset];
                    a = (a ^ mix) + self.mem[base + m2_offset];
                    let y = ind!(x) + a + b;
                    self.mem[base + mr_offset] = y;

                    b = ind!(y >> RAND_SIZE_LEN) + x;
                    self.rsl[base + mr_offset] = b;
                }}
            }

            macro_rules! rngstepn {
                ($j:expr, $shift:expr) => {{
                    let base = $j;
                    let mix = a >> $shift;

                    let x = self.mem[base  + mr_offset];
                    a = (a ^ mix) + self.mem[base + m2_offset];
                    let y = ind!(x) + a + b;
                    self.mem[base + mr_offset] = y;

                    b = ind!(y >> RAND_SIZE_LEN) + x;
                    self.rsl[base + mr_offset] = b;
                }}
            }

            for i in (0..MIDPOINT/4).map(|i| i * 4) {
                rngstepp!(i + 0, 13);
                rngstepn!(i + 1, 6);
                rngstepp!(i + 2, 2);
                rngstepn!(i + 3, 16);
            }
        }

        self.a = a;
        self.b = b;
        self.cnt = RAND_SIZE;
    }
}

// Cannot be derived because [u32; 256] does not implement Clone
impl Clone for IsaacRng {
    fn clone(&self) -> IsaacRng {
        *self
    }
}

impl Rng for IsaacRng {
    #[inline]
    fn next_u32(&mut self) -> Result<u32, CryptoError> {
        if self.cnt == 0 {
            // make some more numbers
            self.isaac();
        }
        self.cnt -= 1;

        // self.cnt is at most RAND_SIZE, but that is before the
        // subtraction above. We want to index without bounds
        // checking, but this could lead to incorrect code if someone
        // misrefactors, so we check, sometimes.
        //
        // (Changes here should be reflected in Isaac64Rng.next_u64.)
        debug_assert!(self.cnt < RAND_SIZE);

        // (the % is cheaply telling the optimiser that we're always
        // in bounds, without unsafe. NB. this is a power of two, so
        // it optimises to a bitwise mask).
        Ok(self.rsl[(self.cnt % RAND_SIZE) as usize].0)
    }
    
    // Default impl adjusted for native byte size; approx 18% faster in tests
    fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), CryptoError> {
        use core::intrinsics::transmute;
        
        let mut left = dest;
        while left.len() >= 4 {
            let (l, r) = {left}.split_at_mut(4);
            left = r;
            let chunk: [u8; 4] = unsafe {
                transmute(self.next_u32()?.to_le())
            };
            l.copy_from_slice(&chunk);
        }
        let n = left.len();
        if n > 0 {
            let chunk: [u8; 4] = unsafe {
                transmute(self.next_u32()?.to_le())
            };
            left.copy_from_slice(&chunk[..n]);
        }
        Ok(())
    }
}

impl FromRng for IsaacRng {
    fn from_rng<R: Rng+?Sized>(other: &mut R) -> Result<IsaacRng, CryptoError> {
        let mut ret = EMPTY;
        unsafe {
            let ptr = ret.rsl.as_mut_ptr() as *mut u8;

            let slice = slice::from_raw_parts_mut(ptr, RAND_SIZE_USIZE * 4);
            other.fill_bytes(slice)?;
        }
        ret.cnt = 0;
        ret.a = w(0);
        ret.b = w(0);
        ret.c = w(0);

        ret.init(true);
        Ok(ret)
    }
}

impl<'a> SeedableRng<&'a [u32]> for IsaacRng {
    fn reseed(&mut self, seed: &'a [u32]) {
        // make the seed into [seed[0], seed[1], ..., seed[seed.len()
        // - 1], 0, 0, ...], to fill rng.rsl.
        let seed_iter = seed.iter().map(|&x| x).chain(repeat(0u32));

        for (rsl_elem, seed_elem) in self.rsl.iter_mut().zip(seed_iter) {
            *rsl_elem = w(seed_elem);
        }
        self.cnt = 0;
        self.a = w(0);
        self.b = w(0);
        self.c = w(0);

        self.init(true);
    }

    /// Create an ISAAC random number generator with a seed. This can
    /// be any length, although the maximum number of elements used is
    /// 256 and any more will be silently ignored. A generator
    /// constructed with a given seed will generate the same sequence
    /// of values as all other generators constructed with that seed.
    fn from_seed(seed: &'a [u32]) -> IsaacRng {
        let mut rng = EMPTY;
        rng.reseed(seed);
        rng
    }
}

impl fmt::Debug for IsaacRng {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IsaacRng {{}}")
    }
}

const RAND_SIZE_64_LEN: usize = 8;
const RAND_SIZE_64: usize = 1 << RAND_SIZE_64_LEN;

/// A random number generator that uses ISAAC-64[1], the 64-bit
/// variant of the ISAAC algorithm.
///
/// The ISAAC algorithm is generally accepted as suitable for
/// cryptographic purposes, but this implementation has not be
/// verified as such. Prefer a generator like `OsRng` that defers to
/// the operating system for cases that need high security.
///
/// [1]: Bob Jenkins, [*ISAAC: A fast cryptographic random number
/// generator*](http://www.burtleburtle.net/bob/rand/isaacafa.html)
#[derive(Copy)]
pub struct Isaac64Rng {
    cnt: usize,
    rsl: [w64; RAND_SIZE_64],
    mem: [w64; RAND_SIZE_64],
    a: w64,
    b: w64,
    c: w64,
}

static EMPTY_64: Isaac64Rng = Isaac64Rng {
    cnt: 0,
    rsl: [w(0); RAND_SIZE_64],
    mem: [w(0); RAND_SIZE_64],
    a: w(0), b: w(0), c: w(0),
};

impl Isaac64Rng {
    /// Create a 64-bit ISAAC random number generator using the
    /// default fixed seed.
    pub fn new_unseeded() -> Isaac64Rng {
        let mut rng = EMPTY_64;
        rng.init(false);
        rng
    }

    /// Initialises `self`. If `use_rsl` is true, then use the current value
    /// of `rsl` as a seed, otherwise construct one algorithmically (not
    /// randomly).
    fn init(&mut self, use_rsl: bool) {
        macro_rules! init {
            ($var:ident) => (
                let mut $var = w(0x9e3779b97f4a7c13);
            )
        }
        init!(a); init!(b); init!(c); init!(d);
        init!(e); init!(f); init!(g); init!(h);

        macro_rules! mix {
            () => {{
                a=a-e; f=f^(h>>9);  h=h+a;
                b=b-f; g=g^(a<<9);  a=a+b;
                c=c-g; h=h^(b>>23); b=b+c;
                d=d-h; a=a^(c<<15); c=c+d;
                e=e-a; b=b^(d>>14); d=d+e;
                f=f-b; c=c^(e<<20); e=e+f;
                g=g-c; d=d^(f>>17); f=f+g;
                h=h-d; e=e^(g<<14); g=g+h;
            }}
        }

        for _ in 0..4 {
            mix!();
        }

        if use_rsl {
            macro_rules! memloop {
                ($arr:expr) => {{
                    for i in (0..RAND_SIZE_64 / 8).map(|i| i * 8) {
                        a=a+$arr[i  ]; b=b+$arr[i+1];
                        c=c+$arr[i+2]; d=d+$arr[i+3];
                        e=e+$arr[i+4]; f=f+$arr[i+5];
                        g=g+$arr[i+6]; h=h+$arr[i+7];
                        mix!();
                        self.mem[i  ]=a; self.mem[i+1]=b;
                        self.mem[i+2]=c; self.mem[i+3]=d;
                        self.mem[i+4]=e; self.mem[i+5]=f;
                        self.mem[i+6]=g; self.mem[i+7]=h;
                    }
                }}
            }

            memloop!(self.rsl);
            memloop!(self.mem);
        } else {
            for i in (0..RAND_SIZE_64 / 8).map(|i| i * 8) {
                mix!();
                self.mem[i  ]=a; self.mem[i+1]=b;
                self.mem[i+2]=c; self.mem[i+3]=d;
                self.mem[i+4]=e; self.mem[i+5]=f;
                self.mem[i+6]=g; self.mem[i+7]=h;
            }
        }

        self.isaac64();
    }

    /// Refills the output buffer (`self.rsl`)
    fn isaac64(&mut self) {
        self.c = self.c + w(1);
        // abbreviations
        let mut a = self.a;
        let mut b = self.b + self.c;
        const MIDPOINT: usize =  RAND_SIZE_64 / 2;
        const MP_VEC: [(usize, usize); 2] = [(0,MIDPOINT), (MIDPOINT, 0)];
        macro_rules! ind {
            ($x:expr) => {
                *self.mem.get_unchecked((($x >> 3usize).0 as usize) & (RAND_SIZE_64 - 1))
            }
        }

        for &(mr_offset, m2_offset) in MP_VEC.iter() {
            for base in (0..MIDPOINT / 4).map(|i| i * 4) {

                macro_rules! rngstepp {
                    ($j:expr, $shift:expr) => {{
                        let base = base + $j;
                        let mix = a ^ (a << $shift);
                        let mix = if $j == 0 {!mix} else {mix};

                        unsafe {
                            let x = *self.mem.get_unchecked(base + mr_offset);
                            a = mix + *self.mem.get_unchecked(base + m2_offset);
                            let y = ind!(x) + a + b;
                            *self.mem.get_unchecked_mut(base + mr_offset) = y;

                            b = ind!(y >> RAND_SIZE_64_LEN) + x;
                            *self.rsl.get_unchecked_mut(base + mr_offset) = b;
                        }
                    }}
                }

                macro_rules! rngstepn {
                    ($j:expr, $shift:expr) => {{
                        let base = base + $j;
                        let mix = a ^ (a >> $shift);
                        let mix = if $j == 0 {!mix} else {mix};

                        unsafe {
                            let x = *self.mem.get_unchecked(base + mr_offset);
                            a = mix + *self.mem.get_unchecked(base + m2_offset);
                            let y = ind!(x) + a + b;
                            *self.mem.get_unchecked_mut(base + mr_offset) = y;

                            b = ind!(y >> RAND_SIZE_64_LEN) + x;
                            *self.rsl.get_unchecked_mut(base + mr_offset) = b;
                        }
                    }}
                }

                rngstepp!(0, 21);
                rngstepn!(1, 5);
                rngstepp!(2, 12);
                rngstepn!(3, 33);
            }
        }

        self.a = a;
        self.b = b;
        self.cnt = RAND_SIZE_64;
    }
}

// Cannot be derived because [u32; 256] does not implement Clone
impl Clone for Isaac64Rng {
    fn clone(&self) -> Isaac64Rng {
        *self
    }
}

impl Rng for Isaac64Rng {
    #[inline]
    fn next_u32(&mut self) -> Result<u32, CryptoError> {
        Ok(self.next_u64()? as u32)
    }

    #[inline]
    fn next_u64(&mut self) -> Result<u64, CryptoError> {
        if self.cnt == 0 {
            // make some more numbers
            self.isaac64();
        }
        self.cnt -= 1;

        // See corresponding location in IsaacRng.next_u32 for
        // explanation.
        debug_assert!(self.cnt < RAND_SIZE_64);
        Ok(self.rsl[(self.cnt % RAND_SIZE_64) as usize].0)
    }
}

impl FromRng for Isaac64Rng {
    fn from_rng<R: Rng+?Sized>(other: &mut R) -> Result<Isaac64Rng, CryptoError> {
        let mut ret = EMPTY_64;
        unsafe {
            let ptr = ret.rsl.as_mut_ptr() as *mut u8;

            let slice = slice::from_raw_parts_mut(ptr, RAND_SIZE_64 * 8);
            other.fill_bytes(slice)?;
        }
        ret.cnt = 0;
        ret.a = w(0);
        ret.b = w(0);
        ret.c = w(0);

        ret.init(true);
        Ok(ret)
    }
}

impl<'a> SeedableRng<&'a [u64]> for Isaac64Rng {
    fn reseed(&mut self, seed: &'a [u64]) {
        // make the seed into [seed[0], seed[1], ..., seed[seed.len()
        // - 1], 0, 0, ...], to fill rng.rsl.
        let seed_iter = seed.iter().map(|&x| x).chain(repeat(0u64));

        for (rsl_elem, seed_elem) in self.rsl.iter_mut().zip(seed_iter) {
            *rsl_elem = w(seed_elem);
        }
        self.cnt = 0;
        self.a = w(0);
        self.b = w(0);
        self.c = w(0);

        self.init(true);
    }

    /// Create an ISAAC random number generator with a seed. This can
    /// be any length, although the maximum number of elements used is
    /// 256 and any more will be silently ignored. A generator
    /// constructed with a given seed will generate the same sequence
    /// of values as all other generators constructed with that seed.
    fn from_seed(seed: &'a [u64]) -> Isaac64Rng {
        let mut rng = EMPTY_64;
        rng.reseed(seed);
        rng
    }
}

impl fmt::Debug for Isaac64Rng {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Isaac64Rng {{}}")
    }
}

#[cfg(test)]
mod test {
    use {Rng, SeedableRng, iter};
    use distributions::ascii_word_char;
    use super::{IsaacRng, Isaac64Rng};

    #[test]
    fn test_rng_32_rand_seeded() {
        let s = iter(&mut ::test::rng()).map(|rng| rng.next_u32()).take(256).collect::<Vec<u32>>();
        let mut ra: IsaacRng = SeedableRng::from_seed(&s[..]);
        let mut rb: IsaacRng = SeedableRng::from_seed(&s[..]);
        assert!(::test::iter_eq(iter(&mut ra).map(|rng| ascii_word_char(rng)).take(100),
                                iter(&mut rb).map(|rng| ascii_word_char(rng)).take(100)));
    }
    #[test]
    fn test_rng_64_rand_seeded() {
        let s = iter(&mut ::test::rng()).map(|rng| rng.next_u64()).take(256).collect::<Vec<u64>>();
        let mut ra: Isaac64Rng = SeedableRng::from_seed(&s[..]);
        let mut rb: Isaac64Rng = SeedableRng::from_seed(&s[..]);
        assert!(::test::iter_eq(iter(&mut ra).map(|rng| ascii_word_char(rng)).take(100),
                                iter(&mut rb).map(|rng| ascii_word_char(rng)).take(100)));
    }

    #[test]
    fn test_rng_32_seeded() {
        let seed: &[_] = &[1, 23, 456, 7890, 12345];
        let mut ra: IsaacRng = SeedableRng::from_seed(seed);
        let mut rb: IsaacRng = SeedableRng::from_seed(seed);
        assert!(::test::iter_eq(iter(&mut ra).map(|rng| ascii_word_char(rng)).take(100),
                                iter(&mut rb).map(|rng| ascii_word_char(rng)).take(100)));
    }
    #[test]
    fn test_rng_64_seeded() {
        let seed: &[_] = &[1, 23, 456, 7890, 12345];
        let mut ra: Isaac64Rng = SeedableRng::from_seed(seed);
        let mut rb: Isaac64Rng = SeedableRng::from_seed(seed);
        assert!(::test::iter_eq(iter(&mut ra).map(|rng| ascii_word_char(rng)).take(100),
                                iter(&mut rb).map(|rng| ascii_word_char(rng)).take(100)));
    }

    #[test]
    fn test_rng_32_reseed() {
        let s = iter(&mut ::test::rng()).map(|rng| rng.next_u32()).take(256).collect::<Vec<u32>>();
        let mut r: IsaacRng = SeedableRng::from_seed(&s[..]);
        let string1: String = iter(&mut r).map(|rng| ascii_word_char(rng)).take(100).collect();

        r.reseed(&s[..]);

        let string2: String = iter(&mut r).map(|rng| ascii_word_char(rng)).take(100).collect();
        assert_eq!(string1, string2);
    }
    #[test]
    fn test_rng_64_reseed() {
        let s = iter(&mut ::test::rng()).map(|rng| rng.next_u64()).take(256).collect::<Vec<u64>>();
        let mut r: Isaac64Rng = SeedableRng::from_seed(&s[..]);
        let string1: String = iter(&mut r).map(|rng| ascii_word_char(rng)).take(100).collect();

        r.reseed(&s[..]);

        let string2: String = iter(&mut r).map(|rng| ascii_word_char(rng)).take(100).collect();
        assert_eq!(string1, string2);
    }

    #[test]
    fn test_rng_32_true_values() {
        let seed: &[_] = &[1, 23, 456, 7890, 12345];
        let mut ra: IsaacRng = SeedableRng::from_seed(seed);
        // Regression test that isaac is actually using the above vector
        let v = (0..10).map(|_| ra.next_u32().unwrap()).collect::<Vec<_>>();
        assert_eq!(v,
                   vec!(2558573138, 873787463, 263499565, 2103644246, 3595684709,
                        4203127393, 264982119, 2765226902, 2737944514, 3900253796));

        let seed: &[_] = &[12345, 67890, 54321, 9876];
        let mut rb: IsaacRng = SeedableRng::from_seed(seed);
        // skip forward to the 10000th number
        for _ in 0..10000 { rb.next_u32(); }

        let v = (0..10).map(|_| rb.next_u32().unwrap()).collect::<Vec<_>>();
        assert_eq!(v,
                   vec!(3676831399, 3183332890, 2834741178, 3854698763, 2717568474,
                        1576568959, 3507990155, 179069555, 141456972, 2478885421));
    }
    #[test]
    fn test_rng_64_true_values() {
        let seed: &[_] = &[1, 23, 456, 7890, 12345];
        let mut ra: Isaac64Rng = SeedableRng::from_seed(seed);
        // Regression test that isaac is actually using the above vector
        let v = (0..10).map(|_| ra.next_u64().unwrap()).collect::<Vec<_>>();
        assert_eq!(v,
                   vec!(547121783600835980, 14377643087320773276, 17351601304698403469,
                        1238879483818134882, 11952566807690396487, 13970131091560099343,
                        4469761996653280935, 15552757044682284409, 6860251611068737823,
                        13722198873481261842));

        let seed: &[_] = &[12345, 67890, 54321, 9876];
        let mut rb: Isaac64Rng = SeedableRng::from_seed(seed);
        // skip forward to the 10000th number
        for _ in 0..10000 { rb.next_u64(); }

        let v = (0..10).map(|_| rb.next_u64().unwrap()).collect::<Vec<_>>();
        assert_eq!(v,
                   vec!(18143823860592706164, 8491801882678285927, 2699425367717515619,
                        17196852593171130876, 2606123525235546165, 15790932315217671084,
                        596345674630742204, 9947027391921273664, 11788097613744130851,
                        10391409374914919106));
    }

    #[test]
    fn test_rng_clone() {
        let seed: &[_] = &[1, 23, 456, 7890, 12345];
        let mut rng: Isaac64Rng = SeedableRng::from_seed(seed);
        let mut clone = rng.clone();
        for _ in 0..16 {
            assert_eq!(rng.next_u64().unwrap(), clone.next_u64());
        }
    }
}
