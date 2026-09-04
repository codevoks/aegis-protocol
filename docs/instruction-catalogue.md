# Aegis — Instruction Catalogue

**Status: FROZEN (Phase 0). Adding, removing, or changing the account list of an instruction requires an ADR.**

Notation: `[S]` signer, `[W]` writable, `[R]` read-only, `[PDA]` program-derived.
Every instruction implicitly validates: account owner, Anchor discriminator, canonical PDA bump, and
all `has_one` relations. Those are not repeated per instruction.

**Universal preconditions** (checked by a shared guard on every instruction):
`Protocol` is the canonical singleton · every passed token program equals the market's pinned program ·
all arithmetic is checked · no duplicate mutable accounts (Anchor 1.0 default; `dup` is never used).

---

## Instruction summary

| # | Instruction | Caller | Oracle? | Writes `Market`? | Pausable? | Phase |
|---|---|---|---|---|---|---|
| 1 | `initialize_protocol` | admin | – | – | – | 2 |
| 2 | `set_pending_admin` | admin | – | – | – | 12 |
| 3 | `accept_admin` | pending admin | – | – | – | 12 |
| 4 | `set_guardian` | admin | – | – | – | 12 |
| 5 | `set_protocol_pause` | admin or guardian | – | – | – | 12 |
| 6 | `create_market` | admin | – | – | – | 2 |
| 7 | `set_market_params` | admin | – | yes | – | 12 |
| 8 | `set_market_pause` | admin or guardian | – | yes | – | 12 |
| 9 | `init_position` | anyone (payer) | – | no | no | 2 |
| 10 | `deposit_collateral` | anyone | **no** | **no** | **no** | 3 |
| 11 | `withdraw_collateral` | owner | yes | **no** | yes | 3 |
| 12 | `supply` | lender | no | yes | yes | 4 |
| 13 | `withdraw` | lender | no | yes | yes | 4 |
| 14 | `borrow` | owner | yes | yes | yes | 4 |
| 15 | `repay` | anyone | **no** | yes | **no** | 4 |
| 16 | `accrue_interest` | anyone | no | yes | no | 4 |
| 17 | `liquidate` | anyone | yes | yes | yes | 6 |
| 18 | `absorb_bad_debt` | anyone | **no** | yes | **no** | 6 |
| 19 | `withdraw_collateral_fees` | admin | no | yes | no | 6 |
| 20 | `close_position` | owner | no | no | no | 3 |

Phases 3 and 4 ship instructions whose oracle dependency is stubbed by a compile-time-absent path
until Phase 5; see the phase specs for exactly how that is sequenced without shipping insecure code.

---

## 1. `initialize_protocol(args: InitProtocolArgs)`

- **Caller/signer:** deployer, becomes `admin`. `[S][W] payer`
- **Accounts:** `[W][PDA] protocol` (init) · `[R] system_program`
- **Args:** `guardian: Pubkey`, `fee_recipient: Pubkey`
- **Preconditions:** `protocol` does not exist. `guardian != default`. `fee_recipient != default`.
- **State transition:** `admin = payer`, `pending_admin = default`, `guardian`, `fee_recipient`,
  `paused = 0`, `bump`.
- **Tokens:** none. **Arithmetic:** none.
- **Events:** `ProtocolInitialized`
- **Invariants:** INV-ACCT-01, INV-AUTH-01
- **Failure cases:** already initialized (Anchor `init` rejects); zero-pubkey guardian/fee recipient.
- **Attack vectors:** *front-running initialization* — whoever lands first becomes admin. Mitigation:
  deploy and initialize in the same operational step and verify `admin` before funding anything; the
  Phase 2 deployment checklist requires asserting `protocol.admin` post-deploy.

## 2–5. Administrative instructions

### `set_pending_admin(new_admin)` / `accept_admin()`
Two-step transfer. `set_pending_admin` writes `pending_admin`; `accept_admin` requires
`signer == pending_admin`, sets `admin = pending_admin`, clears `pending_admin`.
**Why two-step:** a single-step transfer to a typo'd or non-existent key permanently bricks
governance. This is the standard mitigation and is cheap.
*Events:* `AdminTransferStarted`, `AdminTransferred`. *Invariant:* INV-ADM-01.

### `set_guardian(new_guardian)`
Admin only. *Invariant:* INV-ADM-02.

### `set_protocol_pause(flags: u8)`
**Admin or guardian.** Asymmetric authority, deliberately:
- **Guardian may only set bits** (increase pausing) — the emergency key can stop the protocol but
  never restart it.
- **Admin may set or clear bits.**

Bits `SUPPLY|BORROW|WITHDRAW|LIQUIDATE` only. `repay`, `deposit_collateral`, `absorb_bad_debt` and
`close_position` are structurally unpausable (INV-ADM-04) — **a pause must never trap a user's funds
or prevent them from reducing their own risk.**

Pausing `LIQUIDATE` is itself dangerous (it lets bad debt accumulate) and is documented in the
threat model as an accepted operator power with a runbook, not as a safe default.

*Events:* `ProtocolPauseSet`. *Invariants:* INV-ADM-03, INV-ADM-04.

---

## 6. `create_market(args: CreateMarketArgs)`

- **Caller/signer:** `[S][W] admin` (must equal `protocol.admin`)
- **Accounts:**
  `[R][PDA] protocol` · `[W][PDA] market` (init) · `[R] collateral_mint` · `[R] loan_mint` ·
  `[W][PDA] collateral_vault` (init) · `[W][PDA] loan_vault` (init) ·
  `[W][PDA] fee_position` (init) · `[R] collateral_token_program` · `[R] loan_token_program` ·
  `[R] system_program`
- **Args:** `config_id: u16`, oracle config (`oracle_kind`, `collateral_feed_id`, `loan_feed_id`,
  `max_price_age_secs`, `max_conf_bps`), risk params, IRM params, `min_debt`, `ack_freeze_authority: bool`
- **Preconditions:**
  1. `admin == protocol.admin`
  2. `collateral_mint != loan_mint` — *a same-asset market is degenerate: LTV is meaningless and
     liquidation is a no-op. Rejected explicitly.*
  3. Each token program is SPL Token or Token-2022, matching each mint's actual owner.
  4. **Token extension policy check** for both mints (see `token-compatibility.md`) — reject any
     extension outside the allowlist; reject transfer-fee mints as the *loan* asset.
  5. If either mint has a `freeze_authority`, require `ack_freeze_authority == true`.
  6. Risk-parameter bounds (economic-model §5), including
     `max_ltv < liq_threshold < WAD` and `liq_threshold·(WAD + liq_bonus)/WAD < WAD`.
  7. IRM bounds: `0 < u_kink < WAD`, `max_rate_ps > 0`, all rates ≤ `max_rate_ps`.
  8. `min_debt > 0`.
  9. `max_price_age_secs` in `[1, 3600]`; `max_conf_bps` in `[1, 2000]`.
- **State transition:** market fully populated; `last_accrual_ts = now`; totals zero; decimals cached
  from the mints; vaults created with `Market` as authority and the correct Token-2022 length.
- **Tokens:** two token accounts created; **no transfers**.
- **Events:** `MarketCreated` (full parameter snapshot — this is the audit record for the market's
  risk configuration)
- **Invariants:** INV-ACCT-02, 04, 05, 06; INV-ADM-05 (bounds); INV-TOK-01 (policy)
- **Attack vectors:**
  - *Mint/token-program mismatch* → mint owner is checked against the passed program.
  - *Hostile mint with a permanent delegate or transfer hook* → rejected by the policy check. This is
    the single most important check in `create_market`; a permanent delegate can drain the vault.
  - *Parameter griefing (e.g. bonus that makes liquidation worsen HF)* → the derived bound in
    precondition 6 makes it unrepresentable.
  - *`config_id` squatting* → admin-only, so not exploitable in v1.

## 7. `set_market_params(args)`

Admin only. Updates risk params, IRM params, oracle params, `fee`, `min_debt`, `fee_recipient`.
**Never** updates mints, token programs, vaults, decimals, or `config_id` — those are immutable
identity (INV-ADM-06).

All bounds from `create_market` are re-checked. Calls `accrue_mut` **before** applying changes, so
interest already earned is settled under the old parameters (INV-ADM-07) — otherwise a fee increase
would retroactively tax accrued interest.

Phase 12 adds the tighten/loosen asymmetry: risk-*reducing* changes apply immediately; risk-*increasing*
changes (raising `max_ltv`/`liq_threshold`/`fee`, lowering `min_debt`) are staged in
`PendingMarketParams` with an `effective_at` timelock.

*Events:* `MarketParamsUpdated` (before/after snapshot).

## 8. `set_market_pause(flags)`
Same authority asymmetry as `set_protocol_pause`, scoped to one market.

---

## 9. `init_position()`

- **Caller/signer:** `[S][W] payer` (may be anyone)
- **Accounts:** `[R][PDA] market` · `[R] owner` (not a signer) · `[W][PDA] position` (init) ·
  `[R] system_program`
- **Preconditions:** position does not exist; `owner != default`.
- **State transition:** all balances zero; `market`, `owner`, `bump` set.
- **Why a separate instruction:** `init_if_needed` is the canonical reinitialization footgun. An
  explicit `init` makes reinitialization impossible (Anchor's `init` fails if the account exists) and
  makes account creation an auditable, visible step. The SDK bundles it into the user's first
  transaction so the UX cost is zero (Phase 9).
- **Events:** `PositionInitialized`
- **Invariants:** INV-ACCT-03, INV-LIFE-01
- **Attack vectors:** *reinit attack* → impossible. *Griefing by pre-creating another user's position*
  → harmless: it can only be created empty and only that owner can ever act on it.

## 20. `close_position()`

- **Caller/signer:** `[S][W] owner`
- **Accounts:** `[R][PDA] market` · `[W][PDA] position` (`close = owner`)
- **Preconditions:** `supply_shares == 0 && borrow_shares == 0 && collateral_amount == 0`;
  `position.owner == owner`.
- **Tokens:** none. Lamports returned to owner.
- **Events:** `PositionClosed`
- **Invariants:** INV-LIFE-02, INV-LIFE-03
- **Attack vectors:** *revival / stale-data reuse* → Anchor's `close` zeroes the discriminator and
  defunds; a later `init_position` recreates it empty. *Closing with dust* → the three equality checks
  are exact, never `< epsilon`.

---

## 10. `deposit_collateral(amount: u64)`

- **Caller/signer:** `[S][W] payer/depositor` — **need not be the position owner** (§5.1 of the account model)
- **Accounts:** `[R][PDA] market` **(read-only — see below)** · `[W][PDA] position` ·
  `[W] collateral_vault` · `[W] depositor_collateral_ata` · `[R] collateral_mint` ·
  `[R] collateral_token_program`
- **Preconditions:** `amount > 0`; `position.market == market`; token program matches
  `market.collateral_token_program`; mint matches `market.collateral_mint`.
  **No oracle. No pause check. No health check.**
- **State transition:** `position.collateral_amount += credited`
- **Tokens:** `transfer_checked(depositor_ata → collateral_vault, amount)`, then **reload the vault**
  and compute `credited = after − before` (measured-delta accounting).
- **Arithmetic:** one checked add.
- **Events:** `CollateralDeposited { market, position, amount_in: amount, credited }`
- **Invariants:** INV-CUS-02, INV-ACCT-08, INV-ADM-04
- **Why `Market` is read-only here:** collateral is not pooled, so no market scalar changes. This is
  what delivers claim C2 (intra-market collateral parallelism). **A future change that adds a
  `Market` write here silently destroys that property** — flagged in code and in the Phase 11
  regression check.
- **Attack vectors:** *fake vault* → PDA-derived **and** `has_one`-checked against `market.collateral_vault`.
  *Wrong mint* → `transfer_checked` plus explicit mint equality. *Transfer-fee mint over-crediting* →
  measured delta. *Depositing to another user's position* → allowed and harmless (risk-reducing).

## 11. `withdraw_collateral(amount: u64)`

- **Caller/signer:** `[S] owner` (**required**)
- **Accounts:** `[R][PDA] protocol` · `[R][PDA] market` **(read-only)** · `[W][PDA] position` ·
  `[W] collateral_vault` · `[W] owner_collateral_ata` · `[R] collateral_mint` ·
  `[R] collateral_token_program` · `[R] collateral_price_update` · `[R] loan_price_update`
- **Preconditions:** not paused (`WITHDRAW`); `amount > 0`; `amount ≤ position.collateral_amount`;
  `position.owner == owner`.
  **If `position.borrow_shares > 0`:** oracle must be valid and, using `accrue_view` totals, the
  post-withdrawal LTV must satisfy `debt_value ≤ collateral_value · max_ltv / WAD`.
  **If `position.borrow_shares == 0`:** no oracle read at all (E-08) — a user with no debt must always
  be able to retrieve their own collateral, even during an oracle outage.
- **State transition:** `position.collateral_amount -= amount`
- **Tokens:** `transfer_checked(collateral_vault → owner_ata, amount)` signed by the market PDA.
- **Arithmetic:** `accrue_view`, valuation, LTV comparison.
- **Events:** `CollateralWithdrawn`
- **Invariants:** INV-SOLV-01, INV-AUTH-02, INV-ORA-01, INV-ACCT-08
- **Attack vectors:** *withdraw into insolvency* → post-state health check. *Stale price to inflate
  collateral* → oracle validity (fail closed). *Missing signer* → `owner` is a required signer;
  this is the Wormhole-class bug and is covered by dedicated test `A-AUTH-02`. *Reading pre-accrual
  debt* → `accrue_view` is mandatory before the check.

---

## 12. `supply(assets: u64, shares: u128)`

Exactly one of `assets`/`shares` is non-zero (E-22, E-23).

- **Caller/signer:** `[S][W] lender`
- **Accounts:** `[R][PDA] protocol` · `[W][PDA] market` · `[W][PDA] position` ·
  `[W][PDA] fee_position` · `[W] loan_vault` · `[W] lender_loan_ata` · `[R] loan_mint` ·
  `[R] loan_token_program`
- **Preconditions:** not paused (`SUPPLY`); exactly one input non-zero.
- **State transition:** `accrue_mut` (which may mint fee shares to `fee_position`), then
  `assets`-given → `shares = to_shares_down(...)`; `shares`-given → `assets = to_assets_up(...)`;
  `position.supply_shares += shares`; `total_supply_shares += shares`;
  `total_supply_assets += credited`.
- **Tokens:** `transfer_checked(lender_ata → loan_vault, assets)`; **measured delta**; the credited
  amount is what is recorded (loan assets are policy-restricted to fee-free mints, so `credited ==
  assets` is expected — but it is *verified*, not assumed, and a mismatch aborts).
- **Events:** `Supplied`
- **Invariants:** INV-ACC-01, INV-ACC-02, INV-CUS-01, INV-ROUND-01
- **Attack vectors:** *first-depositor share inflation* → virtual shares (§3.2 of the economic model)
  plus never crediting unsolicited donations. *Rounding to mint excess shares* → `to_shares_down`.
  *`fee_position` substitution* → constrained to `PDA(market, market.fee_recipient)`.

## 13. `withdraw(assets: u64, shares: u128)`

- **Caller/signer:** `[S] lender` (owner of the position)
- **Accounts:** as `supply`, plus the vault must have liquidity.
- **Preconditions:** not paused (`WITHDRAW`); exactly one input non-zero; after computing `assets`,
  require `assets ≤ total_supply_assets − total_borrow_assets` (free liquidity) — else
  `InsufficientLiquidity` (E-05); require `shares ≤ position.supply_shares`.
- **State transition:** `accrue_mut`; `assets`-given → `shares = to_shares_up`; `shares`-given →
  `assets = to_assets_down`; decrement position and market.
- **Tokens:** `transfer_checked(loan_vault → lender_ata, assets)` signed by the market PDA.
- **Events:** `Withdrawn`
- **Invariants:** INV-ACC-03, INV-CUS-01, INV-ROUND-02
- **Attack vectors:** *withdrawing borrowed liquidity* → the free-liquidity check, which is exactly the
  vault-reconciliation identity. *Rounding to withdraw more than owned* → `to_shares_up`.

## 14. `borrow(assets: u64, shares: u128)`

- **Caller/signer:** `[S] owner`
- **Accounts:** `[R][PDA] protocol` · `[W][PDA] market` · `[W][PDA] position` · `[W][PDA] fee_position` ·
  `[W] loan_vault` · `[W] owner_loan_ata` · `[R] loan_mint` · `[R] loan_token_program` ·
  `[R] collateral_price_update` · `[R] loan_price_update`
- **Preconditions:** not paused (`BORROW`); `position.owner == owner`; oracle valid (**fail closed**);
  free liquidity ≥ `assets`; post-state `debt_value ≤ collateral_value · max_ltv / WAD`;
  post-state debt is `0` or `≥ min_debt` (E-25).
- **State transition:** `accrue_mut`; `assets`-given → `shares = to_shares_up`; increment
  `position.borrow_shares`, `total_borrow_shares`, `total_borrow_assets`.
- **Tokens:** `transfer_checked(loan_vault → owner_ata, assets)` signed by the market PDA.
- **Events:** `Borrowed`
- **Invariants:** INV-BOR-01..04, INV-SOLV-01, INV-ORA-01
- **Attack vectors:** *borrow beyond LTV* → post-state check with conservative prices. *Stale/wide
  oracle* → fail closed. *Draining lender liquidity* → free-liquidity check. *Dust-position spam to
  create unliquidatable debt* → `min_debt`.

## 15. `repay(assets: u64, shares: u128)`

- **Caller/signer:** `[S][W] payer` — **anyone may repay anyone's debt.**
- **Accounts:** `[R][PDA] protocol` · `[W][PDA] market` · `[W][PDA] position` · `[W][PDA] fee_position` ·
  `[W] loan_vault` · `[W] payer_loan_ata` · `[R] loan_mint` · `[R] loan_token_program`
- **Preconditions:** **unpausable**; **no oracle**; exactly one input non-zero;
  computed `shares ≤ position.borrow_shares` (clamp, E-06 — never pull more tokens than the debt).
- **State transition:** `accrue_mut`; `assets`-given → `shares = to_shares_down`; decrement position
  and market borrow totals by the credited amount.
- **Tokens:** `transfer_checked(payer_ata → loan_vault, assets)`; measured delta.
- **Events:** `Repaid`
- **Invariants:** INV-REP-01..03, INV-ADM-04, INV-ORA-02
- **Design note:** repay requires neither a signature from the position owner, an oracle, nor an
  unpaused market. **Debt repayment must never be blockable** — by the admin, by an oracle outage, or
  by anything else. This is a deliberate, load-bearing property (INV-ADM-04) and has its own
  adversarial test (`A-ADM-01`).
- **Attack vectors:** *over-repay to drain the payer* → clamped to actual debt. *Repay to grief a
  liquidator* → possible and harmless; it makes the position healthier.

## 16. `accrue_interest()`

- **Caller:** anyone; no signer required beyond fee payment.
- **Accounts:** `[W][PDA] market` · `[W][PDA] fee_position`
- **Preconditions:** none beyond account validity. `dt = 0` is a successful no-op (E-02).
- **State transition:** `accrue_mut`.
- **Events:** `InterestAccrued { interest, fee_amount, fee_shares, total_borrow_assets, total_supply_assets }`
- **Why it exists:** interest accrual is implicit inside other instructions, which makes it hard to
  observe and hard to test in isolation. A standalone permissionless instruction makes accrual
  independently testable, gives keepers a way to keep state fresh, and produces a clean event stream
  for off-chain indexing. It writes only two accounts and is cheap.
- **Invariants:** INV-ACC-04..08

---

## 17. `liquidate(repay_assets: u64, seize_collateral: u64)`

The most dangerous instruction in the protocol. Exactly one of the two inputs is non-zero; the other
is derived. (Supporting a `seize_collateral`-specified form lets a liquidator size the trade against
available swap liquidity, which matters for Phase 8.)

- **Caller/signer:** `[S][W] liquidator`
- **Accounts:**
  `[R][PDA] protocol` · `[W][PDA] market` · `[W][PDA] position` · `[W][PDA] fee_position` ·
  `[W] loan_vault` · `[W] collateral_vault` · `[W] liquidator_loan_ata` ·
  `[W] liquidator_collateral_ata` · `[R] loan_mint` · `[R] collateral_mint` ·
  `[R] loan_token_program` · `[R] collateral_token_program` ·
  `[R] collateral_price_update` · `[R] loan_price_update`
  *(14 accounts + program — comfortably within a v0 transaction; see the performance strategy.)*
- **Preconditions:** not paused (`LIQUIDATE`); oracle valid for **both** assets (fail closed);
  after `accrue_mut`, `HF < WAD` (**strict** — `HF == WAD` is not liquidatable, E-12);
  `repay_assets ≤ max_repay` per the close-factor and dust rules (economic-model §7.1).
- **State transition:** exactly economic-model §7.3.
- **Tokens:** two transfers — liquidator → `loan_vault` (repayment, measured delta), and
  `collateral_vault` → liquidator (`to_liquidator`), signed by the market PDA. `protocol_cut` stays
  in the vault and is recorded in `collateral_fee_accrued`.
- **Events:** `Liquidated { position, repay_assets, repay_shares, seized, to_liquidator, protocol_cut, hf_before, hf_after }`
- **Invariants:** INV-LIQ-01..08, INV-CUS-01, INV-CUS-02, INV-SOLV-02
- **Failure cases:** healthy position; oracle invalid; repay exceeds close factor; leaves dust debt;
  zero amounts; seizure exceeds collateral without the clamp path.
- **Attack vectors:**
  - *Liquidating a healthy position via a stale/manipulated price* → strict oracle validity, fail
    closed, confidence-adjusted conservative prices, and `max_conf_bps` rejection during volatility.
  - *Self-liquidation for profit* → possible and **permitted**; it is not an attack. A borrower
    liquidating themselves pays the bonus to themselves minus the protocol cut, which is never
    profitable versus simply repaying. Explicitly analyzed in the threat model (T-22) rather than
    blocked with a special case that would break legitimate liquidator bots.
  - *Seizing more than owed via rounding* → floor on seizure, ceil on the clamped repay.
  - *Bonus-driven death spiral* → the derived `LT·(1+b) < WAD` bound plus `full_liq_hf`.
  - *Griefing by repeated 1-unit liquidations* → `min_debt` dust rule and the fact that each
    liquidation must pay its own transaction cost while moving HF toward safety.
  - *Wrong collateral ATA* → `transfer_checked` with the pinned mint; the liquidator's ATA owner is
    not constrained (a liquidator may direct proceeds anywhere), which is intentional and safe since
    the amount is fully determined by protocol state.

## 18. `absorb_bad_debt()`

- **Caller:** anyone (permissionless).
- **Accounts:** `[W][PDA] market` · `[W][PDA] position` · `[W][PDA] fee_position`
- **Preconditions:** **no oracle, unpausable**; after `accrue_mut`,
  `position.collateral_amount == 0 && position.borrow_shares > 0`.
- **State transition:** economic-model §8.2 — protocol fee shares burned first, remainder socialized.
- **Tokens:** **none.** Loss recognition never moves tokens; the vault identity is preserved because
  both totals fall by the same amount.
- **Events:** `BadDebtAbsorbed { position, bad_assets, absorbed_by_protocol, socialized }`
- **Invariants:** INV-SOLV-03..05, INV-ACC-05
- **Design note:** requiring `collateral == 0` guarantees liquidators have already extracted every
  recoverable unit, so there is no discretion and nothing to front-run. Requiring **no oracle** is
  deliberate: an oracle outage must never prevent the protocol from recognizing a loss it has already
  taken.
- **Attack vectors:** *premature absorption to dilute lenders* → impossible while any collateral
  remains. *Skipping protocol first-loss* → `fee_position` is required and PDA-constrained.

## 19. `withdraw_collateral_fees(amount: u64)`

- **Caller/signer:** `[S] admin` (`== protocol.admin`)
- **Accounts:** `[R][PDA] protocol` · `[W][PDA] market` · `[W] collateral_vault` ·
  `[W] admin_collateral_ata` · `[R] collateral_mint` · `[R] collateral_token_program`
- **Preconditions:** `amount ≤ market.collateral_fee_accrued`
- **State transition:** `collateral_fee_accrued -= amount`
- **Tokens:** `transfer_checked(collateral_vault → admin_ata, amount)` signed by the market PDA.
- **Events:** `CollateralFeesWithdrawn`
- **Invariants:** INV-ADM-08, INV-CUS-02
- **Attack vectors:** *admin draining user collateral* → **structurally impossible**: the withdrawal
  is bounded by `collateral_fee_accrued`, which only ever increases inside `liquidate` by
  `protocol_cut`. This is the concrete mechanism behind NFR-9 ("the admin cannot move user funds") and
  has a dedicated adversarial test (`A-ADM-02`) that attempts to withdraw user collateral and must fail.

Loan-side protocol fees have **no** withdrawal instruction — the fee recipient calls `withdraw` like
any other lender. One fewer privileged code path.

---

## Cross-cutting checklist (every instruction must satisfy)

| Check | Enforcement |
|---|---|
| Signer where value leaves or risk increases | Anchor `Signer` + explicit `has_one = owner` |
| Account owner + discriminator | Anchor `Account<'info, T>` |
| Canonical bump | `seeds = [...], bump = acct.bump` |
| Relational consistency | `has_one` on `market`, `owner`, vaults, mints |
| Token program pinned | explicit equality vs `market.*_token_program` |
| Mint pinned | explicit equality + `transfer_checked` |
| No duplicate mutable accounts | Anchor 1.0 default; `dup` never used |
| Post-CPI reload before reading balances | manual `reload()` |
| Checked arithmetic | `overflow-checks = true` + `mul_div_*` |
| Fail-closed oracle | shared `require_valid_price` guard |
| Pause respected (except the unpausable set) | shared guard |
| Event emitted | one per state transition |
