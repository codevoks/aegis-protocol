# R-1 — Oracle degradation

**Trigger:** `governance.md` §7 — Pyth feeds stale or confidence persistently above
`market.max_conf_bps` for one or more markets. Detected via off-chain monitoring of the feeds Aegis
consumes (Hermes/Pyth network status), not via an on-chain alert (Aegis has none).

## Prerequisites

- Read access to the affected market's on-chain state (`fetch_market`, or `getAccountInfo` +
  the SDK's decoder) to confirm `max_price_age_secs`/`max_conf_bps` and which feeds are configured.
- If pausing: the **guardian** keypair (can set pause bits; cannot clear them — this is
  intentional, see `governance.md` §1).
- If unpausing: the **admin** keypair (only the admin may clear a pause bit).

## Step 1 — Verify before assuming

**Do not act on the trigger alone.** Aegis already fails closed for `borrow`, the debt-bearing
path of `withdraw_collateral`, and `liquidate` (O-5, `oracle-design.md` §2) — a genuinely stale or
wide-confidence feed already blocks risk-increasing operations on its own, without operator
intervention. Verify this is actually happening before pausing anything:

```ts
// Attempt (or observe a user's recent attempt at) a borrow against the affected market's feeds.
// A stale/wide-confidence price should already be rejected with OraclePriceStale /
// OracleConfidenceTooWide -- confirm this is the observed on-chain behavior, not merely the
// off-chain feed status, before deciding pausing adds anything.
```

If the fail-closed behavior is already working as designed, the incident may not need a pause at
all — see "Decide," below.

## Step 2 — Decide

| Condition | Action |
|---|---|
| Feed merely stale/wide-confidence, fail-closed already blocking `borrow`/debt-bearing `withdraw_collateral`/`liquidate` | **No pause needed.** Monitor; document; wait for the feed to recover. Risk-reducing operations (`repay`, `deposit_collateral`, debt-free `withdraw_collateral`, `absorb_bad_debt`) remain open by design and should **not** be blocked. |
| Feed degradation is prolonged and new borrowing against a decaying position is a specific concern | Guardian pauses **`BORROW`** only, for the affected market. |
| Price is suspected **wrong**, not merely stale (e.g., a publisher outage producing a plausible-looking but incorrect value that somehow passes O-1..O-10) | Escalate immediately — this is a T-06/T-20-class concern beyond this runbook's scope; do not assume O-checks alone are sufficient in this specific case. |

**Do not** pause `LIQUIDATE` for ordinary staleness. A stale price already blocks `liquidate` on
its own (it needs a valid price for both assets); pausing `LIQUIDATE` on top of that has no
protective effect and, if the guardian later cannot tell staleness from a resolved outage in time,
risks leaving genuinely liquidatable positions unliquidated once the feed recovers and the pause
has not yet been lifted.

## Step 3 — Action (if pausing `BORROW` for one market)

```ts
const ix = buildSetMarketPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: guardianKeypair.address, protocol: protocolAddress, market: affectedMarketAddress },
  { flags: currentMarketPausedBits | PAUSE_BORROW }, // 0b0010 -- OR in, never overwrite other bits
);
// sign with guardianKeypair, submit, await confirmation
```

Read `market.paused` **before** constructing `flags` — the guardian may only ever set bits, so the
new value must be the old value with `PAUSE_BORROW` added, never a bare `0b0010`.

## Step 4 — Post-action verification

- Re-fetch the market account; confirm `paused & PAUSE_BORROW != 0`.
- Confirm the `MarketPauseSet` event landed at the expected slot.
- Attempt (in a controlled way, or observe) a real `borrow` against the market; confirm it now
  fails with `OperationPaused`, not merely the oracle error from Step 1.
- Confirm `repay`, `deposit_collateral`, `absorb_bad_debt`, and `close_position` are **still**
  callable against this market — they must be, structurally (INV-ADM-04); if any of them is
  blocked, that is a separate, more serious incident (escalate as R-2).

## Recovery

Unpause only after feeds are fresh (age comfortably under `max_price_age_secs`) and confidence is
normal (comfortably under `max_conf_bps`) for a **sustained window**, not the first fresh tick.

```ts
const ix = buildSetMarketPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: adminKeypair.address, protocol: protocolAddress, market: affectedMarketAddress }, // only admin may clear a bit
  { flags: currentMarketPausedBits & ~PAUSE_BORROW },
);
```

Verify: re-fetch, confirm the bit is cleared, confirm a real `borrow` now succeeds under a normal
price.

## Logging / evidence

Record: the feed(s) affected, the observed staleness/confidence values and when they were first
seen, whether fail-closed behavior alone was sufficient or a pause was required and why, the pause
and unpause transaction signatures, and the sustained-recovery window observed before unpausing.

## Escalation

If the price is suspected **wrong** rather than merely unavailable, or if the feed does not recover
within a time frame that risks accumulating meaningful bad debt, escalate beyond this runbook —
this touches T-20/T-21's accepted residual risks (`threat-model.md` §4), which this runbook cannot
resolve, only contain.

## Do not

- Do not pause `LIQUIDATE` for ordinary staleness (see Step 2).
- Do not assume the guardian can "fix" the feed — its only power here is pausing `BORROW`.
- Do not construct `flags` from scratch; always OR/AND against the current on-chain value.
- Do not skip Step 1; a pause that duplicates behavior the protocol already provides is operational
  noise, not safety.
