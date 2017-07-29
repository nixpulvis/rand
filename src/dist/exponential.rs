// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The exponential distribution.

use {Rng};
use dist::{ziggurat, ziggurat_tables, Sample, uniform01};

/// Generates Exp(1) random numbers.
///
/// See `Exp` for the general exponential distribution.
///
/// Implemented via the ZIGNOR variant[1] of the Ziggurat method. The
/// exact description in the paper was adjusted to use tables for the
/// exponential distribution rather than normal.
///
/// [1]: Jurgen A. Doornik (2005). [*An Improved Ziggurat Method to
/// Generate Normal Random
/// Samples*](http://www.doornik.com/research/ziggurat.pdf). Nuffield
/// College, Oxford
///
/// # Example
///
/// ```rust
/// use rand::dist::exponential::exp1;
///
/// let x = exp1(&mut rand::thread_rng());
/// println!("{}", x);
/// ```
// This could be done via `-rng.gen::<f64>().ln()` but that is slower.
#[inline]
pub fn exp1<R: Rng>(rng: &mut R) -> f64 {
    #[inline]
    fn pdf(x: f64) -> f64 {
        (-x).exp()
    }
    #[inline]
    fn zero_case<R:Rng>(rng: &mut R, _u: f64) -> f64 {
        ziggurat_tables::ZIG_EXP_R - uniform01::<f64, _>(rng).ln()
    }

    (ziggurat(rng, false,
                    &ziggurat_tables::ZIG_EXP_X,
                    &ziggurat_tables::ZIG_EXP_F,
                    pdf, zero_case))
}

/// The exponential distribution `Exp(lambda)`.
///
/// This distribution has density function: `f(x) = lambda *
/// exp(-lambda * x)` for `x > 0`.
///
/// # Example
///
/// ```rust
/// use rand::dist::{Exp, Sample};
///
/// let exp = Exp::new(2.0);
/// let v = exp.sample(&mut rand::thread_rng());
/// println!("{} is from a Exp(2) distribution", v);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Exp {
    /// `lambda` stored as `1/lambda`, since this is what we scale by.
    lambda_inverse: f64
}

impl Exp {
    /// Construct a new `Exp` with the given shape parameter
    /// `lambda`. Panics if `lambda <= 0`.
    #[inline]
    pub fn new(lambda: f64) -> Exp {
        assert!(lambda > 0.0, "Exp::new called with `lambda` <= 0");
        Exp { lambda_inverse: 1.0 / lambda }
    }
}

impl Sample<f64> for Exp {
    fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        exp1(rng) * self.lambda_inverse
    }
}

#[cfg(test)]
mod test {
    use dist::{Sample};
    use super::Exp;

    #[test]
    fn test_exp() {
        let exp = Exp::new(10.0);
        let mut rng = ::test::rng();
        for _ in 0..1000 {
            assert!(exp.sample(&mut rng) >= 0.0);
        }
    }
    #[test]
    #[should_panic]
    fn test_exp_invalid_lambda_zero() {
        Exp::new(0.0);
    }
    #[test]
    #[should_panic]
    fn test_exp_invalid_lambda_neg() {
        Exp::new(-10.0);
    }
}