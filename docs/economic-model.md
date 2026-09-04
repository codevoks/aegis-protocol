# Aegis — Economic Model and Formula Specification

**Status: FROZEN (Phase 0). Any change requires an ADR and a re-run of the property-test suite.**

This document is normative. Implementation phases must match it exactly, including rounding
directions. Where this document and code disagree, **this document wins** until an ADR changes it.

---

## 1. Numeric foundations

### 1.1 Representations

| Concept | Type | Scale | Notes |
|---|---|---|---|
| Token amount | `u64` | native mint base units | Never scaled. What the token program actually moves. |
| Fraction / ratio / rate | `u128` | **WAD = 1e18** | LTV, thresholds, bonus, fees, utilization, health factor. |
| Price | `u128` | **WAD = 1e18**, quoted in USD per *whole token* | Normalized from the oracle. See §6. |
| Value | `u128` | **WAD = 1e18**, USD | Result of amount × price ÷ 10^decimals. |
| Shares (supply & borrow) | `u128` | dimensionless | Internal accounting only. |
| Time | `i64` | unix **seconds** | From `Clock::unix_timestamp`. **Never slots** (NFR-13). |

```
WAD            = 1_000_000_000_000_000_000            // 1e18
VIRTUAL_SHARES = 1_000_000                            // 1e6
VIRTUAL_ASSETS = 1
SECONDS_PER_YEAR = 31_536_000                         // 365 days, fixed, non-leap
```

**NFR-1 restated:** no `f32`/`f64` may appear in the on-chain crate. This is enforced by a CI grep and
by `aegis-math` being `#![no_std]`-compatible and float-free.

### 1.2 The one arithmetic primitive

Every economic multiply-divide in Aegis goes through exactly two functions, in `aegis-math`:

```
mul_div_floor(a: u128, b: u128, d: u128) -> Result<u128>   // ⌊a·b / d⌋
mul_div_ceil (a: u128, b: u128, d: u128) -> Result<u128>   // ⌈a·b / d⌉
```

Both compute `a·b` in a **256-bit intermediate**, then divide, then check that the result fits `u128`.
`d == 0` is an error, never a panic-by-division.

**This is not optional.** Worked justification: supply shares can reach
`assets × VIRTUAL_SHARES ≈ 1.8e19 × 1e6 = 1.8e25`. Converting shares back to assets computes
`shares × total_assets ≈ 1.8e25 × 1.8e19 = 3.2e44`, which overflows `u128` (max ≈ 3.4e38) even though
the *result* fits comfortably. Any implementation using plain `u128` multiplication here is wrong and
will abort on large-but-legal states. A property test (`P-ARITH-3`) must exercise exactly this.

`mul_div_ceil` is implemented as `⌊(a·b + d − 1)/d⌋` in 256-bit space (no overflow risk at 256 bits).

### 1.3 Rounding law

> **Rounding always favors the protocol, never the user. Where "the protocol" is ambiguous, rounding
> favors the pool's existing participants over the acting user.**

Applied consistently:

| Operation | Quantity computed | Direction | Reason |
|---|---|---|---|
| `supply(assets)` | shares minted | **floor** | User receives fewer shares. |
| `withdraw(assets)` | shares burned | **ceil** | User burns more shares. |
| `borrow(assets)` | borrow shares minted | **ceil** | User owes more. |
| `repay(assets)` | borrow shares burned | **floor** | User is credited less. |
| `supply(shares)` | assets required | **ceil** | User pays more. |
| `withdraw(shares)` | assets returned | **floor** | User receives less. |
| `borrow(shares)` | assets returned | **floor** | User receives less. |
| `repay(shares)` | assets required | **ceil** | User pays more. |
| Interest accrual | interest added | **floor** | Never over-accrue debt beyond the exact figure. |
| Protocol fee shares | fee shares | **floor** | Never dilute lenders more than exactly owed. |
| Collateral value | value | **floor** | Understate collateral. |
| Debt value | value | **ceil** | Overstate debt. |
| Liquidation seize amount | collateral seized | **floor** | Liquidator receives no more than earned. |
| Liquidation repay (when collateral-capped) | repay required | **ceil** | Liquidator pays no less than owed. |
| Liquidation protocol fee | fee taken from bonus | **floor** | Never over-take from the liquidator. |

Every row above is a distinct unit test (`U-ROUND-01..14`).

---

## 2. Market state

A `Market` holds four accounting scalars plus timing:

```
total_supply_assets : u64    // loan-asset base units owed to lenders (principal + accrued interest)
total_supply_shares : u128
total_borrow_assets : u64    // loan-asset base units owed by borrowers
total_borrow_shares : u128
last_accrual_ts     : i64
collateral_fee_accrued : u64 // collateral-asset base units owned by the protocol
```

Collateral is **not** a market scalar. It lives per-position, because collateral is escrowed
individually and never pooled (ADR-0005). This is what allows collateral deposit/withdraw to avoid
writing `Market` (NFR-7).

**Physical/logical reconciliation (INV-CUS-01, INV-CUS-02):**

```
loan_vault.amount        ==  total_supply_assets − total_borrow_assets      (exactly)
collateral_vault.amount  ==  Σ position.collateral_amount + collateral_fee_accrued
```

The first identity is why `total_supply_assets ≥ total_borrow_assets` must hold at all times
(INV-ACC-03) — it is the definition of available liquidity, not a derived nicety.

---

## 3. Share accounting

### 3.1 Conversions

With virtual offsets applied to both sides:

```
to_shares_down(assets, total_assets, total_shares) =
    mul_div_floor(assets, total_shares + VIRTUAL_SHARES, total_assets + VIRTUAL_ASSETS)

to_shares_up(assets, total_assets, total_shares) =
    mul_div_ceil (assets, total_shares + VIRTUAL_SHARES, total_assets + VIRTUAL_ASSETS)

to_assets_down(shares, total_assets, total_shares) =
    mul_div_floor(shares, total_assets + VIRTUAL_ASSETS, total_shares + VIRTUAL_SHARES)

to_assets_up(shares, total_assets, total_shares) =
    mul_div_ceil (shares, total_assets + VIRTUAL_ASSETS, total_shares + VIRTUAL_SHARES)
```

### 3.2 Why virtual shares exist (do not remove them)

Without virtual offsets, an empty market is attackable:

1. Attacker supplies 1 base unit → receives 1 share. `total_assets = 1`, `total_shares = 1`.
2. Attacker donates 10^9 base units directly to the vault, or (more realistically here) waits for
   interest to accrue on a tiny borrow, inflating `total_assets` to 10^9 + 1 with `total_shares` still 1.
3. Victim supplies 1.5×10^9 → `shares = ⌊1.5e9 × 1 / (1e9+1)⌋ = 1` share, and the pool now has 2 shares
   for 2.5e9 assets. Attacker redeems 1 share for 1.25e9 — stealing 0.25e9 from the victim.

With `VIRTUAL_SHARES = 1e6`, the attacker's share price can only be inflated by a factor bounded by
their own donation relative to `1e6` virtual shares, making the attack cost exceed the profit by ~6
orders of magnitude. Aegis additionally never credits unsolicited vault donations (§9.3), which closes
the direct-donation vector outright; the virtual offsets defend the interest-accrual variant and the
general first-depositor rounding edge.

**Test:** `A-SHARE-01` reproduces the attack with `VIRTUAL_SHARES = 0` (must steal) and with the real
constant (must be unprofitable).

### 3.3 Worked example

Empty market. Alice supplies 1,000,000 base units (1 USDC at 6 decimals is 1e6; assume 1,000 USDC = 1e9 —
use 1e9 here).

```
assets = 1e9, total_assets = 0, total_shares = 0
shares = ⌊1e9 × (0 + 1e6) / (0 + 1)⌋ = 1e15
→ total_supply_assets = 1e9, total_supply_shares = 1e15
```

Bob supplies 1e9 immediately after (no interest yet):

```
shares = ⌊1e9 × (1e15 + 1e6) / (1e9 + 1)⌋
       = ⌊1e9 × 1000000000001000000 / 1000000001⌋
       ≈ 999999999_000000000  ≈ 9.99999999e17 ... 
```
Recompute carefully:
`1e9 × (1e15 + 1e6) = 1e24 + 1e15`. Divided by `1e9 + 1`:
`(1e24 + 1e15) / (1e9 + 1) ≈ 1e15 − 1e6 + …` → Bob receives ≈ `999_999_999_000_000` ≈ 1e15 minus ~1e6.

Bob receives marginally fewer shares than Alice — the virtual-offset "tax" on later depositors, which
is bounded by `VIRTUAL_ASSETS/VIRTUAL_SHARES` and is negligible (≈1 part in 1e9 here) while making the
inflation attack uneconomic. This asymmetry is intentional and is asserted in `U-SHARE-02`.

---

## 4. Interest accrual

### 4.1 Interest rate model (stateless, piecewise-linear with a kink)

Market parameters, all **per-second WAD rates**:

```
base_rate_ps, slope1_ps, slope2_ps : u128   // per-second, WAD
u_kink : u128                                // WAD, 0 < u_kink < WAD
max_rate_ps : u128                           // per-second WAD hard cap
```

Utilization:

```
u = if total_supply_assets == 0 { 0 }
    else { min(WAD, mul_div_floor(total_borrow_assets, WAD, total_supply_assets)) }
```

Borrow rate:

```
r = if u <= u_kink:
        base_rate_ps + mul_div_floor(slope1_ps, u, u_kink)
    else:
        base_rate_ps + slope1_ps
          + mul_div_floor(slope2_ps, u − u_kink, WAD − u_kink)

r = min(r, max_rate_ps)
```

Rationale for statelessness: the rate is a pure function of `(u, params)`. There is no IRM state
account, no IRM accrual, and no cross-market coupling. It is fully deterministic and exhaustively
testable, and an adaptive curve can later replace it behind the same signature (ADR-0007).

**Reference parameter set** (documented default; each market sets its own):

| Param | Annualized | Per-second WAD |
|---|---|---|
| `base_rate` | 0% | 0 |
| `slope1` | 4% at kink | `0.04 × WAD / SECONDS_PER_YEAR` = 1_268_391_679 |
| `slope2` | +100% above kink | `1.00 × WAD / SECONDS_PER_YEAR` = 31_709_791_983 |
| `u_kink` | 80% | `0.8 × WAD` |
| `max_rate` | 1000% | `10.0 × WAD / SECONDS_PER_YEAR` = 317_097_919_837 |

At `u = 0.9`: `r = 0 + slope1 + slope2 × (0.9−0.8)/(1−0.8) = slope1 + 0.5·slope2` ≈ 4% + 50% = **54% APR**.

### 4.2 Compounding

Let `dt = clamp(now − last_accrual_ts, 0, ..)` seconds. If `dt == 0`, accrual is a no-op.

```
x      = r · dt                                    (WAD)
growth = taylor3(x) = x + x²/(2·WAD) + x³/(6·WAD²)   (WAD)
interest = mul_div_floor(total_borrow_assets, growth, WAD)
```

`taylor3` is the third-order Taylor expansion of `e^x − 1`. It is used because:

- It compounds per second without a loop and without `exp()` (which is unavailable and would be
  floating-point).
- It **under-approximates** `e^x − 1` for `x > 0`, so it can never over-charge borrowers. That
  direction is deliberate and asserted (`P-IRM-2`).

Bound on the approximation error: for `x ≤ 0.1` (a full day at ~3650% APR), the relative error vs
`e^x − 1` is < 0.05%. Because accrual is invoked on essentially every interaction, `x` is normally
`≤ 1e-3` and the error is negligible. If `dt` is genuinely large (a dormant market), the error remains
a *discount* to borrowers, never a charge. `max_rate_ps` bounds `x` further.

Then:

```
total_borrow_assets += interest
total_supply_assets += interest        // lenders' claim grows identically
last_accrual_ts      = now
```

Note both use the same `interest` — interest is a pure transfer of claim from borrowers to lenders,
so `total_supply_assets − total_borrow_assets` (i.e. free liquidity) is invariant under accrual
(**INV-ACC-04**). That identity is what keeps the vault reconciliation of §2 true across accrual with
no token movement at all.

### 4.3 Protocol fee

The market takes `fee` (WAD, capped at `MAX_FEE = 0.25 × WAD`) of accrued interest, minted as supply
shares to the fee recipient's position — so protocol fees are ordinary supply shares and require no
separate vault, no separate withdrawal path, and no extra account in the hot path.

```
fee_amount = mul_div_floor(interest, fee, WAD)

// shares priced AFTER interest is added but EXCLUDING the fee itself, so the fee
// dilutes existing lenders by exactly fee_amount of value and no more:
fee_shares = to_shares_down(
    fee_amount,
    total_supply_assets − fee_amount,
    total_supply_shares
)

total_supply_shares += fee_shares
fee_position.supply_shares += fee_shares
```

The `total_supply_assets − fee_amount` denominator is the subtle part: pricing the fee shares at the
pre-fee asset base is what makes the dilution exactly `fee_amount`. Pricing at the post-fee base would
under-issue and silently give lenders part of the fee. Asserted by `P-FEE-1`:
*after accrual, the fee recipient's claimable assets increase by exactly `fee_amount` (±1 base unit
rounding), and every other lender's claimable assets increase by exactly `interest − fee_amount` in
aggregate.*

### 4.4 Worked accrual example

Market: `total_supply_assets = 1_000e6`, `total_borrow_assets = 900e6` (USDC, 6 dp), `fee = 0.10 WAD`.

```
u = 900e6 × WAD / 1000e6 = 0.9 WAD
r = slope1 + slope2 × 0.5 = 1_268_391_679 + 15_854_895_991 = 17_123_287_670  (per-second WAD ≈ 54% APR)
dt = 86_400 (one day)
x  = 17_123_287_670 × 86_400 = 1_479_452_054_688_000  ≈ 0.00147945 WAD
growth = x + x²/2WAD + x³/6WAD²
       ≈ 1_479_452_054_688_000 + 1_094_469_... ≈ 1_480_546_...e0  ≈ 0.001480 WAD
interest = ⌊900e6 × 0.001480 WAD / WAD⌋ = ⌊1_332_492⌋ = 1_332_492 base units ≈ 1.332 USDC
total_borrow_assets = 901_332_492
total_supply_assets = 1_001_332_492
fee_amount = ⌊1_332_492 × 0.10⌋ = 133_249
fee_shares = to_shares_down(133_249, 1_001_332_492 − 133_249, total_supply_shares)
```

Sanity: 54% APR on 900 USDC for one day ≈ `900 × 0.54 / 365 = 1.331` USDC. Matches. ✅

`U-IRM-03` asserts this exact vector.

### 4.5 View accrual vs. mutating accrual

Two entry points with an enforced equality:

- `accrue_view(market, now) -> AccruedTotals` — pure, returns updated totals, **writes nothing**.
- `accrue_mut(market, now)` — calls `accrue_view` and persists, including fee shares.

`accrue_view` exists so that `withdraw_collateral` can evaluate solvency against fully-accrued debt
**without taking a write lock on `Market`** (NFR-7, ADR-0004). This is safe because accrual is a pure,
idempotent function of `(state, now)`, and because using accrued (larger) debt is strictly
conservative for a solvency check.

**INV-ACC-08 (mandatory test `P-ACCRUE-1`):** for any state and any `now`,
`accrue_view(s, now).totals == { accrue_mut(s', now); s'.totals }`. The only permitted divergence is
that `accrue_mut` also mints fee shares — which do not affect `total_supply_assets` or
`total_borrow_assets`, only `total_supply_shares`. Because health checks never read
`total_supply_shares`, the view path is exact for its purpose. This carve-out must be stated in code
comments, because it is the kind of thing a later refactor silently breaks.

---

## 5. Risk parameters

Per market, all WAD:

| Param | Meaning | Bounds enforced on-chain |
|---|---|---|
| `max_ltv` | Max LTV after borrow / collateral withdrawal | `0 < max_ltv < liq_threshold` |
| `liq_threshold` (LT) | LTV above which the position is liquidatable | `max_ltv < liq_threshold < WAD` |
| `liq_bonus` (b) | Liquidator's discount on seized collateral | `0 ≤ b ≤ MAX_LIQ_BONUS (0.25 WAD)` **and** `mul_div_floor(liq_threshold, WAD + b, WAD) < WAD` |
| `close_factor` | Max fraction of debt repayable in one partial liquidation | `0.05 WAD ≤ cf ≤ WAD` |
| `full_liq_hf` | HF below which 100% liquidation is allowed | `0 < full_liq_hf ≤ WAD` |
| `liq_protocol_fee` | Protocol's cut **of the bonus only** | `0 ≤ f ≤ 0.5 WAD` |
| `fee` | Protocol's cut of interest | `0 ≤ fee ≤ 0.25 WAD` |
| `min_debt` | Dust floor: position debt is 0 or ≥ this | `> 0` |

### 5.1 The bonus/threshold constraint is derived, not assumed

Let `c` = collateral value, `d` = debt value, `HF = c·LT/d`. A liquidation repaying debt value `δ`
seizes collateral value `δ(1+b)`.

```
HF' = (c − δ(1+b))·LT / (d − δ)
```

`HF' > HF` ⟺ `d(c − δ(1+b)) > c(d − δ)` ⟺ `−dδ(1+b) > −cδ` ⟺ `(1+b) < c/d = HF/LT`.

**Therefore liquidation improves health if and only if `HF > LT·(1+b)`.**

Two consequences, both designed for rather than discovered later:

1. **Config constraint.** Liquidation triggers at `HF < WAD`. For liquidation to be *improving* at the
   moment it first becomes possible, we need `LT·(1+b) < WAD`. This is exactly the bound enforced
   above. With `LT = 0.80`, the maximum admissible bonus is `b < 0.25`.
2. **The "death spiral" band is real and must be handled, not ignored.** For
   `HF ∈ (0, LT·(1+b))`, partial liquidation *reduces* HF further. Aegis therefore sets
   `full_liq_hf ≥ LT·(1+b)` by convention so that any position in that band is eligible for **full**
   liquidation in one transaction, rather than being repeatedly partially liquidated into insolvency.
   This is enforced as a **recommended** bound and checked in tests (`P-LIQ-4`); it is not a hard
   on-chain constraint because `full_liq_hf` interacts with `min_debt`, and an admin may legitimately
   set it higher.

**Reference parameter set** (SOL collateral / USDC loan): `max_ltv = 0.75`, `LT = 0.80`, `b = 0.05`,
`close_factor = 0.5`, `full_liq_hf = 0.95`, `liq_protocol_fee = 0.10`, `fee = 0.10`,
`min_debt = 10e6` (10 USDC).

Check: `LT·(1+b) = 0.80 × 1.05 = 0.84 < 1` ✅, and `full_liq_hf = 0.95 ≥ 0.84` ✅.

At `HF = 1`, `c = d/LT = 1.25d` — a 25% buffer against a 5% bonus. Comfortable. ✅

---

## 6. Prices and valuation

### 6.1 Normalization

The oracle yields, per asset, a conservative pair:

```
price_lo : u128   // WAD, USD per whole token, confidence-adjusted lower bound
price_hi : u128   // WAD, USD per whole token, confidence-adjusted upper bound
```

From a Pyth `(price: i64, conf: u64, expo: i32)`:

```
require price > 0
require conf ≤ mul_div_floor(price, max_conf_bps, 10_000)      // else FAIL CLOSED

raw_lo = price − conf        (saturating at 1)
raw_hi = price + conf        (checked)

// scale to WAD:  value_wad = raw × 10^(18 + expo)
price_lo = scale_to_wad(raw_lo, expo)
price_hi = scale_to_wad(raw_hi, expo)
```

`scale_to_wad(raw, expo)`:
- if `18 + expo ≥ 0`: `raw × 10^(18+expo)` (checked; typical `expo = −8` → ×1e10)
- else: `raw / 10^(−18−expo)` (floor for `lo`, ceil for `hi`)

Sanity bounds (**reject outside**): `MIN_PRICE_WAD = 1e6` (1e-12 USD) and `MAX_PRICE_WAD = 1e30`
(1e12 USD). This bounds every downstream product and turns an absurd oracle value into a clean error
instead of an overflow abort.

### 6.2 Valuation

```
collateral_value = mul_div_floor(collateral_amount, price_c_lo, 10^collateral_decimals)   // floor, lower bound
debt_value       = mul_div_ceil (debt_assets,       price_l_hi, 10^loan_decimals)         // ceil,  upper bound
```

Both directions are conservative twice over: conservative *bound* from confidence, conservative
*rounding*. Pyth's own guidance is exactly this ("lower bound for collateral, upper bound for
liabilities"), and Aegis follows it verbatim.

`collateral_decimals` and `loan_decimals` are **cached in the `Market` account at creation**, read from
the mints. They are never re-read in the hot path, and mint decimals are immutable in both token
programs — so the cache cannot go stale. Caching removes two account reads from every priced
instruction.

### 6.3 Health factor

```
HF = if debt_value == 0 { u128::MAX }
     else { mul_div_floor(collateral_value, liq_threshold, debt_value) }     // WAD
```

- **Liquidatable** ⟺ `HF < WAD`.
- **Borrow / withdraw-collateral allowed** ⟺ `debt_value × WAD ≤ collateral_value × max_ltv`,
  evaluated as `debt_value ≤ mul_div_floor(collateral_value, max_ltv, WAD)`.

Note the borrow check is expressed against `max_ltv` directly rather than as a second health factor,
to avoid a second division and its rounding ambiguity.

### 6.4 Liquidation price (SDK read model, not on-chain)

The collateral price at which a position becomes liquidatable:

```
price_c_liquidation = debt_value × 10^collateral_decimals / (collateral_amount × liq_threshold)
```

Computed off-chain by the SDK for UI. Never on-chain (no consumer needs it, and it would burn CU).

### 6.5 Worked valuation example

SOL collateral (9 dp), USDC loan (6 dp). SOL = $150.00 ± $0.30, USDC = $1.0000 ± $0.0002.
Position: 10 SOL collateral, 900 USDC debt. `LT = 0.80`, `max_ltv = 0.75`.

```
price_c_lo = (150.00 − 0.30) WAD = 149.70e18
price_l_hi = (1.0000 + 0.0002) WAD = 1.0002e18

collateral_value = ⌊10e9 × 149.70e18 / 1e9⌋ = 1497.0e18   ($1,497.00)
debt_value       = ⌈900e6 × 1.0002e18 / 1e6⌉ = 900.18e18   ($900.18)

HF = ⌊1497.0e18 × 0.80e18 / 900.18e18⌋ = ⌊1.33083…e18⌋ = 1.330838…  → HF ≈ 1.3308 ✅ healthy
max borrow check: debt_value ≤ ⌊1497.0e18 × 0.75⌋ = 1122.75e18 ✅ (900.18 ≤ 1122.75)
Additional borrowing capacity ≈ $222.57 of debt value.
```

Now SOL falls to $95.00 ± $0.20:

```
collateral_value = 10 × 94.80 = 948.00e18
HF = ⌊948.00e18 × 0.80e18 / 900.18e18⌋ = 0.842495…e18  → HF ≈ 0.8425 < 1 → LIQUIDATABLE
```

`0.8425 < full_liq_hf (0.95)` → **full liquidation permitted** (also note `0.8425 ≈ LT(1+b) = 0.84`, i.e.
this position is right at the edge of the improving band — precisely why `full_liq_hf` is set to 0.95).

`U-HEALTH-01/02` assert both vectors exactly.

---

## 7. Liquidation

### 7.1 Maximum repayable

```
debt_assets = to_assets_up(position.borrow_shares, total_borrow_assets, total_borrow_shares)

max_repay =
    if HF < full_liq_hf                       { debt_assets }              // full
    else                                      { mul_div_floor(debt_assets, close_factor, WAD) }

// dust rule: never leave a position with 0 < remaining_debt < min_debt
if debt_assets − max_repay > 0 && debt_assets − max_repay < min_debt {
    max_repay = debt_assets
}

require repay_assets ≤ max_repay
require repay_assets > 0
```

### 7.2 Seizure

```
repay_value  = mul_div_ceil(repay_assets, price_l_hi, 10^loan_decimals)      // ceil: liquidator's credit valued conservatively low for them

base_seize   = mul_div_floor(repay_value, 10^collateral_decimals, price_c_lo)
total_seize  = mul_div_floor(mul_div_floor(repay_value, WAD + liq_bonus, WAD),
                             10^collateral_decimals, price_c_lo)
bonus_amount = total_seize − base_seize
```

**Collateral cap.** If `total_seize > position.collateral_amount`, the position cannot pay the full
bonus. Clamp and recompute the repayment downward so the liquidator never receives unearned collateral:

```
if total_seize > collateral_amount {
    total_seize  = collateral_amount
    seize_value  = mul_div_ceil(total_seize, price_c_lo, 10^collateral_decimals)
    repay_value' = mul_div_ceil(seize_value, WAD, WAD + liq_bonus)
    repay_assets = mul_div_ceil(repay_value', 10^loan_decimals, price_l_hi)   // ceil: liquidator pays more
    repay_assets = min(repay_assets, debt_assets)
    base_seize   = mul_div_floor(mul_div_ceil(repay_assets, price_l_hi, 10^loan_decimals),
                                 10^collateral_decimals, price_c_lo)
    bonus_amount = total_seize.saturating_sub(base_seize)
}
```

### 7.3 Protocol cut and settlement

```
protocol_cut  = mul_div_floor(bonus_amount, liq_protocol_fee, WAD)      // from the BONUS only
to_liquidator = total_seize − protocol_cut

repay_shares  = to_shares_down(repay_assets, total_borrow_assets, total_borrow_shares)
repay_shares  = min(repay_shares, position.borrow_shares)

position.borrow_shares      −= repay_shares
position.collateral_amount  −= total_seize
total_borrow_shares         −= repay_shares
total_borrow_assets         −= repay_assets
total_supply_assets         −= 0                       // unchanged: lenders are repaid, not impaired
collateral_fee_accrued      += protocol_cut

// token movement
transfer loan_asset:       liquidator → loan_vault        (repay_assets)
transfer collateral_asset: collateral_vault → liquidator  (to_liquidator)
```

`protocol_cut` never leaves the collateral vault; it is credited to `collateral_fee_accrued` and
withdrawn later by the admin. This keeps `liquidate`'s account list short and avoids write contention
on a shared fee account (see performance strategy).

### 7.4 Post-conditions (asserted on-chain where cheap, in tests always)

- `position.collateral_amount ≥ 0` and `total_seize ≤ collateral_before`.
- `repay_assets ≤ debt_assets`.
- If `position.borrow_shares > 0` after: remaining debt ≥ `min_debt`, **or** collateral is now 0.
- If `HF_before > LT·(1+b)`: `HF_after > HF_before` (**P-LIQ-1**, the derived improvement property).
- `collateral_vault.amount` decreased by exactly `to_liquidator`.
- `loan_vault.amount` increased by exactly `repay_assets` (measured; see Token-2022 delta accounting).

### 7.5 Worked liquidation

Continuing §6.5 at SOL = $95.00 ± $0.20 (`HF ≈ 0.8425`, full liquidation permitted).
`price_c_lo = 94.80e18`, `price_l_hi = 1.0002e18`, `b = 0.05`, `liq_protocol_fee = 0.10`.
Debt = 900e6 (900 USDC), collateral = 10e9 (10 SOL).

Liquidator repays the full 900 USDC:

```
repay_value  = ⌈900e6 × 1.0002e18 / 1e6⌉ = 900.18e18                       ($900.18)
base_seize   = ⌊900.18e18 × 1e9 / 94.80e18⌋ = 9_495_569_620 lamports        (9.49557 SOL)
bonus_value  = 900.18 × 1.05 = 945.189e18
total_seize  = ⌊945.189e18 × 1e9 / 94.80e18⌋ = 9_970_348_101                (9.97035 SOL)
bonus_amount = 9_970_348_101 − 9_495_569_620 = 474_778_481                  (0.47478 SOL)
protocol_cut = ⌊474_778_481 × 0.10⌋ = 47_477_848                            (0.04748 SOL)
to_liquidator = 9_970_348_101 − 47_477_848 = 9_922_870_253                  (9.92287 SOL)
```

Check: `9.97035 ≤ 10` ✅ — collateral suffices, no clamp.
Liquidator pays $900.18 of value, receives 9.92287 SOL ≈ $940.73 at $94.80 → **~4.5% net profit**
after the protocol's cut. The incentive is real and positive. ✅
Position afterwards: debt 0, collateral `10 − 9.97035 = 0.02965 SOL` returned to the borrower's
position. No bad debt. ✅

`U-LIQ-01` asserts these figures exactly.

---

## 8. Bad debt and insolvency

### 8.1 How bad debt arises

Enumerated honestly:

1. **Gap risk.** Price falls faster than liquidators can act — a single block can move a position from
   `HF = 1.2` to `HF < LT(1+b)`.
2. **Liquidation unprofitability.** Seized collateral cannot be sold for the repaid value (thin
   liquidity, high slippage), so no rational liquidator acts even though the position is liquidatable.
3. **Oracle unavailability.** Aegis fails closed, so liquidations are blocked during an outage; the
   position may be deeply underwater when the oracle returns.
4. **Dust.** Positions too small for gas-and-slippage-adjusted profit. Mitigated by `min_debt`, not
   eliminated.
5. **Frozen collateral.** A mint with a freeze authority freezes the vault or the liquidator's ATA,
   blocking seizure. Mitigated by policy (`ack_freeze_authority`), not eliminated.

### 8.2 `absorb_bad_debt`

Permissionless. Preconditions (after `accrue_mut`):

```
require position.collateral_amount == 0
require position.borrow_shares > 0
```

Requiring collateral to be *exactly zero* is deliberate: it means every liquidator has already had the
chance to extract every unit of value, so the residual is genuinely unrecoverable. There is no
discretion and no oracle dependency — `absorb_bad_debt` needs **no price at all**, so it works during
an oracle outage. That is intentional: loss recognition must never be blocked by the thing that caused
the loss.

Settlement, in strict order:

```
bad_assets = to_assets_up(position.borrow_shares, total_borrow_assets, total_borrow_shares)

// 1. Protocol first-loss: burn the fee recipient's supply shares up to the loss.
fee_assets  = to_assets_down(fee_position.supply_shares, total_supply_assets, total_supply_shares)
absorbed    = min(bad_assets, fee_assets)
burn_shares = to_shares_up(absorbed, total_supply_assets, total_supply_shares)
burn_shares = min(burn_shares, fee_position.supply_shares)
fee_position.supply_shares -= burn_shares
total_supply_shares        -= burn_shares

// 2. Socialize the remainder across the market's lenders.
total_supply_assets = total_supply_assets.saturating_sub(bad_assets)
total_borrow_assets = total_borrow_assets.saturating_sub(bad_assets)
total_borrow_shares -= position.borrow_shares
position.borrow_shares = 0
```

Both totals fall by `bad_assets`, so `total_supply_assets − total_borrow_assets` — free liquidity —
is unchanged, preserving the vault reconciliation of §2 with **no token movement**. Supply *shares*
are unchanged in step 2, so each remaining share is now worth less: the loss is socialized pro-rata.
Step 1 burning the fee recipient's shares first means the protocol's accumulated fees absorb the hit
before any lender does.

`fee_position` is a **required** account, constrained to `PDA(market, market.fee_recipient)`, and is
initialized during `create_market` so it always exists. Making it optional would let a caller skip
protocol first-loss and push more loss onto lenders — a real griefing vector, closed by construction.

**INV-SOLV-04:** after `absorb_bad_debt`, `loan_vault.amount == total_supply_assets − total_borrow_assets`
still holds exactly. **P-BADDEBT-1** asserts it over random states.

### 8.3 What Aegis does *not* claim

Aegis does **not** guarantee lenders are made whole. It guarantees:
- losses are **contained** in the originating market (isolation);
- losses are **recognized** promptly and permissionlessly (no admin discretion, no oracle dependency);
- protocol fees absorb losses **before** lenders;
- the accounting stays **exactly reconciled** through insolvency.

That is the honest scope of a v1 overcollateralized lender.

---

## 9. Edge cases (each is a required test)

| # | Case | Required behavior | Test |
|---|---|---|---|
| E-01 | First supply into an empty market | Virtual offsets applied; no division by zero | `U-SHARE-01` |
| E-02 | `dt = 0` accrual (same-slot double call) | No-op; totals unchanged; idempotent | `U-IRM-01` |
| E-03 | `total_supply_assets = 0`, `dt > 0` | `u = 0`, `r = base_rate`, `interest = 0` | `U-IRM-02` |
| E-04 | Borrow when `total_borrow == total_supply` (u = 100%) | `r` capped at `max_rate_ps`; borrow of any amount fails on liquidity | `U-IRM-04` |
| E-05 | Withdraw more than free liquidity | Fail `InsufficientLiquidity`; never partial-fill silently | `U-WD-01` |
| E-06 | Repay more than debt | Clamp to debt; refund nothing (never pull excess tokens) | `U-REPAY-01` |
| E-07 | Repay entire debt via shares | `borrow_shares` reaches exactly 0, no dust share remains | `U-REPAY-02` |
| E-08 | Withdraw all collateral with zero debt | Allowed without any oracle read | `U-WDC-01` |
| E-09 | Withdraw collateral with non-zero debt and stale oracle | Fail closed | `A-ORACLE-03` |
| E-10 | Deposit collateral with stale oracle | **Succeeds** (risk-reducing) | `A-ORACLE-01` |
| E-11 | Repay with stale oracle | **Succeeds** (risk-reducing) | `A-ORACLE-02` |
| E-12 | Liquidate exactly at `HF = WAD` | **Not** liquidatable (strict `<`) | `U-LIQ-02` |
| E-13 | Liquidate with `total_seize > collateral` | Clamp path; repay recomputed upward-rounded | `U-LIQ-03` |
| E-14 | Liquidation leaving dust debt | Forced to full repayment | `U-LIQ-04` |
| E-15 | Liquidation seizing all collateral, debt remains | Succeeds; position becomes bad-debt-eligible | `U-LIQ-05` |
| E-16 | `absorb_bad_debt` with collateral > 0 | Rejected | `U-BD-01` |
| E-17 | `absorb_bad_debt` when fee shares exceed the loss | Fully absorbed by protocol; lenders untouched | `U-BD-02` |
| E-18 | Price = 0 or negative from oracle | Rejected before any arithmetic | `A-ORACLE-04` |
| E-19 | `conf/price` above `max_conf_bps` | Fail closed | `A-ORACLE-05` |
| E-20 | Collateral mint with 0 decimals; loan mint with 9 | Valuation correct; no shift errors | `P-VAL-1` |
| E-21 | Borrow of 0 / supply of 0 / repay of 0 | Rejected (`ZeroAmount`) | `U-GUARD-01` |
| E-22 | Both `assets` and `shares` non-zero in one call | Rejected (`InconsistentInput`) | `U-GUARD-02` |
| E-23 | Both `assets` and `shares` zero | Rejected | `U-GUARD-03` |
| E-24 | Interest accrual on a market dormant for 1 year | Bounded by `max_rate_ps`; no overflow; Taylor under-approximates | `P-IRM-3` |
| E-25 | Position debt below `min_debt` after borrow | Rejected unless debt is 0 | `U-BORROW-02` |

---

## 10. Property / invariant tests (economics)

| ID | Property |
|---|---|
| `P-ARITH-1` | `mul_div_floor(a,b,d) ≤ mul_div_ceil(a,b,d) ≤ mul_div_floor(a,b,d)+1` |
| `P-ARITH-2` | `mul_div_*` never panics; overflow returns `Err` |
| `P-ARITH-3` | `to_assets_*` succeeds for the maximum legal share/asset state (256-bit intermediate required) |
| `P-SHARE-1` | `to_assets_down(to_shares_down(a, T, S), T, S) ≤ a` for all valid `(a,T,S)` — round-tripping never creates value |
| `P-SHARE-2` | `to_shares_up(to_assets_up(s, T, S), T, S) ≥ s` |
| `P-SHARE-3` | Supply-then-immediately-withdraw never returns more than was supplied |
| `P-SHARE-4` | Borrow-then-immediately-repay never repays less than was borrowed |
| `P-IRM-1` | `r` is monotone non-decreasing in `u` |
| `P-IRM-2` | `taylor3(x) ≤ e^x − 1` for `x ≥ 0` (checked against a high-precision reference) |
| `P-IRM-3` | Accrual over `n` steps of `dt` ≤ accrual over one step of `n·dt` (sub-additivity of the discount) |
| `P-FEE-1` | Fee shares dilute lenders by exactly `fee_amount` (±1 unit) |
| `P-ACCRUE-1` | `accrue_view` totals == `accrue_mut` totals |
| `P-ACCRUE-2` | `total_supply_assets − total_borrow_assets` is invariant under accrual |
| `P-VAL-1` | Valuation is correct for every decimals pair in `0..=12` |
| `P-VAL-2` | `collateral_value` is monotone non-decreasing in `collateral_amount` and in `price_c_lo` |
| `P-LIQ-1` | If `HF_before > LT·(1+b)` then `HF_after > HF_before` |
| `P-LIQ-2` | Liquidation never seizes more collateral than the position holds |
| `P-LIQ-3` | Liquidation is profitable for the liquidator whenever `HF < WAD` and collateral is not clamped |
| `P-LIQ-4` | For the reference params, `full_liq_hf ≥ LT·(1+b)` |
| `P-BADDEBT-1` | Vault reconciliation holds through `absorb_bad_debt` |
| `P-GLOBAL-1` | Over any random sequence of operations, all of `INV-ACC-*`, `INV-CUS-*`, `INV-SOLV-*` hold after every step (stateful fuzzer, Phase 10) |

---

## 11. Deliberate simplifications in v1

These are **portfolio-grade**, not production-grade. Each is stated so no reader mistakes v1 for
something deployable with real capital.

| Simplification | Why acceptable in v1 | What production would require |
|---|---|---|
| Stateless piecewise-linear IRM | Deterministic and fully testable | Adaptive/PID curve responding to sustained utilization; empirical calibration |
| Fixed-percentage liquidation bonus | Simple, analyzable, profitable | Dutch-auction or health-scaled bonus to reduce over-liquidation |
| Single oracle source (Pyth) per asset | Sufficient to demonstrate oracle safety | Multi-oracle median/fallback, circuit breakers, per-asset TWAP sanity bands |
| No supply/borrow caps | Not needed at zero TVL | Hard caps per market, per-asset global exposure limits |
| No timelock on parameter loosening (until Phase 12) | Single-operator development | Timelock + multisig + public parameter-change notice |
| Third-order Taylor compounding | Error < 0.05% at realistic `dt`, and always in the borrower's favor | Formal error bound with adversarial `dt`, or exact fixed-point `exp` |
| No slippage/liquidity modelling in risk params | Out of scope for v1 | LTVs derived from measured on-chain depth and historical volatility |
| `min_debt` as the only dust defense | Adequate | Gas-aware minimum position sizing, liquidation-cost modelling |
| Bad debt socialized pro-rata with no lender opt-out | Honest and simple | Tranching, insurance fund, or explicit first-loss capital |
| Risk parameters chosen by illustration, not by research | v1 is an engineering artifact | Quantitative risk research per asset; this is a discipline, not a constant |

**Stated plainly: Aegis v1 must not be deployed to mainnet with real user capital.** The economic
parameters are illustrative, the oracle configuration is single-source, and the code will not have been
audited. The engineering rigor is real; the risk calibration is not a substitute for risk research.
