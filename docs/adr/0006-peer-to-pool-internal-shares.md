# ADR-0006 — Peer-to-pool lending with internal shares, not a share token

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

Two questions had to be answered before any code:

1. **Where does loan liquidity come from?**
2. **How is a lender's claim represented?**

## Decision

1. **Peer-to-pool.** Lenders supply the loan asset into the market's vault and earn
   utilization-driven interest.
2. **Internal scaled shares** stored in the lender's `Position`. **No SPL share mint in v1.**
3. **Share-based accounting for both supply and borrow**, with virtual offsets
   (`VIRTUAL_SHARES = 1e6`, `VIRTUAL_ASSETS = 1`).

## Alternatives considered

**Protocol-funded liquidity / treasury-supplied loans.** Rejected as economically incoherent. Liquidity
would appear from nowhere and the interest model would be decorative rather than load-bearing.

**Separate `supply_index` / `borrow_index` scalars** (the Aave pattern) instead of shares. Rejected:
index-based accounting drifts subtly and makes conservation properties harder to state. Share math
gives directly assertable invariants (`total_supply_shares == Σ position.supply_shares`) and forces
every rounding direction to be explicit at the call site — which is exactly where a reviewer wants it.

**Tokenized supply shares (an Aegis-issued SPL mint).** Deferred, not rejected on principle. It would
enable composability — shares usable as collateral elsewhere, secondary markets — but it adds a mint
authority, custody surface, and Token-2022 interaction surface for composability v1 does not need. A
clean v2 addition behind the same accounting.

**Peer-to-peer matching** (Morpho Optimizer style). Rejected: substantially more complex matching
machinery for capital-efficiency gains irrelevant at v1 scale.

## Consequences

**Positive**
- The economics close: liquidity has a source, lenders have a reason to supply, and the loss-bearer in
  a bad-debt event is named.
- Interest accrual is a pure transfer of claim from borrowers to lenders, so
  `total_supply_assets − total_borrow_assets` is invariant under accrual (INV-ACC-04) — which is what
  keeps the vault reconciliation true with no token movement.
- Protocol fees are minted as ordinary supply shares to the fee recipient's position, so there is **no
  fee vault, no privileged fee-withdrawal path, and no extra account in the hot path**. The fee
  recipient calls `withdraw` like any other lender.
- That same structure makes protocol **first-loss** natural: `absorb_bad_debt` burns the fee
  recipient's shares before socializing anything to lenders.

**Negative**
- **Virtual offsets are mandatory** and must never be removed. Without them an empty market is
  attackable by first-depositor share inflation (`economic-model.md` §3.2). `A-SHARE-01` runs the
  attack with offsets disabled (must succeed) and enabled (must be unprofitable).
- Later depositors receive marginally fewer shares than the first — a bounded, negligible asymmetry
  that is the price of the defense, and is asserted in `U-SHARE-02` so it is documented rather than
  discovered.
- Lenders bear socialized bad debt pro-rata with no opt-out. Honest and simple; tranching and
  insurance funds are named as production-grade extensions in `economic-model.md` §11.

**Related requirement**
Unsolicited tokens sent directly to a vault are **never** credited (INV-CUS-08). Aegis never treats a
vault balance as a source of truth — only as a delta measurement across its own CPI. This closes the
direct-donation inflation vector outright, leaving the virtual offsets to defend the interest-accrual
variant.
