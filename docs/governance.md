# Aegis — Governance, Upgrades and Operational Model

**Status: FROZEN (Phase 0). Implementation in Phase 12.**

> A security-conscious protocol does not rely on "the admin can fix anything." Aegis's governance
> design is mostly about **what the admin structurally cannot do**.

---

## 1. Roles

| Role | Powers | Cannot |
|---|---|---|
| **Admin** | Create markets; set risk/IRM/oracle params within bounds; set guardian; transfer admin (two-step); withdraw accrued collateral fees; clear pause bits | Move user funds; change a market's mints, token programs, vaults, decimals or `config_id`; set parameters outside on-chain bounds; block repayment or collateral deposits |
| **Guardian** | **Set** pause bits only (protocol-wide or per-market) | **Clear** pause bits; change any parameter; move any funds; pause `repay`, `deposit_collateral`, `absorb_bad_debt`, or `close_position` |
| **Fee recipient** | Withdraw its accrued supply shares via the ordinary `withdraw` path | Anything else — it is a plain position with no privileges |
| **Upgrade authority** | Replace the program entirely | Nothing constrains it. This is the largest residual risk in the system (T-30) |
| **Users / liquidators** | Everything permissionless | — |

The **guardian's asymmetry is the point**: an emergency key that can stop the protocol but cannot
restart it is safe to hold in a hot wallet, because compromising it causes an outage rather than a
loss. Recovery requires the colder admin key. This is standard practice and is cheap to implement.

---

## 2. What the admin structurally cannot do (INV-ADM-01)

There is exactly **one** admin-initiated token movement in the entire protocol:
`withdraw_collateral_fees`, bounded by `market.collateral_fee_accrued`, which increases only inside
`liquidate` by exactly `protocol_cut`.

There is **no** instruction that lets any authority:
- transfer from a vault to an arbitrary destination;
- alter a position's balances;
- change a market's vault addresses or mints;
- mint, burn, or reassign shares.

This is enforced by the account model, not by policy, and `A-ADM-02` attempts the attack and must
fail. **Loan-side protocol fees have no privileged withdrawal path at all** — the fee recipient calls
`withdraw` like any other lender, which removes an entire privileged code path.

---

## 3. Pause philosophy

Pausing is a blunt instrument that itself creates risk, so it is bounded in three ways:

1. **Only four bits exist:** `SUPPLY`, `BORROW`, `WITHDRAW`, `LIQUIDATE`. There is no "pause
   everything" and no arbitrary flag space.
2. **Four operations are structurally unpausable (INV-ADM-04):** `repay`, `deposit_collateral`,
   `absorb_bad_debt`, `close_position`.
   *Rationale:* a pause must never trap a user's funds or prevent a user from reducing their own risk.
   A borrower must always be able to repay and top up collateral; the protocol must always be able to
   recognize a loss. Any design where an operator can prevent debt repayment is custodial in effect,
   whatever it claims.
3. **Pausing `LIQUIDATE` is itself dangerous** and is treated as such. It stops liquidations exactly
   when they matter, converting market risk into guaranteed bad debt. It exists only for a specific
   scenario — a suspected oracle compromise where liquidations would be *wrong* — and is governed by
   the runbook in §7, not by operator instinct.

`WITHDRAW` covers both lender withdrawals and collateral withdrawals. Pausing it prevents users from
retrieving their own funds and is therefore the second most serious pause; it exists for suspected
accounting-bug scenarios.

---

## 4. Parameter-change policy

**Every** parameter write re-validates the full bounds from `economic-model.md` §5, including the
derived `liq_threshold · (WAD + liq_bonus) / WAD < WAD` constraint. An out-of-bounds parameter set is
unrepresentable, not merely discouraged.

**Phase 12 adds an asymmetry** — the key governance idea in Aegis:

| Direction | Examples | Timing |
|---|---|---|
| **Risk-reducing (tightening)** | Lower `max_ltv`; lower `liq_threshold`; raise `min_debt`; lower `max_price_age_secs`; lower `max_conf_bps`; lower `fee` | **Immediate** |
| **Risk-increasing (loosening)** | Raise `max_ltv`/`liq_threshold`; lower `min_debt`; raise `max_price_age_secs`/`max_conf_bps`; raise `fee`/`liq_bonus`; change oracle feed IDs | **Timelocked** via `PendingMarketParams { params, effective_at }` |

Rationale: an operator responding to deteriorating market conditions must be able to act *now*;
an operator (or an attacker holding the admin key) increasing risk must be observable in advance.
A uniform timelock would be actively harmful — it would prevent emergency de-risking.

`set_market_params` calls `accrue_mut` **before** applying changes (INV-ADM-07), so accrued interest
settles under the old parameters; otherwise a fee increase would retroactively tax interest already
earned.

Feed IDs are classified as risk-increasing because swapping a feed is equivalent to swapping the
asset's price source — the highest-leverage change an admin can make.

---

## 5. Upgrade-authority progression

| Stage | Authority | When | Residual risk |
|---|---|---|---|
| **0 — Local** | Local dev keypair | Phases 1–11 | None (no real value) |
| **1 — Single hardware key** | Hardware wallet, offline backup | First devnet deploy | Total compromise if the key is lost or stolen |
| **2 — Multisig** | Squads-style m-of-n | Before any deployment holding real value | Compromise requires m signers; still total if reached |
| **3 — Multisig + timelock** | m-of-n with a delay on upgrades | Meaningful TVL | Users get advance notice and an exit window |
| **4 — Revoked (immutable)** | None | Only after audits and a long stable period | **Bugs become unfixable.** Immutability is a trade, not a virtue |

Aegis v1 reaches **Stage 1** and documents the rest. Claiming stages 2–4 without implementing them
would be exactly the kind of unsupported assertion this repository forbids.

**Verifiable builds:** builds are reproducible and verified against the deployed bytecode via the
OtterSec registry (`verify.osec.io`) — `apr.dev` is defunct, and Anchor 1.1.1 reimplemented
`verifiedBuild` against OtterSec. Without a verifiable build, "the source is public" says nothing
about what is actually deployed.

### The honest statement about T-30

> Whoever holds the upgrade authority can replace the program and take every asset in every vault,
> immediately, regardless of every check, invariant and test in this repository. No in-program
> mitigation exists. Anyone evaluating a deployed Solana protocol should check the upgrade authority
> before reading the code.

---

## 6. Migration strategy

Account layout changes use **Anchor 1.0's `Migration<'info, From, To>`** rather than a hand-rolled
scheme — a real, framework-supported primitive now exists and hand-rolling one would be strictly worse
(INV-UPG-01).

Design rules:
1. **Additive first.** Every account carries `_reserved` bytes, so new fields are added without
   realloc or migration in the common case.
2. **Explicit and idempotent.** A migration is an instruction, is permissionless or admin-gated per
   case, rejects an already-migrated account, and is tested with `I-UPG-01/02`.
3. **No in-place reinterpretation.** Changing the meaning of existing bytes is forbidden; add a field
   and migrate.
4. **Never during an emergency.** Migration under time pressure is how funds are lost.

---

## 7. Operational runbooks (Phase 13)

Short, specific, decision-oriented — written before they are needed.

### R-1 — Oracle degradation
*Trigger:* feeds stale or confidence persistently above threshold.
*Immediate:* the protocol already fails closed for `borrow`, `withdraw_collateral` (with debt), and
`liquidate`; risk-reducing operations continue. **Verify this rather than assuming it.**
*Decide:* pause `BORROW` to stop the hole deepening. Do **not** pause `LIQUIDATE` unless the price is
suspected *wrong* rather than merely *stale* — a stale price already blocks liquidation on its own.
*Recovery:* unpause only after feeds are fresh and confidence is normal for a sustained window.

### R-2 — Suspected accounting bug
*Trigger:* INV-CUS-01/02 violated in monitoring.
*Immediate:* guardian pauses `SUPPLY`, `BORROW`, `WITHDRAW`. Leave `LIQUIDATE` active unless the bug
is in liquidation itself.
*Note:* `repay` and `deposit_collateral` remain open by design — users must always be able to reduce
their own risk.
*Then:* reproduce locally against the exact state; write a failing test before writing a fix.

### R-3 — Bad debt event
*Trigger:* positions with collateral exhausted and debt outstanding.
*Immediate:* nothing — `absorb_bad_debt` is permissionless and needs no oracle. Anyone can call it.
*Then:* publish the size and cause; verify protocol fee shares absorbed first; review whether the
market's parameters (LTV, bonus, `min_debt`) contributed.

### R-4 — Suspected admin key compromise
*Immediate:* guardian pauses everything pausable. Note the guardian **cannot** unpause, which is
exactly the property wanted here.
*Then:* rotate via `set_pending_admin`/`accept_admin` **if** the legitimate holder still controls the
key; otherwise the upgrade authority is the only recourse — which is why stages 2–3 exist.

### R-5 — Hostile market parameters detected
*Trigger:* a market created or updated with dangerous-but-legal parameters.
*Immediate:* pause that market only. Isolation means no other market is affected — this is ADR-0004
paying off operationally.

---

## 8. What Aegis deliberately does not build

| Not built | Reason |
|---|---|
| On-chain token-voting governance | Governance theatre without a real stakeholder set. A multisig is the honest answer at this stage. |
| A DAO treasury | No token, no treasury. |
| Permissionless market creation (v1) | Risk-first: allowlisted parameter sets are the honest path to permissionless creation, and that is a v2 with its own ADR. |
| An emergency fund migration ("rescue") instruction | Any instruction able to move user funds under an emergency condition is exactly the backdoor INV-ADM-01 exists to prevent. If the protocol needs rescuing, that is the upgrade authority's job, visibly. |
| Admin-forced liquidation | Liquidation is permissionless; a privileged path would add risk and no capability. |

The "rescue instruction" row is the one most often gotten wrong in practice: it feels prudent, and it
converts a non-custodial protocol into a custodial one with extra steps.
