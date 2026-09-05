use crate::u256::U256;

/// Errors returned by the Phase 1 arithmetic primitives. Every failure mode is typed;
/// nothing in this crate panics on a caller-reachable input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    /// The divisor was zero.
    DivisionByZero,
    /// The exact mathematical result does not fit in a `u128`.
    Overflow,
}

/// Computes `floor(a * b / d)` using a 256-bit intermediate product, so the multiplication
/// itself can never overflow even when the final quotient fits comfortably in a `u128`
/// (see ADR-0009 and `U-ARITH-04`).
pub fn mul_div_floor(a: u128, b: u128, d: u128) -> Result<u128, MathError> {
    if d == 0 {
        return Err(MathError::DivisionByZero);
    }
    U256::mul128(a, b)
        .div_u128(d)
        .map(|(quotient, _remainder)| quotient)
        .ok_or(MathError::Overflow)
}

/// Computes `ceil(a * b / d)` as `(a * b + d - 1) / d` in 256-bit space.
pub fn mul_div_ceil(a: u128, b: u128, d: u128) -> Result<u128, MathError> {
    if d == 0 {
        return Err(MathError::DivisionByZero);
    }
    let numerator = U256::mul128(a, b)
        .checked_add_u128(d - 1)
        .ok_or(MathError::Overflow)?;
    numerator
        .div_u128(d)
        .map(|(quotient, _remainder)| quotient)
        .ok_or(MathError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    // U-ARITH-01 — known vectors.
    #[test]
    fn known_vectors() {
        assert_eq!(mul_div_floor(3, 5, 2), Ok(7));
        assert_eq!(mul_div_ceil(3, 5, 2), Ok(8));

        assert_eq!(mul_div_floor(10, 10, 5), Ok(20));
        assert_eq!(mul_div_ceil(10, 10, 5), Ok(20));

        assert_eq!(mul_div_floor(0, 100, 7), Ok(0));
        assert_eq!(mul_div_ceil(0, 100, 7), Ok(0));

        assert_eq!(mul_div_floor(1, 1, 1), Ok(1));
        assert_eq!(mul_div_ceil(1, 1, 1), Ok(1));
    }

    // U-ARITH-02 — division by zero returns a typed error, never panics.
    #[test]
    fn division_by_zero() {
        assert_eq!(mul_div_floor(1, 1, 0), Err(MathError::DivisionByZero));
        assert_eq!(mul_div_ceil(1, 1, 0), Err(MathError::DivisionByZero));
        assert_eq!(mul_div_floor(0, 0, 0), Err(MathError::DivisionByZero));
    }

    // U-ARITH-03 — a result exceeding u128::MAX returns a typed overflow error.
    #[test]
    fn result_overflow() {
        assert_eq!(
            mul_div_floor(u128::MAX, u128::MAX, 1),
            Err(MathError::Overflow)
        );
        assert_eq!(
            mul_div_ceil(u128::MAX, u128::MAX, 1),
            Err(MathError::Overflow)
        );
        // Product overflows u128 and the quotient still does not fit: d = 2 halves a
        // ~2^256 product down to ~2^255, still far beyond u128::MAX.
        assert_eq!(
            mul_div_floor(u128::MAX, u128::MAX, 2),
            Err(MathError::Overflow)
        );
    }

    // U-ARITH-04 — the case that overflows a naive u128 multiplication but whose exact
    // result fits in u128. This is the entire justification for the 256-bit intermediate.
    #[test]
    fn large_multiplication_survives_256_bit_intermediate() {
        let a: u128 = 18_000_000_000_000_000_000_000_000; // 1.8e25
        let b: u128 = 18_000_000_000_000_000_000; // 1.8e19
        let d: u128 = 1_000_000_000_000_000_000; // 1e18

        // a * b ~= 3.24e44, which overflows u128::MAX (~3.4e38) by six orders of
        // magnitude, but a*b/d ~= 3.24e26 fits comfortably.
        assert!(
            a.checked_mul(b).is_none(),
            "test input must overflow naive u128 multiplication"
        );

        // b / d == 18 exactly, so a * b / d == a * 18: an independent way to derive the
        // expected value that never itself goes through mul_div.
        let expected = a * 18;

        let result = mul_div_floor(a, b, d).expect("must succeed with a 256-bit intermediate");
        assert_eq!(result, expected);

        let ceil_result = mul_div_ceil(a, b, d).expect("must succeed with a 256-bit intermediate");
        assert_eq!(ceil_result, expected);
    }

    #[test]
    fn ceil_only_rounds_up_on_a_nonzero_remainder() {
        assert_eq!(mul_div_floor(7, 1, 2), Ok(3));
        assert_eq!(mul_div_ceil(7, 1, 2), Ok(4));
        assert_eq!(mul_div_floor(8, 1, 2), Ok(4));
        assert_eq!(mul_div_ceil(8, 1, 2), Ok(4));
    }
}
