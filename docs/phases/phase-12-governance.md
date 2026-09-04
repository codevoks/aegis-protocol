# Phase 12 — Governance, Upgrades and Migrations

**Status: NOT STARTED.** **Prerequisite: Phase 11 complete and tagged.**

## Scope
1. `set_pending_admin` / `accept_admin` (two-step transfer).
2. `set_guardian`.
3. `set_protocol_pause` / `set_market_pause` with the **guardian asymmetry**: the guardian may only
   *set* pause bits; only the admin may clear them.
4. `set_market_params` with full bounds re-validation and `accrue_mut` **before** applying changes.
5. **The tighten/loosen asymmetry**: risk-reducing changes apply immediately; risk-increasing changes
   are staged in `PendingMarketParams { params, effective_at }` behind a timelock
   (`governance.md` §4).
6. An account migration using Anchor 1.0's **`Migration<'info, From, To>`** — a real framework
   primitive now exists, so do not hand-roll one (INV-UPG-01).
7. Verifiable builds via the OtterSec registry (`verify.osec.io`; `apr.dev` is defunct).
8. Deployment and upgrade-authority documentation for stages 0–1.

## Explicit NON-scope
No on-chain token voting. No DAO treasury. No emergency fund-migration ("rescue") instruction —
**ever**; any instruction able to move user funds under an emergency condition is precisely the
backdoor INV-ADM-01 exists to prevent. No multisig integration code (a documented operational
procedure, not a program feature).

## Files
`instructions/admin/{set_pending_admin.rs, accept_admin.rs, set_guardian.rs, set_protocol_pause.rs, set_market_pause.rs, set_market_params.rs, commit_pending_params.rs, migrate_*.rs}` ·
`guards.rs` (pause guard) · `state/market.rs` (`PendingMarketParams`) ·
`docs/governance.md` (deployment procedure)

## Concepts demonstrated
Two-step authority transfer · asymmetric emergency roles · bounded administrative power ·
timelocked risk-increasing changes · account schema migration · verifiable builds · upgrade-authority
risk management.

## Security work
- **INV-ADM-04 is the load-bearing property of this phase**: `repay`, `deposit_collateral`,
  `absorb_bad_debt`, and `close_position` must remain callable with **every** pause bit set. The pause
  guard must be written so these instructions do not consult it at all, rather than consulting it and
  being excepted — an exception is a line someone can delete.
- The guardian must be unable to clear a pause bit (`A-AUTH-04`).
- `set_market_params` must be unable to change mints, token programs, vaults, decimals, or
  `config_id` (`A-ADM-06`).
- All bounds re-validated on every write, including the derived liquidation bound.
- Migration must be idempotent and reject an already-migrated account.

## Tests
`A-AUTH-04` (guardian cannot unpause), `A-AUTH-05` (only the pending admin can accept),
`A-ADM-01` (**repay/deposit/absorb succeed with every pause bit set** — the single most important test
in this phase), `A-ADM-03` (paused borrow fails), `A-ADM-04` (bounds re-validated on update),
`A-ADM-05` (undefined pause bits rejected), `A-ADM-06` (immutable identity fields),
`U-ADM-01` (accrual precedes a parameter change),
`I-ADM-01` (tighten immediate, loosen timelocked),
`I-UPG-01` (migration correctness), `I-UPG-02` (migration idempotence),
`I-UPG-03` (verifiable build matches the deployed program).

## Demo
Transfer admin in two steps; guardian pauses everything and demonstrably **cannot** unpause; a
borrower still repays and deposits collateral while fully paused; admin unpauses; tighten a parameter
immediately; attempt to loosen and observe the timelock; commit after the delay; run a migration on a
live account.

## Acceptance criteria
- [ ] Two-step admin transfer works; a single-step transfer is impossible.
- [ ] Guardian can set but not clear pause bits.
- [ ] `repay`, `deposit_collateral`, `absorb_bad_debt`, `close_position` succeed with all pause bits set.
- [ ] Tighten/loosen asymmetry implemented and tested.
- [ ] Bounds re-validated on every parameter write.
- [ ] Identity fields immutable.
- [ ] A migration executes, is idempotent, and is tested.
- [ ] A verifiable build is produced and verified against the deployed program.
- [ ] `governance.md` updated with the real deployment procedure and the current upgrade authority.
- [ ] INV-ADM-01..09, INV-AUTH-04/05, INV-UPG-01..05 tested.
- [ ] Universal checklist satisfied. Tag `phase-12-governance`.

**STOP after this phase.**
