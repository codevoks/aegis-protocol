//! Property tests for `mul_div_floor` / `mul_div_ceil`.
//!
//! `P-ARITH-1`, `P-ARITH-2`, `P-ARITH-3` from `docs/phases/phase-01-foundation.md` §6.
//! `P-ARITH-3` checks against `num-bigint`, an independent implementation from the
//! hand-rolled 256-bit division in `aegis-math`, so it cannot share a bug with it.

use aegis_math::{mul_div_ceil, mul_div_floor, MathError};
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use proptest::prelude::*;

fn reference_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    let product = BigUint::from(a) * BigUint::from(b);
    (&product / BigUint::from(d)).to_u128()
}

fn reference_ceil(a: u128, b: u128, d: u128) -> Option<u128> {
    let product = BigUint::from(a) * BigUint::from(b);
    let d_big = BigUint::from(d);
    let quotient = &product / &d_big;
    let remainder = &product % &d_big;
    let ceil = if remainder == BigUint::from(0u32) {
        quotient
    } else {
        quotient + BigUint::from(1u32)
    };
    ceil.to_u128()
}

proptest! {
    // P-ARITH-2 — never panics for any (a, b, d), including d == 0. A panic aborts the
    // proptest run itself, so simply calling both functions over the full input space is
    // the test.
    #[test]
    fn never_panics(a: u128, b: u128, d: u128) {
        let _ = mul_div_floor(a, b, d);
        let _ = mul_div_ceil(a, b, d);
    }

    // P-ARITH-1 — floor <= ceil <= floor + 1 for all inputs (restricted to nonzero
    // divisors here; d == 0 is covered by `U-ARITH-02` and by `never_panics` above).
    #[test]
    fn floor_le_ceil_le_floor_plus_one(a: u128, b: u128, d in 1..=u128::MAX) {
        match (mul_div_floor(a, b, d), mul_div_ceil(a, b, d)) {
            (Ok(floor), Ok(ceil)) => {
                prop_assert!(ceil >= floor);
                prop_assert!(ceil == floor || ceil == floor + 1);
            }
            (Err(MathError::Overflow), Err(MathError::Overflow)) => {
                // The exact quotient exceeds u128::MAX either way; ceil (>= floor) can
                // only fail to fit as well.
            }
            (Err(MathError::Overflow), Ok(_)) => {
                prop_assert!(false, "ceil succeeded while floor overflowed");
            }
            other => prop_assert!(false, "unexpected result pair: {:?}", other),
        }
    }

    // P-ARITH-3 — exact agreement with an independent bignum reference.
    #[test]
    fn matches_bignum_reference(a: u128, b: u128, d in 1..=u128::MAX) {
        match reference_floor(a, b, d) {
            Some(expected) => prop_assert_eq!(mul_div_floor(a, b, d), Ok(expected)),
            None => prop_assert_eq!(mul_div_floor(a, b, d), Err(MathError::Overflow)),
        }
        match reference_ceil(a, b, d) {
            Some(expected) => prop_assert_eq!(mul_div_ceil(a, b, d), Ok(expected)),
            None => prop_assert_eq!(mul_div_ceil(a, b, d), Err(MathError::Overflow)),
        }
    }
}
