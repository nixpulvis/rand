// Copyright 2013-2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Random number generation traits
//! 
//! This crate is mainly of interest to crates publishing implementations of
//! `Rng`. Other users are encouraged to use the
//! [rand crate](https://crates.io/crates/rand) instead.
//!
//! `Rng` is the core trait implemented by algorithmic pseudo-random number
//! generators and external random-number sources.
//! 
//! `SeedFromRng` and `SeedableRng` are extension traits for construction and
//! reseeding.
//! 
//! `Error` is provided for error-handling. It is safe to use in `no_std`
//! environments.
//! 
//! The `impls` sub-module includes a few small functions to assist
//! implementation of `Rng`. Since this module is only of interest to `Rng`
//! implementors, it is not re-exported from `rand`.
//! 
//! The `mock` module includes a mock `Rng` implementation. Even though this is
//! only useful for testing, it is currently always built.

#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk.png",
       html_favicon_url = "https://www.rust-lang.org/favicon.ico",
       html_root_url = "https://docs.rs/rand/0.3")]

#![deny(missing_debug_implementations)]

#![cfg_attr(not(feature="std"), no_std)]
#![cfg_attr(feature = "i128_support", feature(i128_type))]

// We need to use several items from "core" for no_std support.
#[cfg(feature="std")]
extern crate core;

use core::fmt;

pub mod impls;


/// A random number generator (not necessarily suitable for cryptography).
/// 
/// "Random" number generators can be categorised multiple ways:
/// 
/// *   *True* and *apparent* random number generators: *true* generators must
///     depend on some phenomenon which is actually random, such as atomic decay
///     or photon polarisation (note: bias may still be present, and it is
///     possible that these phenomena are in fact dependent on other unseen
///     parameters), while *apparent* random numbers are merely unpredictable.
/// *   *Algorithmic* and *external* generators: *algorithmic generators* can
///     never produce *true random numbers* but can still yield hard-to-predict
///     output. External generators may or may not use *true random sources*.
///     
///     *Algorithmic generators* are also known as *psuedo-random number
///     generators* or *PRNGs*.
///     
///     *Algorithmic* generators are necessarily deterministic: if two
///     generators using the same algorithm are initialised with the same seed,
///     they will necessarily produce the same output. PRNGs should normally
///     implement the `SeedableRng` trait, allowing this. To be reproducible
///     across platforms, conversion of output from one type to another should
///     pay special attention to endianness (we generally choose LE, e.g.
///     `fn next_u32(&mut self) -> u32 { self.next_u64() as u32 }`).
/// *   *Cryptographically secure*, *trivially predictable*, and various states
///     in between: if, after observing some output from an algorithmic
///     generator future output of the generator can be predicted, then it is
///     insecure.
///     
///     Note that all algorithmic generators eventually cycle,
///     returning to previous internal state and repeating all output, but in
///     good generators the period is so long that it is never reached (e.g. our
///     implementation of ChaCha would produce 2^134 bytes of output before
///     cycling, although cycles are not always so long). Predictability may not
///     be a problem for games, simulations and some randomised algorithms,
///     but unpredictability is essential in cryptography.
/// 
/// The `Rng` trait can be used for all the above types of generators. If using
/// random numbers for cryptography prefer to use numbers pulled from the OS
/// directly (`OsRng`) or at least use a generator implementing `CryptoRng`.
/// For applications where performance is important and unpredictability is
/// less critical but still somewhat important (e.g. to prevent a DoS attack),
/// one may prefer to use a `CryptoRng` generator or may consider other "hard
/// to predict" but not cryptographically proven generators.
/// 
/// PRNGs are usually infallible, while external generators may fail. Since
/// errors are rare and may be hard for the user to handle, most of the output
/// functions only allow errors to be reported as panics; byte output can
/// however be retrieved from `try_fill` which allows for the usual error
/// handling.
pub trait Rng {
    /// Return the next random u32.
    fn next_u32(&mut self) -> u32;

    /// Return the next random u64.
    fn next_u64(&mut self) -> u64;

    /// Return the next random u128.
    #[cfg(feature = "i128_support")]
    fn next_u128(&mut self) -> u128;

    /// Fill `dest` entirely with random data.
    ///
    /// This method does *not* have any requirement on how much of the
    /// generated random number stream is consumed; e.g. `fill_bytes_via_u64`
    /// implementation uses `next_u64` thus consuming 8 bytes even when only
    /// 1 is required. A different implementation might use `next_u32` and
    /// only consume 4 bytes; *however* any change affecting *reproducibility*
    /// of output must be considered a breaking change.
    fn fill_bytes(&mut self, dest: &mut [u8]);

    /// Fill `dest` entirely with random data.
    ///
    /// If a RNG can encounter an error, this is the only method that reports
    /// it. The other methods either handle the error, or panic.
    ///
    /// This method does *not* have any requirement on how much of the
    /// generated random number stream is consumed; e.g. `try_fill_via_u64`
    /// implementation uses `next_u64` thus consuming 8 bytes even when only
    /// 1 is required. A different implementation might use `next_u32` and
    /// only consume 4 bytes; *however* any change affecting *reproducibility*
    /// of output must be considered a breaking change.
    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error>;
}

/// A marker trait for an `Rng` which may be considered for use in
/// cryptography.
/// 
/// *Cryptographically secure generators*, also known as *CSPRNGs*, should
/// satisfy an additional properties over other generators: given the first
/// *k* bits of an algorithm's output
/// sequence, it should not be possible using polynomial-time algorithms to
/// predict the next bit with probability significantly greater than 50%.
/// 
/// Some generators may satisfy an additional property, however this is not
/// required: if the CSPRNG's state is revealed, it should not be
/// computationally-feasible to reconstruct output prior to this. Some other
/// generators allow backwards-computation and are consided *reversible*.
/// 
/// Note that this trait is provided for guidance only and cannot guarantee
/// suitability for cryptographic applications. In general it should only be
/// implemented for well-reviewed code implementing well-regarded algorithms.
pub trait CryptoRng: Rng {}


impl<'a, R: Rng+?Sized> Rng for &'a mut R {
    fn next_u32(&mut self) -> u32 {
        (**self).next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }

    #[cfg(feature = "i128_support")]
    fn next_u128(&mut self) -> u128 {
        (**self).next_u128()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (**self).fill_bytes(dest)
    }

    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        (**self).try_fill(dest)
    }
}

#[cfg(feature="std")]
impl<R: Rng+?Sized> Rng for Box<R> {
    fn next_u32(&mut self) -> u32 {
        (**self).next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }

    #[cfg(feature = "i128_support")]
    fn next_u128(&mut self) -> u128 {
        (**self).next_u128()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (**self).fill_bytes(dest)
    }

    fn try_fill(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        (**self).try_fill(dest)
    }
}


/// Support mechanism for creating random number generators seeded by other
/// generators. All PRNGs should support this to enable `NewSeeded` support,
/// which should be the preferred way of creating randomly-seeded generators.
pub trait SeedFromRng: Sized {
    /// Creates a new instance, seeded from another `Rng`.
    /// 
    /// Seeding from a cryptographic generator should be fine. On the other
    /// hand, seeding a simple numerical generator from another of the same
    /// type sometimes has serious side effects such as effectively cloning the
    /// generator.
    fn from_rng<R: Rng>(rng: R) -> Result<Self, Error>;
}

/// A random number generator that can be explicitly seeded to produce
/// the same stream of randomness multiple times.
/// 
/// Note: this should only be implemented by reproducible generators (i.e.
/// where the algorithm is fixed and results should be the same across
/// platforms). This should not be implemented by wrapper types which choose
/// the underlying implementation based on platform, or which may change the
/// algorithm used in the future. This is to ensure that manual seeding of PRNGs
/// actually does yield reproducible results.
pub trait SeedableRng<Seed>: Rng {
    /// Create a new RNG with the given seed.
    /// 
    /// The type of `Seed` is specified by the implementation (implementation
    /// for multiple seed types is possible).
    fn from_seed(seed: Seed) -> Self;
}


/// Error kind which can be matched over.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ErrorKind {
    /// Permanent failure: likely not recoverable without user action.
    Unavailable,
    /// Temporary failure: recommended to retry a few times, but may also be
    /// irrecoverable.
    Transient,
    /// Not ready yet: recommended to try again a little later.
    NotReady,
    /// Uncategorised error
    Other,
    #[doc(hidden)]
    __Nonexhaustive,
}

impl ErrorKind {
    /// True if this kind of error may resolve itself on retry.
    /// 
    /// See also `should_wait()`.
    pub fn should_retry(self) -> bool {
        match self {
            ErrorKind::Transient | ErrorKind::NotReady => true,
            _ => false,
        }
    }
    /// True if we should wait before retrying
    pub fn should_wait(self) -> bool {
        match self {
            ErrorKind::NotReady => true,
            _ => false,
        }
    }
    /// A description of this error kind
    pub fn description(self) -> &'static str {
        match self {
            ErrorKind::Unavailable => "permanent failure or unavailable",
            ErrorKind::Transient => "transient failure",
            ErrorKind::NotReady => "not ready yet",
            ErrorKind::Other => "uncategorised",
            ErrorKind::__Nonexhaustive => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum ErrorDetails {
    None,
    Str(&'static str),
    #[cfg(feature="std")]
    Error(Box<std::error::Error + Send + Sync>),
}

impl fmt::Display for ErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ::std::error::Error;
        write!(f, "{}", self.description())
    }
}

#[cfg(feature="std")]
impl ::std::error::Error for ErrorDetails {
    fn description(&self) -> &str {
        match *self {
            ErrorDetails::Str(ref s) => s,
            _ => unreachable!(),
        }
    }
    
    // no cause: Error case gets unwrapped by impl for Error
}

/// Error type of random number generators
/// 
/// This embeds two things, an `ErrorKind`, which can be matched over, and
/// optional details provided to `new_str` or `new_err`.
#[derive(Debug)]
pub struct Error {
    /// Error kind
    pub kind: ErrorKind,
    details: ErrorDetails,
}

impl Error {
    /// Create a new instance, with no details.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, details: ErrorDetails::None }
    }
    
    /// Create a new instance, with a static string describing the details.
    /// 
    /// This may be used in both `std` and `no_std` enviroments.
    pub fn new_str(kind: ErrorKind, details: &'static str) -> Self {
        Self { kind, details: ErrorDetails::Str(details) }
    }
    
    /// Create a new instance, with a chained `Error` cause.
    #[cfg(feature="std")]
    pub fn new_err<E>(kind: ErrorKind, cause: E) -> Self
        where E: Into<Box<std::error::Error + Send + Sync>>
    {
        Self { kind, details: ErrorDetails::Error(cause.into()) }
    }
    
    /// Get details on the error, if available (string message or chained
    /// "cause").
    /// 
    /// In the case of a chained error, the actual error can be obtained with
    /// `std::error::Error::cause`.
    pub fn details(&self) -> Option<&str> {
        match self.details {
            ErrorDetails::None => None,
            ErrorDetails::Str(ref s) => Some(s),
            ErrorDetails::Error(ref e) => Some(e.description()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RNG error: {}", self.kind.description())
    }
}

#[cfg(feature="std")]
impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.kind.description()
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match self.details {
            ErrorDetails::None => None,
            // use a trick: implement Error for ErrorDetails
            ErrorDetails::Str(_) => Some(&self.details),
            ErrorDetails::Error(ref e) => Some(&**e),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ErrorKind;
    use std::error::Error;  // for description() and cause() functions
    
    #[test]
    fn description() {
        let kind = ErrorKind::NotReady;
        let err = super::Error::new_str(kind, "abc");
        assert_eq!(kind.description(), err.description());
    }
    
    #[test]
    fn cause() {
        let kind = ErrorKind::NotReady;
        let err = super::Error::new_str(kind, "abc");   // static string
        assert_eq!(err.details(), Some("abc"));
        assert_eq!(err.cause().map(|e| e.description()), Some("abc"));
        
        assert_eq!(super::Error::new(kind).details(), None);
        
        let err = super::Error::new_err(kind, "def");   // automatically boxed
        assert_eq!(err.details(), Some("def"));
        assert_eq!(err.cause().map(|e| e.description()), Some("def"));
    }
}
