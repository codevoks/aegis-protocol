# ADR-0013 — Liquidation callback: account contract, custody model, and reentrancy guard

**Status:** Accepted · **Date:** 2026-09-12 · **Phase:** 8

## Context

`docs/composability.md` (Phase 0, frozen) already commits Aegis to an **optional** `liquidate`
callback: after seizing collateral, Aegis CPIs into a liquidator-specified, **untrusted** program so it
can swap the collateral and fund the repayment in the same transaction. That document fixes the
security *properties* (no signer forwarded, full post-CPI state re-read, post-condition
re-verification, opt-in, a state-machine guard) but not the exact account layout, and RV-6 (`docs/
ecosystem-research.md` §16.1) confirms the Solana runtime already blocks indirect `A → B → A` CPI
reentrancy on its own — which by the same document's own rule must not become a reason to skip a
protocol-level guard.

Three concrete design questions had to be answered before writing code, none of which any frozen
document settles at the account level:

1. Where does the reentrancy-guard bit live?
2. What token account does the callback actually receive seized collateral into, and how does it repay
   without ever holding the liquidator's or the market's signing authority?
3. Is a global lock acceptable, given the phase spec's explicit "no unnecessary global lock" and
   "different markets should retain parallelism" requirements?

## Decision

**1. The guard is one new `Market` field, taken from `_reserved`.**

`pub liquidation_guard: u8` is added to `Market`, and `_reserved` shrinks from `[u8; 64]` to
`[u8; 63]`. `Market::LEN` is unchanged at 640 bytes — no realloc, no migration instruction, which is
exactly the purpose `_reserved` was carrying per `INV-UPG-02`. `docs/account-model.md` §4 is updated in
this same commit.

The existing `flags: u8` field was considered and rejected as the host for this bit (see Alternatives).

**2. The callback receives seized collateral into an account it controls, and repays `loan_vault`
directly, using its own authority.**

Aegis transfers `outcome.to_liquidator` from `collateral_vault` into a liquidator-supplied
`callback_collateral_account` (mint-checked, ownership unconstrained — the callback's own business),
signed by the `Market` PDA exactly as every other vault outflow already is. Aegis then CPIs into the
callback with **that account, `loan_vault`, both mints, and both token programs** — nothing else of
Aegis's own, plus whatever liquidator-supplied `remaining_accounts` the callback's own swap needs
(rejected outright if any of them alias a protected Aegis key — see `error.rs`'s
`CallbackAccountNotPermitted`). The callback is expected to end by transferring the required loan
asset into `loan_vault` using **its own** authority (typically its own PDA via `invoke_signed`, which
is the callback's CPI, not Aegis's).

Neither `market` nor `liquidator`'s `AccountInfo` is ever included in the callback's account list, and
the dispatch uses plain `invoke()`, never `invoke_signed`. This makes "no signer forwarded" a
structural fact about which accounts appear in one `Vec<AccountMeta>`, checkable by a unit test against
that exact list (`INV-AUTH-07`/`A-AUTH-07`), rather than a policy someone could get wrong under future
changes.

**3. The measured repayment must meet or exceed the required amount — never match it exactly.**

`actual_delta >= outcome.repay_assets`, not `==`. A real external swap essentially never lands on
the exact base-unit amount required (slippage, rounding, a DEX quoting in whole-token increments);
requiring exact equality would make the callback unusable for any real integration. Any surplus
becomes ordinary, unaccounted `loan_vault` surplus — exactly the same shape as an unsolicited
direct donation (INV-CUS-08) — never credited beyond `outcome.repay_assets`.

This is a **deliberate, documented exception to INV-CUS-01's exact-equality form**
(`docs/invariants.md` §B), recorded there rather than silently accepted: a callback branch that
overpays leaves `loan_vault.amount > total_supply_assets − total_borrow_assets`, which the
exact-equality reading of INV-CUS-01 would flag as a violation. The invariant's *security-relevant*
direction — the vault is never short of what accounting requires — holds unconditionally in both
branches; only the "never more than expected" direction is relaxed, and only for the callback
branch specifically. The no-callback branch and every other instruction retain the invariant's
original, exact form with no exception (`I-LIQ-CB-02`, `tests/phase6_liquidation.rs`).

**4. The guard is per-`Market`, checked at the top of `liquidate` unconditionally, set only on the
callback branch.**

`require!(market.liquidation_guard == 0, LiquidationCallbackReentrancy)` runs before any other logic,
in both branches. Only the callback branch ever sets it to `1` (immediately before the CPI) and clears
it to `0` (immediately after, on the success path only — a failure anywhere reverts the whole
transaction atomically, so a "stuck" guard is not reachable). No `Protocol`-level or cross-market state
is touched.

## Alternatives considered

**Reuse a spare bit in `Market.flags`.** Rejected. `flags` is documented and used as a permanent,
set-once-at-`create_market` record of mint properties (`FLAG_ACK_FREEZE_AUTHORITY`,
`FLAG_COLLATERAL_HAS_TRANSFER_FEE`), read by the Token-2022 policy code. A transient, per-instruction
runtime lock has a completely different lifetime; packing it into the same byte would make `flags`
ambiguous to read and easy to reason about incorrectly in a future phase.

**A separate guard PDA account.** Rejected. `Market` is already a mutable account in every `liquidate`
call; a second account would add rent and an extra account slot for a single bit, with no benefit over
using `_reserved`.

**A global `Protocol`-level lock.** Rejected outright. `Protocol` is read-only in every user
instruction today (a deliberate parallelism property, `NFR-7`/`ADR-0004`). Making it writable during
any market's liquidation callback would serialize liquidations across the *entire* protocol — a
liquidation on a slow, low-value market would block every other market's liquidations for the duration
of an untrusted external CPI. This directly violates the phase spec's explicit requirement and would be
a severe, self-inflicted denial-of-service surface.

**Rely on the runtime's own reentrancy protection instead of a protocol guard, given RV-6 confirms
indirect `A → B → A` is already rejected.** Rejected — stated as a hard rule in `docs/composability.md`
§2 and repeated in the phase spec: Aegis's security must not depend on the answer to RV-6, because the
runtime's behavior is not a document Aegis controls or can pin a version against the way it pins
Anchor or a dependency crate.

**Route seized collateral through the existing `liquidator_collateral_ata` plus an SPL `Approve`
delegate to the callback's PDA**, rather than a dedicated callback-owned account. Considered because it
would keep `liquidate`'s existing 14-account shape for the seizure leg. Rejected: it requires the
liquidator to construct and include an extra `approve` instruction ahead of `liquidate` in the same
transaction, adds an SPL Token delegate-authority code path this repository has never needed, and buys
nothing over having the callback simply own its own receiving account outright — which needs no
delegate mechanics at all and keeps the callback's authority story to "the callback signs for its own
PDA," full stop.

## Consequences

**Positive**
- `INV-RES-03` (no writable account shared between two markets) and cross-market parallelism
  (`A-PAR-02`) are untouched — the guard lives per-`Market`, and nothing protocol-wide is written.
- "No signer forwarded" is provable by inspecting one constructed `Vec<AccountMeta>`, not by auditing
  every call site for a mistake that could creep in later.
- `Market::LEN` does not change, so no account migration is required to ship this phase.
- The custody surface (`account-model.md` §6.3) gains exactly one new *destination* for an existing
  signer path (`collateral_vault → callback_collateral_account`, market PDA signs) rather than a new
  kind of authority.

**Negative**
- The callback interface asks the callback author to manage their own PDA-owned token accounts (a
  collateral-receiving account, and whatever holds the swap output before the final transfer into
  `loan_vault`), rather than Aegis handling custody on their behalf. This is documented plainly in the
  callback interface docs (`docs/instruction-catalogue.md` §17) so a third-party integrator is not
  surprised by it.
- One byte of `Market._reserved` is permanently spent; 63 bytes remain for future additive migration.

**Requirements this imposes**
- `A-CPI-01..04` must assert **atomic rollback** (zero state diff on failure), not merely "the
  transaction returned an error" — the guard's non-stuck property depends entirely on Solana's
  transaction atomicity, and that dependency must be exercised by a real reverted transaction, not
  assumed.
- Any future instruction that also touches `Market.liquidation_guard` must preserve the invariant that
  it is `0` outside the callback CPI's exact lifetime.
