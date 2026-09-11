# `example-liquidator`

A **lab/example** program demonstrating the Aegis Phase 8 liquidation-callback interface
(`docs/instruction-catalogue.md` §17, `docs/composability.md`, ADR-0013). It is deliberately
minimal: a fixed, caller-supplied exchange rate against its own pre-funded reserve.

**This is not production exchange infrastructure.** It has no price discovery, no order book, no
slippage protection, and no liquidity beyond whatever its own reserve holds. Its only purpose is to
prove the composability path end-to-end, fully offline and fully deterministic
(`I-LIQ-CB-01`, `crates/aegis-test-kit/examples/phase8_demo.rs`).

## What it does

`handle_liquidation(rate_wad: u128)`:
1. Measures the collateral it actually received (`collateral_account.amount`) — never trusts an
   amount from instruction data.
2. Computes `output = seized * rate_wad / (WAD * decimals_adjustment)` — a fixed WAD-scaled price,
   supplied entirely by whoever calls Aegis's `liquidate` (Aegis itself never inspects or
   constrains this program's instruction data).
3. Requires its own `loan_reserve` to hold at least that much.
4. Transfers the computed amount from `loan_reserve` into Aegis's `loan_vault`, signed by this
   program's own PDA (`seeds = [b"authority"]`) — never anyone else's authority.

## Account contract (must match exactly, positionally)

Aegis's callback CPI (`build_callback_instruction` in `programs/aegis/src/instructions/liquidate/
liquidate.rs`) supplies, in order:

1. `collateral_account` (mut) — owned by this program's `authority` PDA; already funded with the
   seized collateral by the time this instruction runs.
2. `loan_vault` (mut) — Aegis's vault; this program must end by transferring the repayment here.
3. `collateral_mint` (readonly)
4. `loan_mint` (readonly)
5. `collateral_token_program` (readonly)
6. `loan_token_program` (readonly)

Then, forwarded verbatim from whatever the liquidator supplied as `remaining_accounts` on the
*outer* `liquidate` call:

7. `authority` — this program's own PDA, `seeds = [b"authority"]`.
8. `loan_reserve` (mut) — this program's own pre-funded loan-asset reserve, owned by `authority`.

## What Aegis does NOT trust this program for

Nothing. Aegis never assumes this program behaves honestly, returns expected data, repays, or does
not reenter — it re-reads `loan_vault` after the CPI and measures the actual delta
(`docs/composability.md`). This program happening to be "the honest one" in tests is a property of
*this specific program*, not something Aegis relies on structurally.

## Setting up the reserve (test/demo fixtures)

`collateral_account` and `loan_reserve` are plain SPL token accounts owned by the `authority` PDA,
created and funded the same way any test fixture creates a token account — there is no
`initialize`/`fund` instruction on this program itself; keeping it to one instruction is
deliberate. See `crates/aegis-test-kit/examples/phase8_demo.rs` for a complete, working example.
