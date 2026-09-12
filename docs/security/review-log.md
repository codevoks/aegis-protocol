# Manual Security Review Log (Phase 10)

Systematic manual review of the checklist `docs/phases/phase-10-security.md` #42 requires, read
directly against the current source (not from memory or a prior session). Each row states what was
checked, where, and the concrete conclusion — not generic prose.

| Area | Checked | Location | Conclusion |
|---|---|---|---|
| Account constraints | Every `Account`/`InterfaceAccount` field declares owner/discriminator validation implicitly via Anchor's typed wrappers; every PDA declares `seeds =`/`bump =` against a stored canonical bump, never a caller-supplied bump | `programs/aegis/src/instructions/**/*.rs` (all 13 instruction files) | Consistent. No `UncheckedAccount` appears except the two Pyth price-update fields (`collateral_price_update`/`loan_price_update` in `borrow.rs`, `withdraw_collateral.rs`, `liquidate.rs`) and `callback_program` in `liquidate.rs` — all three are field-by-field validated in code immediately after (`oracle::require_valid_price`, `.executable` check), documented with a `/// CHECK:` comment explaining exactly why the type-level check is skipped, per Anchor convention |
| Signer boundaries | Cross-checked `account-model.md` §5.1's signer table against every `#[derive(Accounts)]` struct's `Signer<'info>` fields | `supply.rs`, `withdraw.rs`, `borrow.rs`, `withdraw_collateral.rs`, `close_position.rs` require `owner: Signer`; `repay.rs`, `deposit_collateral.rs`, `absorb_bad_debt.rs`, `accrue.rs` declare no owner-signer field; `liquidate.rs`'s `liquidator: Signer` is explicitly not constrained to the position owner (self-liquidation permitted, `U-LIQ-07`) | Matches the documented asymmetry exactly — verified field-by-field, not merely by re-reading the doc |
| PDA seeds | Every seed list matches `account-model.md` §3-6 exactly: `[b"protocol"]`, `[b"market", collateral_mint, loan_mint, config_id_le]`, `[b"position", market, owner]`, `[b"cvault"/"lvault", market]` | `programs/aegis/src/constants.rs` + every `#[account(seeds = [...])]` site | No seed drift found. `fee_position` is derived at `PDA(market, market.fee_recipient)` everywhere it appears (5 instructions), never re-derived with a different owner key |
| Vault ownership | `collateral_vault`/`loan_vault` are `InterfaceAccount<TokenAccount>` with `address = market.collateral_vault`/`market.loan_vault` (defense in depth: PDA derivation *and* stored-pubkey equality, per `account-model.md` principle 5) | `deposit_collateral.rs`, `withdraw_collateral.rs`, `supply.rs`, `withdraw.rs`, `borrow.rs`, `repay.rs`, `liquidate.rs` | Both checks present at every one of the 7 vault-touching instructions — no instruction relies on seed derivation alone |
| Token-program validation | Every instruction that touches a token account pins `market.collateral_token_program`/`loan_token_program` via `require_keys_eq!` before any transfer | Same 7 files as above | Confirmed at every site; `deposit_collateral.rs`'s comment explicitly names this as the T-05/T-11/INV-CUS-07 defense |
| Measured-delta paths | `transfer_checked_in` is the *only* function that credits an inbound transfer, and it always uses `after - before` post-`reload()`, never the requested `amount` | `programs/aegis/src/token/transfer.rs` | One call site, one implementation — no duplicated/hand-rolled delta logic elsewhere to drift out of sync. Grep-confirmed: `transfer_checked_in` is the only crediting path in `programs/aegis/src` |
| Accounting totals | Every `total_supply_assets`/`total_borrow_assets`/`*_shares` mutation site was read directly (not inferred) | `supply.rs`, `withdraw.rs`, `borrow.rs`, `repay.rs`, `liquidate.rs::apply_liquidation_accounting`, `absorb_bad_debt.rs`, `state/market.rs::accrue_mut` | Every asset-side mutation has a paired share-side mutation in the same instruction (the exact pairing this phase's mutation validation exercised one-by-one) |
| Rounding direction | Cross-checked `economic-model.md` §1.3's 14-row rounding-law table against `to_shares_up`/`to_shares_down`/`to_assets_up`/`to_assets_down` call sites | `supply.rs` (down/up), `withdraw.rs` (up/down), `borrow.rs` (up/up), `repay.rs`, `liquidate.rs` | Every call site uses the documented direction; `crates/aegis-math/tests/rounding_law.rs`'s 15 `U-ROUND-*` tests already pin this numerically, this pass confirms the *call sites* match, not just the primitives |
| Oracle validation order | `require_valid_price` must run before any state mutation (INV-ORA-07) in every priced instruction | `borrow.rs` (line ~95, before `accrue_mut`), `withdraw_collateral.rs` (debt-bearing branch), `liquidate.rs` (before `accrue_mut`) | Confirmed in all three; `liquidate.rs`'s own doc comment states this explicitly and `A-ORACLE-13` tests it with byte-exact before/after snapshots |
| Stale/confidence checks | O-1..O-11 implemented as one function, called uniformly | `programs/aegis/src/oracle/mod.rs::require_valid_price` → `oracle::pyth::PythPull::read_price` | Single implementation, three call sites, no duplicated/divergent copy |
| Interest accrual | `accrue_view`/`accrue_mut` split (pure view vs. mutating), `dt` clamped to `>= 0`, fee-share minting uses `to_shares_down` | `programs/aegis/src/state/market.rs` | As documented; `P-ACCRUE-1` (view/mut agreement) and `P-ACCRUE-2` (free-liquidity invariance) already pin this. This phase's own INV-ACC-04 mutation (add interest to borrow but not supply) confirms the fuzzer independently detects a violation here too, not only the property test |
| Health/LTV | `is_within_max_ltv` and `health_factor` both take already-conservatively-valued `collateral_value`/`debt_value` (never a raw price) | `crates/aegis-math/src/health.rs`, called from `borrow.rs`/`withdraw_collateral.rs`/`liquidate.rs` | Single implementation, three call sites, no divergence. This phase's INV-SOLV-01 mutation (skip the post-borrow check) was caught by the fuzzer's own *independent* re-derivation of this same math (`assert_inv_solv_01`), not by trusting the program's result |
| Liquidation | `HF < WAD` checked strictly (`is_liquidatable`, never `<=`); seizure/repay clamped by `aegis-math::liquidation`'s pure functions, never re-implemented inline | `liquidate.rs` | Confirmed no inline duplication of the clamp math; `U-LIQ-01..07`/`P-LIQ-1..4` already pin the math itself |
| Bad debt | `absorb_bad_debt` requires exact-zero collateral (no dust tolerance), reads no oracle, moves no tokens | `absorb_bad_debt.rs` | Confirmed structurally (no price-update field in the `Accounts` struct at all — not merely unchecked, absent). This phase's INV-SOLV-04 mutation and the dedicated `mutation_probe_bad_debt` fuzzer probe both independently confirm the accounting pairing here |
| Token-2022 policy | Positive allowlist, fail-closed default arm | `programs/aegis/src/token/policy.rs::evaluate_mint` | Confirmed: the `match` statement's only catch-all arm returns `Err`, and it is placed *after* every explicitly-allowed variant — a `Tier A`/`Tier B` extension added to `spl-token-2022-interface` in the future without updating this match will still be rejected, not silently accepted, which is the intended fail-closed property |
| CPI callback | No signer forwarded, `invoke` (never `invoke_signed`), reentrancy guard, protected-key aliasing check | `liquidate.rs::build_callback_instruction`, `Market::liquidation_guard` | Every `AccountMeta` the callback receives is unconditionally `is_signer: false` at construction (a property of the function's *output*, not caller discipline); this phase re-confirmed by reading the function body directly rather than trusting the existing `A-AUTH-07` test's own description of it |
| Post-CPI reload | Every vault read after a CPI reloads first | `token/transfer.rs::transfer_checked_in`, `liquidate.rs`'s callback branch (`loan_vault.reload()` before measuring the repayment delta) | Confirmed at both sites; the callback branch is the one place a *second*, later reload matters (after the callback's own CPI, not just after Aegis's own transfer), and it is present |
| Admin authority | `Protocol.admin`-gated instructions use `has_one = admin`; `withdraw_collateral_fees` cannot exceed `collateral_fee_accrued` | `admin/*.rs` | Confirmed; `A-ADM-02` already tests the fee-withdrawal bound with both an off-by-one and a whole-vault-balance attempt |
| Pause behavior | `repay`, `deposit_collateral`, `absorb_bad_debt`, `close_position` structurally cannot be paused (Phase 12 has not added pause bits yet) | Grepped `programs/aegis/src` for `paused`/`pause` | Zero matches outside `state/protocol.rs`'s field definition and `account-model.md`'s doc comments — confirms no instruction currently reads a pause bit at all, which is the documented, correct pre-Phase-12 state (a check today would be untestable dead code, per `supply.rs`'s own comment) |
| Resource / account-size risks | No unbounded loop; the only loop over caller-controlled length is `liquidate.rs::build_callback_instruction`'s `for acc in remaining_accounts` | Grepped for `for `/`while `/`loop` in `programs/aegis/src` | The one caller-length-bounded loop is inside the *optional*, opt-in callback branch, bounded by the transaction's own account-count limit (a Solana-wide, not Aegis-specific, ceiling) — not a new unbounded-DoS surface. Full CU quantification remains Phase 11 scope (`INV-RES-01`), consistent with `docs/invariants.md`'s own phase assignment |
| SDK/client assumptions | Spot-checked `sdk/ts/src/math.ts` for `Number` usage in a protocol-critical path (a `Number`-based WAD computation would silently lose precision past 2^53) | `sdk/ts/src/math.ts` | `bigint` throughout — confirmed by `docs/project-status.md`'s own Phase 9 record ("grep-verified to contain no ... `Number` in any protocol-critical path"); this phase re-ran the same grep independently rather than trusting the prior record verbatim: zero matches |

## Panic search (`phase-10-security.md` #50)

Grepped `programs/aegis/src` for `.unwrap()`, `.expect(`, and bare array indexing (`[` on a
non-constant index) outside `#[cfg(test)]` blocks.

- `.unwrap()` / `.expect(`: zero matches in production code paths. Every fallible operation uses
  `?`, `.map_err(AegisError::from)?`, or an explicit `require!`.
- Indexing: the only non-constant index patterns found are slice accesses on `_reserved: [u8; N]`
  arrays during zero-initialization (bounded by the fixed array size, not caller input) and
  Anchor-macro-generated account deserialization internals (outside this program's own code).
- No malformed-account-length or invalid-enum-parsing panic surface was found reachable from a
  transaction's own instruction data or account list — every account is either an Anchor-typed
  `Account`/`InterfaceAccount` (which returns a `Result`, never panics, on a malformed buffer) or
  one of the two `UncheckedAccount` oracle fields, which are deserialized via
  `PriceUpdateV2::try_deserialize` (also `Result`-returning; a malformed buffer surfaces as
  `OracleAccountInvalidData`, tested by the O-2 adversarial cases).

## Scope note

This review is a fresh, direct reading of the current source performed for Phase 10 — it does not
merely restate what earlier phases' own doc comments already claim about themselves. Where this
review's conclusion matches an earlier phase's existing claim, that is stated as independent
confirmation, not as inherited certainty.

**Honesty note on limits:** this manual review's "Accounting totals" and "Rounding direction" rows
(above) read every share/asset mutation site and confirmed each asset-side change has a paired
share-side change — true, and still worth recording — but manual review did **not** independently
predict `docs/security/findings.md`'s F-10-02 (the `repay` dust-stranding bug found by the fuzzer's
extended campaign). The bug is real regardless of the pairing being present at every site: both
totals *are* updated together in the ordinary case, but the specific values computed for one of
that pair, at one specific tiny-magnitude boundary, can drift apart. A checklist-style manual
review that confirms *structural* pairing is not equivalent to an exhaustive numeric analysis of
every rounding boundary, and this line records that gap honestly rather than implying the manual
pass would have caught what the fuzzer found.
