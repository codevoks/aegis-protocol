# Aegis — Formal Invariant Catalogue

**Status: FROZEN (Phase 0). An invariant may be added, but never weakened or removed, without an ADR.**

Every invariant is (a) precise enough to be mechanically checkable, (b) mapped to an implementation
site, (c) mapped to at least one **falsifying** test, and (d) assigned to a phase.

The stateful fuzzer (Phase 10) asserts every invariant marked **[GLOBAL]** after *every* instruction in
a randomly generated sequence. Invariants marked **[LOCAL]** are checked at their specific call sites.

Column key — *Test*: `U-` unit, `P-` property, `A-` adversarial, `I-` integration, `F-` fuzz.

---

## A. Authorization

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-AUTH-01 | Only `protocol.admin` may execute `create_market`, `set_market_params`, `set_guardian`, `set_pending_admin`, `withdraw_collateral_fees`. | `has_one = admin` on `Protocol` | `A-AUTH-01` | 2 |
| INV-AUTH-02 | Only `position.owner`, as a transaction signer, may execute `withdraw`, `withdraw_collateral`, `borrow`, `close_position`. | `Signer` + `has_one = owner` | `A-AUTH-02` | 3/4 |
| INV-AUTH-03 | An owner signature is required **exactly** for operations that reduce a position's safety or extract value. `repay`, `deposit_collateral`, `init_position`, `liquidate`, `absorb_bad_debt`, `accrue_interest` require no owner signature. | per-instruction accounts | `A-AUTH-03` | 4 |
| INV-AUTH-04 | Only `protocol.guardian` or `protocol.admin` may pause; the **guardian may only set pause bits, never clear them**. | `set_*_pause` | `A-AUTH-04` | 12 |
| INV-AUTH-05 | Only `protocol.pending_admin` may execute `accept_admin`, and only while `pending_admin != default`. | `accept_admin` | `A-AUTH-05` | 12 |
| INV-AUTH-06 | No instruction accepts an account whose owner program is not explicitly validated. | Anchor `Account`/`InterfaceAccount` | `A-AUTH-06` | 2 |
| INV-AUTH-07 | Aegis never forwards a user's signer privilege to an external program. The only PDA signer is the `Market`, and it signs only for its own two vaults. | CPI sites | `A-AUTH-07` | 3 |

## B. Token custody

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| **INV-CUS-01** **[GLOBAL]** | `loan_vault.amount == total_supply_assets − total_borrow_assets` exactly, after every instruction. | all loan-side ix | `F-INV-01`, `I-CUS-01` | 4 |
| **INV-CUS-02** **[GLOBAL]** | `collateral_vault.amount == Σ(position.collateral_amount) + market.collateral_fee_accrued` exactly. | all collateral-side ix | `F-INV-02`, `I-CUS-02` | 3 |
| INV-CUS-03 | Only the `Market` PDA is the token authority of `collateral_vault` and `loan_vault`, for the market's whole lifetime. | `create_market` | `A-CUS-03` | 2 |
| INV-CUS-04 | Tokens leave a vault only via the six paths enumerated in `account-model.md` §6.3. No other code path invokes a token transfer with the market as signer. | code review + grep test | `A-CUS-04` | 13 |
| INV-CUS-05 | Every vault inflow is credited by measured delta (`after − before` following a `reload()`), never by the requested amount. | all inbound transfers | `U-TOK-02` | 7 |
| INV-CUS-06 | Every token transfer uses `transfer_checked` with the market's pinned mint and cached decimals. | all transfers | `A-CUS-06` | 3 |
| INV-CUS-07 | The token program passed for an asset equals the program pinned in `Market` at creation. | all token ix | `A-TOK-08` | 3 |
| INV-CUS-08 | Unsolicited tokens sent directly to a vault are **never** credited to any position or to any market total. They are permanently unaccounted-for surplus. | absence of any balance-sync path | `A-CUS-08` | 4 |
| INV-CUS-09 | `market.collateral_fee_accrued` increases only inside `liquidate`, by exactly `protocol_cut`. | `liquidate` | `A-ADM-02` | 6 |

**INV-CUS-01/02 are the two most important invariants in Aegis.** They are exact equalities, not
bounds. INV-CUS-08 is what makes them stable: a protocol that syncs internal accounting to observed
vault balances is donation-attackable, so Aegis never reads a vault balance as a source of truth —
only as a delta measurement across its own CPI.

## C. Accounting

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-ACC-01 **[GLOBAL]** | `total_supply_shares == Σ(position.supply_shares)` over all positions in the market, including `fee_position`. | all supply-side ix | `F-INV-03` | 4 |
| INV-ACC-02 **[GLOBAL]** | `total_borrow_shares == Σ(position.borrow_shares)`. | all borrow-side ix | `F-INV-04` | 4 |
| INV-ACC-03 **[GLOBAL]** | `total_supply_assets ≥ total_borrow_assets`. | `borrow`, `withdraw` | `F-INV-05` | 4 |
| INV-ACC-04 | Interest accrual leaves `total_supply_assets − total_borrow_assets` unchanged. | `accrue_mut` | `P-ACCRUE-2` | 4 |
| INV-ACC-05 | `absorb_bad_debt` leaves `total_supply_assets − total_borrow_assets` unchanged and moves no tokens. | `absorb_bad_debt` | `P-BADDEBT-1` | 6 |
| INV-ACC-06 | `total_supply_shares == 0 ⟺ total_supply_assets == 0`, and likewise for borrow. (No orphaned assets without shares, or shares without assets.) | all ix | `F-INV-06` | 4 |
| INV-ACC-07 | `market.last_accrual_ts` is monotonically non-decreasing and never exceeds the current `Clock` timestamp. | `accrue_mut` | `U-IRM-05` | 4 |
| INV-ACC-08 | `accrue_view(s, t)` totals equal `accrue_mut(s, t)` totals for all `s`, `t`. | `aegis-math` | `P-ACCRUE-1` | 4 |
| INV-ACC-09 | No arithmetic operation wraps. All overflow aborts the transaction. | `overflow-checks`, `mul_div_*` | `P-ARITH-2` | 2 |
| INV-ACC-10 | No `f32`/`f64` appears anywhere in the on-chain crate. | CI grep | `CI-NOFLOAT` | 1 |
| INV-ACC-11 | `_reserved` bytes in every persisted account are zero. | account writes | `U-ACCT-01` | 2 |

## D. Solvency

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-SOLV-01 **[GLOBAL]** | After `borrow` or `withdraw_collateral`, the position satisfies `debt_value ≤ collateral_value · max_ltv / WAD`. | health guard | `F-INV-07`, `A-SOLV-01` | 4 |
| INV-SOLV-02 | A position with `HF ≥ WAD` cannot be liquidated. | `liquidate` precondition | `A-LIQ-01` | 6 |
| INV-SOLV-03 | `absorb_bad_debt` requires `collateral_amount == 0` and `borrow_shares > 0`. | precondition | `U-BD-01` | 6 |
| INV-SOLV-04 **[GLOBAL]** | INV-CUS-01 holds through and after `absorb_bad_debt`. | `absorb_bad_debt` | `P-BADDEBT-1` | 6 |
| INV-SOLV-05 | Bad debt in market M never changes any state of any market M′ ≠ M. | account model | `I-ISO-01` | 6 |
| INV-SOLV-06 | Protocol fee shares are burned before any lender loss is socialized. | `absorb_bad_debt` | `U-BD-02` | 6 |
| INV-SOLV-07 | A position's debt is either `0` or `≥ market.min_debt`. | `borrow`, `liquidate` | `U-BORROW-02`, `U-LIQ-04` | 6 |

## E. Borrowing

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-BOR-01 | `borrow` requires a valid oracle for both assets and fails closed otherwise. | oracle guard | `A-ORACLE-03` | 5 |
| INV-BOR-02 | `borrow` cannot remove more than free liquidity (`total_supply_assets − total_borrow_assets`). | precondition | `U-BORROW-01` | 4 |
| INV-BOR-03 | Borrow shares are computed with `to_shares_up` (borrower owes at least the exact amount). | `borrow` | `U-ROUND-03` | 4 |
| INV-BOR-04 | `borrow` fails when the `BORROW` pause bit is set. | pause guard | `A-ADM-03` | 12 |
| INV-BOR-05 | A borrow of zero assets and zero shares is rejected. | guard | `U-GUARD-01` | 4 |

## F. Repayment

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-REP-01 | `repay` never requires an oracle. | instruction accounts | `A-ORACLE-02` | 5 |
| INV-REP-02 | `repay` cannot be paused by admin or guardian. | pause guard exclusion | `A-ADM-01` | 12 |
| INV-REP-03 | `repay` never transfers more than the position's outstanding debt (clamped). | `repay` | `U-REPAY-01` | 4 |
| INV-REP-04 | Repay shares are computed with `to_shares_down` (payer credited no more than exact). | `repay` | `U-ROUND-04` | 4 |
| INV-REP-05 | Full repayment drives `position.borrow_shares` to exactly `0`, leaving no dust share. | `repay` | `U-REPAY-02` | 4 |

## G. Oracle

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-ORA-01 | Every priced instruction validates checks O-1..O-11 (`oracle-design.md` §2) and fails closed on any failure. | `require_valid_price` | `A-ORACLE-03..11` | 5 |
| INV-ORA-02 | Risk-reducing operations (`repay`, `deposit_collateral`, `absorb_bad_debt`, `supply`, `withdraw`, debt-free `withdraw_collateral`) succeed regardless of oracle state. | instruction accounts | `A-ORACLE-01`, `A-ORACLE-02` | 5 |
| INV-ORA-03 | Collateral is valued at the confidence **lower** bound, rounded **down**; debt at the **upper** bound, rounded **up**. | valuation | `U-HEALTH-01` | 5 |
| INV-ORA-04 | A price account whose `feed_id` differs from the market's configured feed is rejected. | O-3 | `A-ORACLE-07` | 5 |
| INV-ORA-05 | The two price accounts in a priced instruction are distinct accounts. | O-11 | `A-ORACLE-12` | 5 |
| INV-ORA-06 | Staleness is measured in unix seconds, never slots. | `Clock::unix_timestamp` | `CI-NOSLOT` | 5 |
| INV-ORA-07 | A failed oracle check leaves **no** state modified. | ordering: validate before mutate | `A-ORACLE-13` | 5 |

## H. Liquidation

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-LIQ-01 | Liquidation is possible only when `HF < WAD` (strict). | precondition | `U-LIQ-02` | 6 |
| INV-LIQ-02 | Seized collateral never exceeds `position.collateral_amount`. | clamp path | `P-LIQ-2` | 6 |
| INV-LIQ-03 | Repaid assets never exceed the position's outstanding debt. | clamp | `U-LIQ-03` | 6 |
| INV-LIQ-04 | Repayment never exceeds `close_factor × debt`, unless `HF < full_liq_hf` or the dust rule forces full repayment. | `max_repay` | `U-LIQ-04` | 6 |
| INV-LIQ-05 | If `HF_before > liq_threshold · (WAD + liq_bonus) / WAD`, then `HF_after > HF_before`. | derived bound | `P-LIQ-1` | 6 |
| INV-LIQ-06 | `create_market` and `set_market_params` reject any parameter set where `liq_threshold · (WAD + liq_bonus) / WAD ≥ WAD`. | bounds check | `A-ADM-04` | 2 |
| INV-LIQ-07 | The protocol cut is taken from the **bonus only**, never from the liquidator's principal-equivalent seizure. | `liquidate` | `U-LIQ-01` | 6 |
| INV-LIQ-08 | Liquidation is strictly profitable for the liquidator whenever `HF < WAD` and the collateral clamp is not hit. | economics | `P-LIQ-3` | 6 |
| INV-LIQ-09 | Liquidation never increases `total_supply_assets`. Lenders are repaid, never enriched, by a liquidation. | `liquidate` | `U-LIQ-06` | 6 |

## I. State lifecycle

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-LIFE-01 | A `Position` can never be re-initialized while it exists. No `init_if_needed` anywhere in the program. | `init` + CI grep | `A-LIFE-01`, `CI-NOINITIF` | 2 |
| INV-LIFE-02 | `close_position` requires all three balances to be exactly zero. | precondition | `U-LIFE-01` | 3 |
| INV-LIFE-03 | A closed position is fully defunded and its discriminator cleared; it can only ever be recreated empty. | Anchor `close` | `A-LIFE-02` | 3 |
| INV-LIFE-04 | `Market` and `Protocol` are never closable by any instruction. | absence of a close path | `CI-NOCLOSE` | 2 |
| INV-LIFE-05 | Every PDA uses its canonical bump, stored at creation and reused thereafter. | `bump = acct.bump` | `A-LIFE-03` | 2 |
| INV-LIFE-06 | No two account types share a seed prefix. | seed constants | `U-LIFE-02` | 2 |

## J. Administrative safety

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| **INV-ADM-01** | **The admin can never transfer, seize, or redirect user funds.** The only admin-initiated token movement is `withdraw_collateral_fees`, bounded by `market.collateral_fee_accrued`. | account model | `A-ADM-02` | 6 |
| INV-ADM-02 | Admin transfer is two-step (`set_pending_admin` → `accept_admin`). | instructions | `A-AUTH-05` | 12 |
| INV-ADM-03 | Pausing can only set the four defined bits; no other behavior is reachable via `paused`. | bitmask validation | `A-ADM-05` | 12 |
| **INV-ADM-04** | **`repay`, `deposit_collateral`, `absorb_bad_debt`, and `close_position` can never be paused**, by anyone, ever. | pause guard exclusion | `A-ADM-01` | 12 |
| INV-ADM-05 | All admin-set parameters are validated against the bounds in `economic-model.md` §5 on **every** write. | bounds check | `A-ADM-04` | 2 |
| INV-ADM-06 | `set_market_params` can never change mints, token programs, vault addresses, cached decimals, or `config_id`. | field exclusion | `A-ADM-06` | 12 |
| INV-ADM-07 | `set_market_params` calls `accrue_mut` before applying changes, so accrued interest settles under the old parameters. | ordering | `U-ADM-01` | 12 |
| INV-ADM-08 | `withdraw_collateral_fees` cannot withdraw more than `collateral_fee_accrued`. | precondition | `A-ADM-02` | 6 |
| INV-ADM-09 | Loosening a risk parameter is timelocked; tightening applies immediately. | Phase 12 | `I-ADM-01` | 12 |

**INV-ADM-01 and INV-ADM-04 are the two properties that make Aegis credibly non-custodial.** They are
structural, not policy: there is no code path for the admin to move user assets, and no code path to
prevent a user from repaying or topping up. Both have dedicated adversarial tests that attempt the
attack and must fail.

## K. Upgrade / governance

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-UPG-01 | Account layout changes are performed via an explicit migration instruction using Anchor's `Migration<From, To>`; never by reinterpreting bytes in place. | Phase 12 | `I-UPG-01` | 12 |
| INV-UPG-02 | Every account carries a version discriminant or reserved space sufficient for additive migration without realloc. | `_reserved` | `U-ACCT-01` | 2 |
| INV-UPG-03 | A migration is idempotent and rejects an already-migrated account. | Phase 12 | `I-UPG-02` | 12 |
| INV-UPG-04 | The upgrade authority is documented at every stage of the protocol's life, and its risk is stated in `governance.md`. | docs | review | 12 |
| INV-UPG-05 | A published build is byte-reproducible and verifiable against the deployed program. | verifiable build | `I-UPG-03` | 12 |

## L. Runtime / resource safety

| ID | Invariant | Impl | Test | Phase |
|---|---|---|---|---|
| INV-RES-01 | Every user-facing instruction completes within the default 200k CU budget, with a committed measurement. | benchmarks | `B-CU-*` | 11 |
| INV-RES-02 | `deposit_collateral` and `withdraw_collateral` do not declare `Market` writable. | account structs | `A-PAR-01` | 3 |
| INV-RES-03 | No writable account is shared between two distinct markets. | account model | `A-PAR-02` | 6 |
| INV-RES-04 | No instruction contains an unbounded loop or a loop over user-controlled length. | code review + CI | `CI-NOLOOP` | 11 |
| INV-RES-05 | No account requires `realloc` during normal operation. | fixed sizes | `U-ACCT-02` | 2 |
| INV-RES-06 | Every instruction's account count fits a legacy (1232-byte) transaction without an address-lookup table. | SDK test | `I-TX-01` | 9 |
| INV-RES-07 | No instruction CPIs into a program that is not `spl_token`, `spl_token_2022`, or `system` — except the Phase 8 liquidation callback, which is opt-in and post-condition-verified. | CPI audit | `A-CPI-01` | 8 |

---

## Coverage summary

| Group | Count | GLOBAL (fuzzer-asserted) |
|---|---|---|
| A. Authorization | 7 | – |
| B. Token custody | 9 | 2 |
| C. Accounting | 11 | 4 |
| D. Solvency | 7 | 3 |
| E. Borrowing | 5 | – |
| F. Repayment | 5 | – |
| G. Oracle | 7 | – |
| H. Liquidation | 9 | – |
| I. State lifecycle | 6 | – |
| J. Administrative safety | 9 | – |
| K. Upgrade / governance | 5 | – |
| L. Runtime / resource safety | 7 | – |
| **Total** | **87** | **9** |

**Enforcement rule (also in `AGENTS.md`):** an invariant without a falsifying test is not an invariant,
it is a hope. Phase completion requires the phase's invariant tests to exist, to fail when the check is
deliberately removed, and to pass when it is restored. A test that passes both with and without the
check is not testing anything.
