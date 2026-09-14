# R-3 — Bad debt event

**Trigger:** `governance.md` §7 — one or more positions with `collateral_amount == 0` and
`borrow_shares > 0` (collateral exhausted, debt outstanding — `economic-model.md` §8.1's five
named mechanisms).

## Prerequisites

- None to *recognize* the loss — `absorb_bad_debt` is permissionless and needs no privileged key.
- Admin access is useful only for the *review* step afterward, not the action itself.

## Step 1 — Confirm the position is genuinely eligible

```ts
// Fetch the position; confirm BOTH:
//   position.collateral_amount === 0   (exactly zero -- no dust tolerance, INV-SOLV-03)
//   position.borrow_shares > 0
```

If `collateral_amount > 0`, the position is not yet eligible — a liquidator has not (or cannot
yet) extract the remaining value; do not call `absorb_bad_debt` prematurely (it will simply fail
with `BadDebtRequiresZeroCollateral`, but confirming first avoids a wasted transaction and a
confusing incident log entry).

## Step 2 — Immediate: anyone calls `absorb_bad_debt`

**No pause action is needed or appropriate here.** `absorb_bad_debt` is unpausable by design
(INV-ADM-04) and needs no oracle — loss recognition must never be blocked by the thing that may
have caused it.

```ts
const ix = buildAbsorbBadDebtInstruction(AEGIS_PROGRAM_ADDRESS, {
  market: marketAddress,
  position: positionAddress,
  feePosition: feePositionAddress, // PDA(market, market.fee_recipient) -- derive, do not guess
}); // no instruction args -- absorb_bad_debt takes none
```

Anyone may submit this transaction — a keeper, the protocol team, or an affected lender. There is
no privileged path and none should be added (`governance.md` §8: "an emergency fund migration...
instruction" is explicitly rejected; this is the honest alternative — permissionless, immediate
loss recognition instead of a rescue).

## Step 3 — Post-action verification

- Confirm the `BadDebtAbsorbed` event: `bad_assets`, `absorbed_by_protocol` (fee shares burned
  first, INV-SOLV-06), `socialized` (the remainder, spread pro-rata across remaining lenders).
- Re-fetch the position: `borrow_shares` must now be exactly `0`.
- Re-fetch the market: confirm `loan_vault.amount == total_supply_assets - total_borrow_assets`
  still holds exactly (INV-SOLV-04) — no tokens moved, both totals fell by the same amount.

## Step 4 — Then: publish and review

- **Publish the size and cause.** State `bad_assets`, which of `economic-model.md` §8.1's five
  mechanisms applied (gap risk, unprofitable liquidation, oracle outage, dust, frozen collateral),
  and the market affected. Losses are isolated to their originating market (`I-ISO-01`) — say so
  explicitly, so lenders in other markets are not left wondering if they are exposed.
- **Verify protocol fee shares absorbed first.** `absorbed_by_protocol` in the event should be
  `min(bad_assets, fee_position's assets at the time)` — confirm this against the event rather than
  assuming it.
- **Review whether market parameters contributed.** Was `max_ltv`/`liq_threshold` too permissive
  for the collateral's actual volatility? Was `liq_bonus` too thin to attract a liquidator in time?
  Was `min_debt` too low, allowing a dust position to survive uncollected? This review feeds a
  possible R-5-style parameter tightening — via `set_market_params`, applied immediately since
  tightening is never timelocked (`governance.md` §4) — not an emergency action of its own.

## Recovery

There is no "recovery" step for the affected debt itself — the loss is recognized, not reversed.
Recovery, if any, is a subsequent, ordinary parameter review (see Step 4) to reduce recurrence.

## Logging / evidence

Record: the position, `bad_assets`, the mechanism from §8.1, the transaction signature, the
`BadDebtAbsorbed` event's exact field values, and the post-hoc parameter review's conclusion (even
if "no change warranted").

## Escalation

If bad debt recurs repeatedly in the same market, or at a scale that raises solvency concerns for
that market's remaining lenders, escalate to a parameter review (R-5) or a decision to pause
`SUPPLY`/`BORROW` on that specific market while the parameters are reconsidered — a deliberate,
reviewed action, not an automatic consequence of this runbook.

## Do not

- Do not wait for admin approval before calling `absorb_bad_debt` — it is permissionless precisely
  so that no one has to.
- Do not pause anything as a reflexive response to a bad debt event; the correct immediate action
  is recognition, which nothing blocks.
- Do not attempt to make lenders whole administratively — no such instruction exists
  (`governance.md` §8), and building one now, under incident pressure, is exactly how a
  non-custodial protocol becomes custodial by accident.
