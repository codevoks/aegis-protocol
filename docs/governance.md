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

### 3.1 Full pause matrix (Phase 12 implementation record)

| Instruction | Protocol pause bit | Market pause bit | Callable while fully paused? |
|---|---|---|---|
| `supply` | `SUPPLY` | `SUPPLY` | No |
| `withdraw` | `WITHDRAW` | `WITHDRAW` | No |
| `borrow` | `BORROW` | `BORROW` | No |
| `withdraw_collateral` | `WITHDRAW` | `WITHDRAW` | No |
| `liquidate` | `LIQUIDATE` | `LIQUIDATE` | No |
| `repay` | — (never consulted) | — (never consulted) | **Yes** |
| `deposit_collateral` | — (never consulted) | — (never consulted) | **Yes** |
| `absorb_bad_debt` | — (never consulted) | — (never consulted) | **Yes** |
| `close_position` | — (never consulted) | — (never consulted) | **Yes** |
| `init_position`, `accrue_interest` | — (never consulted) | — (never consulted) | **Yes** (never pausable; not risk-taking) |
| `create_market`, `set_market_params`, `set_*_pause`, `set_pending_admin`, `accept_admin`, `set_guardian`, `commit_pending_params`, `withdraw_collateral_fees`, `migrate_protocol_v2` | — (admin/guardian-gated, not pause-gated) | — | **Yes** (governance/admin instructions are authorization-gated, not pause-gated) |

Either `protocol.paused` or `market.paused` having the relevant bit set is sufficient to block the
five pausable instructions — a single check against the bitwise OR of both fields
(`guards::require_pause_bit_clear`). The four rows marked "never consulted" are not merely
unaffected by every pause bit being set; their instruction handlers contain no reference to
`guards::require_pause_bit_clear`, `protocol.paused`, or `market.paused` at all, and their
`Accounts` structs do not even include a `protocol` account (ADR-0014 §5) — there is no code path
through which a future change could accidentally start gating them without that change being a
visible, reviewable addition of a whole new account and a whole new guard call, not the deletion of
one `if`. `A-ADM-01` exercises this with real, on-chain state transitions: every protocol AND every
market pause bit set, then `repay`/`deposit_collateral`/`absorb_bad_debt`/`close_position` all
still succeed.

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
asset's price source — the highest-leverage change an admin can make. `oracle_kind` is classified
the same way, for the same reason.

`close_factor`, `full_liq_hf`, `liq_protocol_fee`, and the five IRM parameters (`base_rate_ps`,
`slope1_ps`, `slope2_ps`, `u_kink`, `max_rate_ps`) have no directional classification in this table
— inventing one from general lending-protocol intuition is exactly what Phase 12 forbids
(`docs/phases/phase-12-governance.md`). **Any** change to one of these fields is conservatively
treated as risk-increasing (timelocked), regardless of direction (ADR-0014 §3). `fee_recipient` is
not a risk parameter at all and is not part of this table — it always applies immediately,
independent of whatever else the same call tightens or stages (ADR-0014 §4).

**Timelock duration:** `constants::PARAM_TIMELOCK_SECS` — **48 hours** — is the single canonical
value; `set_market_params` computes `effective_at` from it once, and `commit_pending_params` only
ever reads the already-staged value back, never re-derives it (ADR-0014 §2). No frozen document
pins an exact duration; this is Phase 12's own implementation decision.

**Storage:** a loosening proposal is staged in `PendingMarketParams`, a standalone account at
`PDA([b"pending_params", market])` — not embedded in `Market` (ADR-0014 §1). It carries every
risk/IRM/oracle field plus `effective_at`; `fee_recipient` is never staged (see above).

**Pending-proposal overwrite rule:** while a proposal is already staged for a market, a second
loosening call is rejected outright (`PendingParamsAlreadyStaged`) — never silently overwritten or
merged (ADR-0014 §6). The admin must wait for `commit_pending_params` to clear the existing
proposal (at or after its `effective_at`) before staging a replacement. `commit_pending_params`
itself is permissionless (ADR-0014 §7): by the time the timelock has elapsed, applying the
already-public, already-fixed proposal exercises no further admin discretion. It re-validates the
full canonical bounds against the staged values before applying them — a proposal valid when staged
is never assumed valid forever — and closes `PendingMarketParams`, refunding its rent to
`protocol.admin` (never to whichever address happened to submit the commit transaction).

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

### 5.1 Current state (Phase 12 implementation record)

A real devnet deployment exists — program ID `DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9`
(matches `declare_id!` exactly), upgrade authority `ALjq2DN6nipDE31uadKrSHnvpYwm54sH7LyM2VMp2vBE`.
**Stated plainly and precisely, not rounded up:** that authority is a plain software keypair
generated for this deployment, not a hardware wallet — this is the "first devnet deploy" milestone
the table above uses to date Stage 1, but it is not yet true Stage-1 *key-custody* hardening. A
real Stage 1 still requires moving that authority to an actual hardware wallet with an offline
backup before any deployment holds real value; Stages 2–4 remain entirely future work. Full
deployment record (transaction, program-data address, build architecture, verified hash):
`docs/project-status.md`, Phase 12 §7. Verifiable-build evidence (local build hash == on-chain
program hash, reproducible via `scripts/verify-build.sh`): `docs/project-status.md`, Phase 12 §6.

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

### 6.1 Phase 12 implementation record: `migrate_protocol_v2`

The first real migration is `Protocol` → itself, adding one field: `schema_version: u8`, carved out
of `_reserved` (`_reserved` shrinks `64 → 63` bytes; `Protocol::LEN` is unchanged at 202 — no
realloc). The pre-Phase-12 layout is kept as `state::protocol::ProtocolV1`, used only as the `From`
half of `Migration<'info, ProtocolV1, Protocol>` — no other instruction ever constructs or accepts
a `ProtocolV1`. `Protocol` was chosen over `Market`/`Position` because it carries no economic or
custody state (ADR-0014 §8): the migration touches admin/guardian/pause/fee-recipient bookkeeping
only, never a balance, a share count, or a vault address.

`migrate_protocol_v2` is admin-gated (the pre-migration account's own `admin` field, checked via
`try_as_from()` before calling `.migrate()`), takes no token accounts and no mint at all, and moves
no funds (`A-ADM-01`'s "no rescue path" mandate extends here too — this instruction transforms
schema, nothing else). Idempotence (`I-UPG-02`) comes from the `Migration` primitive itself: an
already-migrated account no longer deserializes as `ProtocolV1` (its bytes now carry `Protocol`'s
own Anchor discriminator), so a second attempt fails at account validation, before the handler body
ever runs — not a hand-rolled "already migrated" check that a future edit could accidentally skip.

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
