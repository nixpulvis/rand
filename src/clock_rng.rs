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
use time::precise_time_ns;

/// Clock-based `Rng`. Not very random.
pub struct ClockRng {}

impl ClockRng {
    /// Create a `ClockRng` (very low cost)
    pub fn new() -> ClockRng {
        ClockRng {}
    }
}

impl Rng for ClockRng {
    fn next_u32(&mut self) -> u32 {
        // Take only the highest-precision 32 bits (~4 sec) and throw away the
        // rest. If using repeatedly the rest will be identical each time anyway,
        // and is also much more predictable.
        precise_time_ns() as u32
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
