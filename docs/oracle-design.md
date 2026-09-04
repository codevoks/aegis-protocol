# Aegis — Oracle Architecture

**Status: FROZEN (Phase 0). Research gates RV-3, RV-4 must be closed in Phase 5.**

> The oracle is the protocol's only trusted external input, and every solvency decision depends on it.
> Aegis treats price validation as a security boundary, not a data-fetching concern.

---

## 1. The abstraction

```rust
/// Conservative price band for one asset, normalized to WAD (1e18) USD per whole token.
pub struct PriceBand {
    pub lo: u128,          // confidence-adjusted lower bound  → value COLLATERAL with this
    pub hi: u128,          // confidence-adjusted upper bound  → value DEBT with this
    pub published_at: i64, // unix seconds
}

pub trait PriceSource {
    /// Returns a validated band, or an error. NEVER returns a "best effort" price.
    fn read_price(
        account: &AccountInfo,
        expected_feed_id: &[u8; 32],
        now: i64,
        max_age_secs: u32,
        max_conf_bps: u16,
    ) -> Result<PriceBand>;
}
```

Dispatch is by `market.oracle_kind`. v1 has exactly one implementer: `PythPull`. The trait exists
because a second implementer (Switchboard, or a redundant-median composite) is a realistic v2 and must
not require restructuring the health-check call sites — not because v1 needs polymorphism.

**There is no `Mock` or `Local` variant in the production program.** See §5.

---

## 2. Validation rules — all mandatory, all fail-closed

A price is used **only if every check passes**. Any failure returns an error; there is no degraded
mode, no last-known-good fallback, no "use it anyway if it's close".

| # | Check | Failure mode prevented |
|---|---|---|
| O-1 | Account owner == the Pyth receiver program ID | Fake price account owned by an attacker program |
| O-2 | Account deserializes as the expected price-update type with the correct discriminator | Type confusion / crafted bytes |
| O-3 | `feed_id == market.collateral_feed_id` (resp. `loan_feed_id`) | **Mismatched feed** — passing a cheap asset's feed for an expensive one |
| O-4 | `verification_level == Full` | Partially-verified Wormhole VAA accepted as authoritative |
| O-5 | `now − publish_time ≤ market.max_price_age_secs` | Stale price |
| O-6 | `publish_time ≤ now + MAX_FUTURE_SKEW` (60s) | Future-dated price from a clock/feed anomaly |
| O-7 | `price > 0` | Zero/negative price making collateral free or debt infinite |
| O-8 | `conf ≤ price · max_conf_bps / 10_000` | Wide confidence during volatility or partial outage |
| O-9 | `lo ≥ MIN_PRICE_WAD (1e6)` and `hi ≤ MAX_PRICE_WAD (1e30)` | Absurd values causing downstream overflow |
| O-10 | Scaling by `expo` is checked | Exponent-driven overflow/truncation |
| O-11 | The two price accounts are distinct accounts | Passing one feed for both assets |

O-3 deserves emphasis: **the account is not the identity — the feed ID is.** Pyth price updates are
permissionlessly-posted ephemeral accounts, so their addresses cannot be pinned. Pinning the address
would be wrong; pinning the feed ID is correct. Any design that stores an expected *pubkey* for a pull
oracle is broken, and this is a common mistake worth demonstrating that we avoided.

O-5 uses **unix seconds, never slots** (NFR-13). Under SIMD-0525 slot duration is being halved in
steps, so any slot-count staleness window silently changes meaning as the network upgrades.

### Recommended parameters

| Parameter | Value | Reasoning |
|---|---|---|
| `max_price_age_secs` (majors: SOL, BTC, ETH) | **30** | Pyth guidance: latency-sensitive protocols should use "a few seconds"; 30s balances liveness on a local/dev cluster against manipulation. |
| `max_price_age_secs` (stables) | **120** | Slower-moving; tolerating more age avoids spurious fail-closed. |
| `max_conf_bps` (majors) | **100** (1%) | Above ~1% relative confidence, Pyth is signalling genuine disagreement. |
| `max_conf_bps` (stables) | **50** (0.5%) | A stable with >0.5% uncertainty is depegging; halting is correct. |
| `MAX_FUTURE_SKEW` | 60s | Constant. |

---

## 3. Conservative valuation

From a validated `(price, conf, expo)`:

```
raw_lo = price.saturating_sub(conf)        // ≥ 1 enforced by O-9
raw_hi = price.checked_add(conf)?
lo = scale_to_wad_floor(raw_lo, expo)
hi = scale_to_wad_ceil (raw_hi, expo)
```

Applied per Pyth's own lending guidance:

- **Collateral is valued at `lo`** and rounded **down**.
- **Debt is valued at `hi`** and rounded **up**.

Both the bound and the rounding push in the protocol's favor, so every health decision is made against
the least-favorable admissible interpretation of the oracle. Consequences, stated honestly:

- Positions become liquidatable slightly *earlier* than a mid-price model would indicate. That is a
  cost borne by borrowers in exchange for lender solvency, and it is the correct trade for a risk-first
  protocol.
- It is **not** a substitute for O-8. Skewing to the band edge handles ordinary uncertainty; a
  genuinely wide confidence interval means the price is unreliable, and the answer to an unreliable
  price is to stop, not to skew harder. Both mechanisms are present and they do different jobs.

---

## 4. Failure policy — which operations need a price

| Operation | Oracle required | Behavior on oracle failure |
|---|---|---|
| `deposit_collateral` | **No** | Succeeds |
| `repay` | **No** | Succeeds |
| `supply` | **No** | Succeeds |
| `withdraw` (loan asset) | **No** | Succeeds |
| `absorb_bad_debt` | **No** | Succeeds |
| `close_position` | **No** | Succeeds |
| `withdraw_collateral` with zero debt | **No** | Succeeds |
| `withdraw_collateral` with debt | Yes | **Fail closed** |
| `borrow` | Yes | **Fail closed** |
| `liquidate` | Yes (both feeds) | **Fail closed** |

The organizing rule: **an operation needs a price if and only if it can make a position less safe.**
Everything that reduces risk, recognizes a loss, or returns unencumbered funds must remain available
during an outage. Lenders can still withdraw; borrowers can still repay and top up collateral; the
protocol can still recognize bad debt.

Only three operations are blocked, and all three are risk-*increasing* or price-dependent transfers of
value between parties.

### 4.1 Fail-closed on `liquidate` — the hard trade-off

Blocking liquidation during an outage lets positions decay past the point of recovery, creating bad
debt. Allowing liquidation with an unvalidated price lets an attacker with a stale or manipulated
price seize healthy positions at a discount. Aegis chooses **fail closed**, because:

1. Bad debt is bounded by the position's collateral shortfall and is **contained inside one market**
   (ADR-0004). Liquidation on a bad price is unbounded theft across *every* position in the market.
2. Bad debt is recognized honestly and socialized transparently (`absorb_bad_debt`, which needs no
   oracle). Stolen collateral is not recoverable.
3. An outage is temporary; the loss from a wrong-price liquidation is permanent.

**Residual risk, stated plainly:** a prolonged oracle outage during a large price move will produce
bad debt that Aegis cannot prevent. Mitigations are operational, not algorithmic — conservative
`max_price_age_secs`, guardian pause of `BORROW` when feeds degrade, and the Phase 13 runbook. A
production system would add a second oracle source and a fallback median; v1 does not, and says so
(economic-model §11).

---

## 5. Deterministic local prices — no mock program (ADR-0008)

The brief called for "a deterministic local oracle abstraction" plus "a real Pyth adapter." The
obvious implementation — a `Mock` oracle variant, or a small mock-oracle program the main program
trusts — was **rejected**.

**Why rejected:**

- A `Mock` variant means the deployed artifact contains a code path whose only purpose is to bypass
  price validation. That is a permanent, load-bearing security liability guarded by a config flag,
  and config flags get set wrong.
- A separate mock program still requires the production program to accept a second, weaker oracle kind
  and to trust a program ID from config.
- Either way, **the tests would exercise the mock path and not the real Pyth deserialization** — so
  the code that actually runs in production would be the least-tested code in the protocol. That is
  precisely backwards.

**What Aegis does instead:** the deterministic local oracle is achieved by **test-fixture account
injection**. LiteSVM and Surfpool can both set arbitrary accounts, so the test harness constructs a
byte-exact Pyth price-update account — correct owner, correct discriminator, chosen feed ID, chosen
price/conf/expo/publish_time — and hands it to the program.

```
crates/aegis-test-kit/src/pyth_fixture.rs   (TEST CODE ONLY — never a program dependency)
    fn price_account(feed_id, price: i64, conf: u64, expo: i32, publish_time: i64) -> (Pubkey, Account)
    fn set_price(svm: &mut LiteSVM, ...)          // move the price deterministically
    fn set_stale(svm: &mut LiteSVM, age_secs: i64)
    fn set_wide_confidence(svm: &mut LiteSVM, bps: u16)
    fn set_wrong_feed(svm: &mut LiteSVM, other_feed_id)
```

This is strictly better on every axis that matters:

| Property | Mock oracle | Fixture injection |
|---|---|---|
| Production code paths tested | The mock path, not Pyth's | **The real path, byte for byte** |
| Test-only code in the deployed program | Yes | **None** |
| Determinism | Yes | Yes |
| Zero-cost / offline | Yes | Yes — reads are account reads, **no CPI**, so the Pyth program need not even be deployed |
| Can simulate stale / wide-conf / wrong-feed / absurd values | Awkwardly | **Trivially and exactly** |

The `PriceSource` trait still exists for the v2 second-source reason in §1; it simply has one
implementer, and the local determinism requirement is met by the harness rather than by a code branch.

**Consequence for Phase 5:** the adapter must be written against the real crate types, and the fixture
builder must produce bytes that the real crate deserializes. If the fixture is wrong, the tests fail —
which is the point. Research gates RV-3 (upgraded receiver program ID) and RV-4 (`VerificationLevel`
shape) must be closed before writing either.

---

## 6. Threat model (oracle-specific)

| ID | Threat | Attacker | Entry point | Prerequisite | Impact | Mitigation | Test | Residual |
|---|---|---|---|---|---|---|---|---|
| O-T1 | Fake price account | Anyone | `borrow`/`liquidate` | None | Total drain | O-1, O-2 | `A-ORACLE-06` | None |
| O-T2 | Wrong feed for the asset | Anyone | any priced ix | None | Mispriced collateral → drain | O-3, O-11 | `A-ORACLE-07` | None |
| O-T3 | Stale price after a crash | Liquidator | `liquidate` | Price moved | Liquidate healthy positions | O-5 | `A-ORACLE-03` | Bounded by `max_price_age_secs` |
| O-T4 | Wide confidence during volatility | Liquidator | `liquidate` | Market stress | Unfair liquidation | O-8 + `lo`/`hi` skew | `A-ORACLE-05` | Fail-closed halts liquidation → bad-debt risk |
| O-T5 | Partially-verified update | Anyone | any priced ix | Post a partial VAA | Unverified price accepted | O-4 | `A-ORACLE-08` | None |
| O-T6 | Future-dated publish time | Anyone | any priced ix | Feed anomaly | Staleness check bypassed indefinitely | O-6 | `A-ORACLE-09` | None |
| O-T7 | Oracle downtime | Environment | all priced ix | Outage | Liquidations blocked → bad debt | Fail closed + guardian pause + runbook | `A-ORACLE-10` | **Accepted, documented** |
| O-T8 | Underlying market manipulation | Well-capitalized | Pyth itself | Move the real market | Wrong-but-valid price | Out of Aegis's control; `max_conf_bps` catches publisher disagreement | — | **Accepted** — inherent to any oracle-based protocol |
| O-T9 | Same-block price exploitation | Sophisticated | `borrow`+`liquidate` in one tx | Latency edge | Extract via price timing | Overcollateralization buffer (`max_ltv` < `LT`) absorbs it at v1 scale | `A-ORACLE-11` | Accepted for v1; Pyth's guidance (commit/execute separation) is a v2 item |
| O-T10 | Exponent overflow via absurd `expo` | Anyone | any priced ix | Crafted/anomalous feed | Panic or wrong value | O-9, O-10 | `A-ORACLE-04` | None |

---

## 7. Testing requirements

Every row of §2 and §6 is a test. Specifically, Phase 5 is not complete until all of these pass:

- **Happy path:** valid prices → correct `PriceBand`, correct HF, borrow succeeds.
- **Each of O-1 … O-11 individually violated** → the specific error is returned, and *no state
  changed*. (Asserting no state change matters: a check that reverts after a partial write is a bug.)
- **Boundary tests:** `age == max_price_age_secs` (pass) vs `+1` (fail); `conf` exactly at
  `max_conf_bps` (pass) vs `+1` (fail); `HF == WAD` (not liquidatable) vs `WAD − 1` (liquidatable).
- **Risk-reducing operations succeed with a maximally broken oracle** (`A-ORACLE-01/02`): stale, wide,
  wrong-feed, and absent price accounts must not prevent `repay`, `deposit_collateral`,
  `absorb_bad_debt`, or debt-free `withdraw_collateral`. This is a *positive* test of a safety
  property and is easy to regress.
- **Decimals matrix:** valuation correct for `(collateral_decimals, loan_decimals)` across `0..=12`
  and for `expo ∈ {−12..0}` (`P-VAL-1`).
- **Deterministic price-path scenario:** a scripted price trajectory driving a position from healthy →
  liquidatable → liquidated → bad debt, with every invariant asserted at each step. This is the
  centrepiece demo (Phase 13).

---

## 8. Off-chain concerns (explicitly outside the zero-cost path)

Posting Pyth price updates on a live cluster requires fetching signed updates from Hermes. As of the
research date the keyless `hermes.pyth.network` endpoint requires an API key and the recommended
endpoint is `pyth.dourolabs.app/hermes`.

**This has no bearing on the core requirement**, because:

- Reading a price update is an **account read, not a CPI** — the local tests inject the account
  directly and never contact Hermes or deploy the Pyth program.
- Hermes is needed only by the optional devnet/mainnet-fork tier (Phase 8's network-tagged tests and
  Phase 9's optional live demo).

The SDK therefore isolates all Hermes interaction behind one module with an injectable endpoint and a
fixture-backed offline implementation, so the app and the liquidator bot can run fully locally.
