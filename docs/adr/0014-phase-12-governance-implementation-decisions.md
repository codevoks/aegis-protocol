# ADR-0014 — Phase 12 governance implementation decisions

**Status:** Accepted · **Date:** 2026-09-13 · **Phase:** 12

## Context

`docs/phases/phase-12-governance.md` and `docs/governance.md` fix the *policy* of Phase 12 — the
guardian asymmetry, the tighten/loosen split, "additive first" migrations — but deliberately leave
several implementation-level questions open, and one place where two frozen documents pull in
different directions. None of these are deviations from a frozen decision; they are gaps the phase
spec left for the implementer to fill, recorded here so a future session does not have to
re-derive them or wonder why a number or a shape was chosen.

## Decisions

### 1. `PendingMarketParams` is a separate PDA account, not embedded in `Market`

`Market._reserved` is 63 bytes; a full risk/IRM/oracle parameter snapshot plus `effective_at` needs
352 bytes (`PendingMarketParams::LEN`). Embedding it would force a `Market` migration in the same
phase that is supposed to demonstrate migrations for an unrelated reason. A standalone
`PDA([b"pending_params", market])`, created lazily by `set_market_params` only on the loosening
path and closed by `commit_pending_params`, needs no `Market` layout change at all and matches the
existing pattern of small, purpose-specific accounts (`Position`, the callback account) elsewhere
in this codebase.

**Alternative rejected:** growing `Market` and migrating it immediately in Phase 12. Rejected
because it conflates "demonstrate the migration primitive" with "grow a load-bearing account,"
and because `governance.md` §6 rule 1 ("additive first... in the common case") already prefers not
reaching for a migration unless one is actually needed.

### 2. The timelock duration is a single constant: 48 hours

Neither `governance.md` §4 nor `economic-model.md` pins an exact duration — only the mechanism
(`PendingMarketParams { params, effective_at }`) is frozen. `constants::PARAM_TIMELOCK_SECS =
172_800` is the one place this number is written; `set_market_params` computes `effective_at` from
it and `commit_pending_params` only ever reads the already-staged value, never re-derives it
(satisfying item 25's "not hardcoded in multiple locations").

**Rationale:** long enough that a depositor watching `ParamsStaged` events has a realistic window
to withdraw before a risk-increasing change takes effect; short enough that legitimate parameter
tuning is not paralyzed. Not derived from any frozen number — an implementation choice, not a
frozen-document fact.

### 3. Tighten/loosen classification for fields `governance.md` §4 does not address

§4's table classifies `max_ltv`, `liq_threshold`, `liq_bonus`, `min_debt`, `max_price_age_secs`,
`max_conf_bps`, `fee`, and (implicitly, by rationale) oracle feed IDs. It says nothing about
`close_factor`, `full_liq_hf`, `liq_protocol_fee`, or the five IRM parameters
(`base_rate_ps`/`slope1_ps`/`slope2_ps`/`u_kink`/`max_rate_ps`).
`docs/phases/phase-12-governance.md` explicitly forbids inventing an "intuition-based rule" for
fields the frozen table does not cover.

**Decision:** any change to one of these unclassified fields is conservatively treated as
loosening (timelocked), regardless of direction. A safe change loses nothing but a delay; a change
that turns out to matter never applies without the observation window the timelock exists to
provide. This is `MutableMarketParams::is_loosening`'s documented default, not a silent guess.

**Alternative rejected:** guessing a direction per field from general lending-protocol intuition
(e.g. "raising IRM rates is always safer"). Rejected because the phase spec names this exact
failure mode and forbids it.

### 4. `fee_recipient` is orthogonal to the timelock, always immediate

`instruction-catalogue.md` §7 lists `fee_recipient` as one of `set_market_params`'s mutable fields,
but it is not a risk parameter and appears in neither row of `governance.md` §4's table. It is kept
out of `PendingMarketParams` entirely and always applied in the same transaction that requests it,
whether or not the same call also stages a risk-parameter change. Changing it requires proving (via
an optional `new_fee_position` account) that the position for the new recipient already exists —
`init_position` is permissionless, so this is never a bottleneck — which prevents the market being
left pointing at a `fee_recipient` whose `Position` PDA has never been created.

### 5. `repay`, `deposit_collateral`, `absorb_bad_debt`, `close_position` never receive a `protocol`
account at all

`instruction-catalogue.md`'s own `repay` row lists `[R][PDA] protocol` even though the same section
calls `repay` "unpausable" — an artifact of that row being drafted alongside `supply`/`withdraw`/
`borrow`'s near-identical shape before Phase 12 existed to need it. `docs/phases/phase-12-governance.md`
is unambiguous that the load-bearing property is structural: "The pause guard must be written so
these instructions do not consult it at all... Unpausable safety exits must simply never call the
guard." Including an unused, always-present `protocol` account in these four instructions would be
an attractive nuisance — the account would already be sitting right there for a future change to
wire a pause check into, exactly the "exception someone could delete" the phase spec warns against.

**Decision:** these four instructions' `Accounts` structs are left exactly as Phase 2-8 wrote them —
no `protocol` field is added. This is a deliberate, security-motivated narrowing relative to the
instruction-catalogue's literal `repay` row, made in the direction the phase's own dominant
directive (INV-ADM-04) requires. `deposit_collateral`/`absorb_bad_debt`/`close_position`'s
documented account lists never included `protocol` in the first place, so only `repay` is actually
affected.

### 6. Pending-proposal overwrite rule: reject, never overwrite or merge

`docs/phases/phase-12-governance.md` item 26 requires reading the current design for what happens
if a second loosening proposal arrives while one is already pending, and using the documented
behavior — or, if genuinely ambiguous (it is; no document addresses this), the safer minimal
behavior. **Decision:** `set_market_params` rejects a second loosening proposal outright
(`PendingParamsAlreadyStaged`) while one exists for that market. There is no `cancel_pending_params`
instruction in this phase's scope; the admin must wait for `commit_pending_params` to clear it (at
or after `effective_at`) before staging a replacement.

**Alternatives rejected:**
- *Silently overwrite.* Lets a later call (or a briefly-compromised admin key) reset the observation
  window a public `ParamsStaged` event already promised, defeating the timelock's actual purpose.
- *Silently merge* the two proposals field-by-field. Undocumented anywhere, and merge semantics
  (which proposal wins per field?) would need their own specification this repository does not have.

### 7. `commit_pending_params` is permissionless

By the time `effective_at` has passed, applying the staged proposal exercises no further admin
discretion — the values were already fixed and publicly observable at staging time. Gating
`commit_pending_params` to the admin would add a liveness dependency (an admin key that goes quiet
after staging a change would leave it stuck forever) for no security benefit, and this repository
already has the precedent of permissionless "apply what is already determined" instructions
(`accrue_interest`, `absorb_bad_debt`). The rent refund on close still targets `protocol.admin`
specifically (never the permissionless caller), so there is no incentive to grief-call it for profit.

### 8. The migration target is `Protocol`, not `Market` or `Position`

`docs/phases/phase-12-governance.md` item 37 suggests "an old versioned protocol/market auxiliary
account" as the example category; no such auxiliary account exists yet, so one of the three real
accounts had to be chosen. `Protocol` carries no economic/custody state (no balances, no shares, no
vault addresses) — migrating it touches nothing accounting-critical, unlike `Market` or `Position`.
The migration adds exactly one field, `schema_version: u8`, carved out of `_reserved` (`_reserved`
shrinks from 64 to 63 bytes; `Protocol::LEN` is unchanged at 202 — no realloc, satisfying
governance.md §6 rule 1). The pre-Phase-12 layout is kept, unchanged, as `ProtocolV1` — used only by
the `From` half of `Migration<'info, ProtocolV1, Protocol>` — so every other instruction's `Account<
'info, Protocol>` usage needed zero changes.

## Consequences

- Positive: no `Market`/`Position` migration was needed to ship Phase 12; the four unpausable
  instructions are structurally, not just behaviorally, incapable of ever consulting pause state;
  the timelock and pending-proposal semantics are each pinned in exactly one place.
- Negative: `instruction-catalogue.md`'s `repay` row (listing a `protocol` account) is now stale
  against the actual implementation and should be corrected in a future documentation pass rather
  than treated as ground truth for that one account.
- Negative: the conservative "any change to an unclassified field is loosening" default means a
  handful of parameter changes (e.g. tuning `close_factor` down, which is arguably risk-reducing)
  are timelocked even though a more precise classification might have allowed them immediately.
  Correcting this requires `governance.md` §4 to be extended with an explicit ruling for those
  fields — a documentation change, not a code change, and deliberately left to a future session
  rather than decided unilaterally here.
