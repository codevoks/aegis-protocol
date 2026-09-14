# R-5 — Hostile market parameters detected

**Trigger:** `governance.md` §7 — a market created, or updated via `set_market_params`, with
dangerous-but-*legal* parameters (they pass the on-chain bounds checks, `economic-model.md` §5, but
are unwise: e.g. `max_ltv` near `liq_threshold`, a thin `liq_bonus` unlikely to attract liquidators,
or a `min_debt` too low for the asset's likely price volatility). Also applies to a market whose
mint later reveals unexpected behavior within its acknowledged risk (e.g. a freeze authority
exercised, per `ack_freeze_authority` — T-26).

## Prerequisites

- The **guardian** keypair, to pause the *specific market* (not the whole protocol).
- The **admin** keypair, for the eventual parameter correction.

## Step 1 — Confirm the market, and only that market, is affected

**Isolation means no other market shares this risk** (ADR-0004) — confirm the concern is genuinely
scoped to one market's own parameters or mint behavior before treating it as protocol-wide. A
parameter that looks aggressive in isolation (e.g. a 20% `liq_bonus`) may be a deliberate,
documented choice for a specific volatile asset pair, not itself evidence of a problem — cross-check
against `MarketCreated`'s original event and any subsequent `MarketParamsUpdated`/
`StagedParamsCommitted` history for that market before concluding it is "hostile."

## Step 2 — Immediate: guardian pauses that market only

```ts
const ix = buildSetMarketPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: guardianKeypair.address, protocol: protocolAddress, market: affectedMarketAddress },
  { flags: currentMarketPausedBits | PAUSE_ALL_BITS }, // pause this market fully; others are untouched
);
```

Using `set_market_pause` (not `set_protocol_pause`) is the entire point of this runbook: **no other
market is affected.** Verify this directly, not just by intent — fetch an unrelated market's
`paused` field and confirm it is unchanged.

## Step 3 — Post-action verification

- Re-fetch the affected market; confirm `paused == PAUSE_ALL_BITS` (or the subset chosen, if a
  narrower pause is judged sufficient — see the note below).
- Re-fetch at least one other market; confirm its `paused` field is untouched.
- Confirm `repay`/`deposit_collateral`/`absorb_bad_debt`/`close_position` remain callable on the
  affected market (INV-ADM-04 — existing positions must still be able to reduce risk or exit).

*Narrower option:* if the concern is specifically about new exposure growing (e.g. an
under-collateralized `max_ltv`), pausing only `SUPPLY` and `BORROW` on that market — leaving
`WITHDRAW` and `LIQUIDATE` active — may be sufficient and less disruptive to existing users. Choose
the narrowest pause that addresses the specific concern; default to full-market pause only when the
risk is not yet well enough understood to scope more precisely.

## Step 4 — Review and correct

- Determine the specific parameter(s) of concern and the intended correction.
- **Tightening applies immediately** (`governance.md` §4: lower `max_ltv`, lower `liq_threshold`,
  raise `min_debt`, lower `max_price_age_secs`/`max_conf_bps`, lower `fee`) — use this path for the
  actual fix once identified:

```ts
const fix = buildSetMarketParamsInstruction(
  AEGIS_PROGRAM_ADDRESS,
  {
    admin: adminKeypair.address,
    protocol: protocolAddress,
    market: affectedMarketAddress,
    feePosition: feePositionAddress,
    pendingMarketParams: pendingMarketParamsAddress, // derived PDA; unused on the tightening path but always required
    systemProgram: SYSTEM_PROGRAM_ADDRESS,
  },
  { /* the FULL parameter set (not a delta) with ONLY the risk-reducing field(s) changed vs. current */ },
);
```

- If the correction is instead risk-*increasing* by this table's classification (raising
  `max_ltv`/`liq_threshold`, lowering `min_debt`, etc.) — this should be rare for a *correction* to
  a hostile-parameters incident, and worth pausing on: a fix to a "too risky" market should almost
  always be a tightening, not a loosening. If a loosening genuinely is the correct fix, it is
  timelocked exactly as any other loosening would be (`governance.md` §4) — **do not treat this
  runbook as grounds to bypass the timelock.**

## Recovery

Unpause the market (admin only) once the parameter correction is confirmed on-chain and, if it
required an unusual review before applying, once that review is documented.

```ts
const unpause = buildSetMarketPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: adminKeypair.address, protocol: protocolAddress, market: affectedMarketAddress },
  { flags: 0 },
);
```

Verify: re-fetch, confirm `paused == 0`, confirm the corrected parameters are in effect
(`market.max_ltv` etc. match the intended values), and confirm a real `supply`/`borrow` now
succeeds under the corrected parameters.

## Logging / evidence

Record: the specific parameter(s) or mint behavior of concern, why it was judged hostile-but-legal
rather than a bounds-check gap (if it were a bounds-check gap, that would be a different, more
serious finding — see R-2), the pause transaction signature, the correction transaction signature
(and its `MarketParamsUpdated`/`ParamsStaged` event), and the unpause transaction signature.

## Escalation

If the same class of parameter mistake recurs across multiple market creations, escalate to a
review of the `create_market` review process itself (an operational gap, not a code gap — the
on-chain bounds are deliberately wide enough to admit legal-but-unwise configurations, per
`economic-model.md` §5's own design).

## Do not

- Do not use `set_protocol_pause` when the issue is scoped to one market — that needlessly blocks
  every other market's users (violates the isolation property this runbook exists to preserve).
- Do not bypass the timelock for a loosening "fix" under the theory that this incident justifies it
  — see Step 4.
- Do not treat every aggressive-looking parameter as automatically hostile; confirm against the
  market's own documented creation intent first (Step 1).
