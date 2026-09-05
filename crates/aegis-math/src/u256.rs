//! Minimal 256-bit unsigned integer support: just enough to multiply two `u128` values
//! into an exact 256-bit product and divide a 256-bit value by a `u128` divisor.
//!
//! Kept internal and deliberately small (per `docs/phases/phase-01-foundation.md` §6/§15,
//! a hand-rolled two-limb type is preferred over a dependency here) so every line is
//! reviewable against the arithmetic it implements.

const LIMB_BITS: u32 = 128;
const MASK64: u128 = u64::MAX as u128;

/// A 256-bit unsigned integer stored as two `u128` limbs: `value = hi * 2^128 + lo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct U256 {
    pub hi: u128,
    pub lo: u128,
}

impl U256 {
    pub const ZERO: Self = Self { hi: 0, lo: 0 };

    /// Exact 256-bit product of two `u128` values, computed from four 64-bit-limb
    /// partial products so no intermediate step can lose precision.
    pub fn mul128(a: u128, b: u128) -> Self {
        let a_lo = a & MASK64;
        let a_hi = a >> 64;
        let b_lo = b & MASK64;
        let b_hi = b >> 64;

        // Every partial product multiplies two values < 2^64, so each fits exactly in u128.
        let ll = a_lo * b_lo;
        let lh = a_lo * b_hi;
        let hl = a_hi * b_lo;
        let hh = a_hi * b_hi;

        let (cross, cross_overflow) = lh.overflowing_add(hl);
        let cross_low = (cross & MASK64) << 64;
        let cross_high = cross >> 64;

        let (lo, lo_carry) = ll.overflowing_add(cross_low);
        let hi = hh + cross_high + ((cross_overflow as u128) << 64) + (lo_carry as u128);

        Self { hi, lo }
    }

    /// Adds a `u128` scalar to this value, returning `None` if the true sum needs more
    /// than 256 bits (unreachable for any product of two `u128` values plus a `u128`
    /// scalar, but checked rather than assumed).
    pub fn checked_add_u128(self, rhs: u128) -> Option<Self> {
        let (lo, carry) = self.lo.overflowing_add(rhs);
        let hi = self.hi.checked_add(carry as u128)?;
        Some(Self { hi, lo })
    }

    fn bit(&self, i: u32) -> u128 {
        if i < LIMB_BITS {
            (self.lo >> i) & 1
        } else {
            (self.hi >> (i - LIMB_BITS)) & 1
        }
    }

    /// Divides this 256-bit value by a nonzero `u128` divisor via bit-by-bit binary long
    /// division, returning `(quotient, remainder)`. Returns `None` if the exact quotient
    /// does not fit in a `u128` (the caller's overflow case), and never panics.
    ///
    /// The per-bit step tracks the remainder as a value that is always `< d <= u128::MAX`.
    /// Shifting it left by one bit can momentarily need a 129th bit; that bit is captured
    /// explicitly as `carry` rather than by widening, so every step stays within `u128`
    /// and the `mul_div_*` callers can run under `overflow-checks = true` without risk of
    /// panicking on a spurious subtraction underflow or shift overflow.
    pub fn div_u128(self, d: u128) -> Option<(u128, u128)> {
        debug_assert!(d != 0);

        let mut remainder: u128 = 0;
        let mut quotient = Self::ZERO;

        for i in (0..2 * LIMB_BITS).rev() {
            let next_bit = self.bit(i);
            let carry = remainder >> (LIMB_BITS - 1);
            let shifted = (remainder << 1) | next_bit;

            let quotient_bit: u128;
            if carry == 1 {
                // True (129-bit) remainder is `shifted + 2^128`, which always exceeds
                // `d <= u128::MAX`, so the subtraction below is unconditional and the
                // wrapping add recovers the correct in-range result.
                remainder = shifted.wrapping_sub(d);
                quotient_bit = 1;
            } else if shifted >= d {
                remainder = shifted - d;
                quotient_bit = 1;
            } else {
                remainder = shifted;
                quotient_bit = 0;
            }

            let new_hi = (quotient.hi << 1) | (quotient.lo >> (LIMB_BITS - 1));
            let new_lo = (quotient.lo << 1) | quotient_bit;
            quotient = Self {
                hi: new_hi,
                lo: new_lo,
            };
        }

        if quotient.hi != 0 {
            None
        } else {
            Some((quotient.lo, remainder))
        }
    }
}
