// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A not-very-random number generator using the system clock.

use {Rng, Error};
use rand_core::impls;
use core::num::Wrapping as w;

/// Clock-based `Rng`.
/// 
/// This is designed as a fast, failsafe alternative to `OsRng`, getting its
/// entropy from the system clock. It could be used directly (but should be
/// considered low-quality and non-deterministic) or could be used to seed
/// another generator via `SeedFromRng`.
/// 
/// The time is checked once per `u32` extracted and mixed into the current
/// state via a RNG, hence in theory long output sequences will contain slightly
/// more entropy than short ones.
#[derive(Debug)]
pub struct ClockRng {
    state: w<u64>,
}

impl ClockRng {
    /// Create a `ClockRng`, and call `advance` a few times to improve initial
    /// endianness.
    /// 
    /// The number of `rounds` used during initialisation may be specified.
    /// Recommended to use at least 2, and up to 32 for "best" initialisation
    /// (using an estimate of 2-4 bits entropy per round, over 64 bits of state),
    /// but any number (including 0) can be used.
    /// Has some impact on init time.
    pub fn new(rounds: usize) -> ClockRng {
        let mut r = ClockRng { state: w(0) };
        for _ in 0..rounds { r.advance(); }
        r
    }
    
    /// Advance the internal state (equivalent to calling `next_u32` but
    /// without generating any output).
    #[inline(always)]
    pub fn advance(&mut self) {
        // Permute the state with time via the PCG algorithm.
        // Vary our increment (<<1 because it must be odd)
        let incr = (w(get_time()) << 1) ^ w(17707716133202733827);
        // Multipier from PCG source:
        self.state = self.state * w(6364136223846793005) + incr;
    }
}

impl Rng for ClockRng {
    fn next_u32(&mut self) -> u32 {
        self.advance();
        let state = self.state;
        
        // PCG output function:
        let xorshifted = ((state >> 18) ^ state) >> 27;
        let rot = state >> 59;
        let rot2 = (-rot) & w(31);
        ((xorshifted >> rot.0 as usize) | (xorshifted << rot2.0 as usize)).0 as u32
    }

    fn next_u64(&mut self) -> u64 {
        // Throw away the low-precision part and use the rest twice.
        impls::next_u64_via_u32(self)
    }
    
    #[cfg(feature = "i128_support")]
    fn next_u128(&mut self) -> u128 {
        impls::next_u128_via_u64(self)
    }
    
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        impls::fill_bytes_via_u64(self, dest)
    }

    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        Ok(self.fill_bytes(dest))
    }
}

fn get_time() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    dur.as_secs() * 1_000_000_000 + dur.subsec_nanos() as u64
}


#[cfg(test)]
mod test {
    use Rng;
    use super::ClockRng;
    
    #[test]
    fn distinct() {
        let mut c1 = ClockRng::new(0);
        let mut c2 = ClockRng::new(0);
        // probabilistic; very small chance of accidental failure
        assert!(c1.next_u64() != c2.next_u64());
    }
}
