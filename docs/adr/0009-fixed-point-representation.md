# ADR-0009 — WAD fixed point with 256-bit multiply-divide intermediates

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Aegis must perform financial arithmetic on-chain with no floating point (NFR-1). The representation
choice determines precision, overflow behavior, and how easy the code is to review.

## Decision

1. **Token amounts:** `u64`, native mint base units, never scaled.
2. **All fractions, rates, prices, and values:** `u128` at **WAD = 1e18**.
3. **Shares:** `u128`, dimensionless, with `VIRTUAL_SHARES = 1e6`, `VIRTUAL_ASSETS = 1`.
4. **Every economic multiply-divide** goes through `mul_div_floor` / `mul_div_ceil`, which compute
   `a·b` in a **256-bit intermediate**, divide, and check the result fits `u128`.
5. **Time:** `i64` unix seconds from `Clock`, never slots.

## Why 256-bit intermediates are mandatory, not defensive

This is not caution; it is required for correctness on legal inputs.

Supply shares can reach `assets × VIRTUAL_SHARES ≈ 1.8e19 × 1e6 = 1.8e25`. Converting shares back to
assets computes `shares × total_assets ≈ 1.8e25 × 1.8e19 = 3.2e44`, which overflows `u128`
(max ≈ 3.4e38) **even though the final result fits comfortably**.

An implementation using plain `u128` multiplication here is wrong and will abort on large-but-legal
states — most likely during a stress event, when the protocol can least afford it. `U-ARITH-04` exists
specifically to pin this case, and it is a Phase 1 acceptance criterion.

## Alternatives considered

**Q64.64 binary fixed point.** Rejected. Faster, but decimal-scaled values are far easier to review
against a written specification, and reviewability is worth more here than a few CU. Every worked
example in `economic-model.md` can be checked by hand.

**RAY (1e27) as a second scale**, Aave-style. Rejected. Two scales means constant conversion and a
class of "wrong scale" bugs. One scale, one mental model.

**`u64` for WAD parameters.** Rejected. Every such parameter feeds directly into `u128` `mul_div`;
storing them narrower forces a widening cast at every use site and invites a truncation bug on the
write path. Under Agave 4.2's ~90% rent reduction the extra bytes cost almost nothing.

**A big-decimal or arbitrary-precision library.** Rejected — unnecessary weight and CU for a fixed,
known value range.

## Consequences

**Positive**
- One scale everywhere; specification and code read the same.
- Legal extreme states cannot overflow.
- `mul_div_*` is a single, small, exhaustively-tested chokepoint for all economic arithmetic — the
  natural place to concentrate review effort.
- Explicit `floor`/`ceil` variants make the rounding law enforceable at every call site.

**Negative**
- 256-bit multiply-divide costs more CU than a native `u128` multiply. Measured in PERF-I2 and
  reported, but **not negotiable for CU**: correctness wins.
- `u128` fields are larger than strictly necessary.

**Enforcement**
- `overflow-checks = true` in the release profile — mandatory, CI-asserted. Release builds do **not**
  check overflow by default, and without this the entire arithmetic safety argument is void in the
  deployed artifact.
- `aegis-math` is `no_std` and float-free; `CI-NOFLOAT` greps for `f32`/`f64`.
- Sanity bounds `MIN_PRICE_WAD = 1e6`, `MAX_PRICE_WAD = 1e30` turn an absurd oracle value into a clean
  error rather than an arithmetic abort.
- 14 rounding directions, each with its own unit test.
