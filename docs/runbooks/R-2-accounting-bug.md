# R-2 — Suspected accounting bug

**Trigger:** `governance.md` §7 — INV-CUS-01 or INV-CUS-02 (the exact vault-reconciliation
identities, `invariants.md` §B) observed violated by monitoring, or any other reproducible
mismatch between on-chain accounting totals and actual vault balances.

## Prerequisites

- Read access to the affected market (`Market`, both vaults, and every `Position` the monitoring
  system flagged).
- The **guardian** keypair, to pause.
- A local clone of this repository at the exact deployed commit, to reproduce.

## Step 1 — Confirm the violation against real on-chain state, not a monitoring artifact

```ts
// Fetch the market and both vaults directly; compute the identities yourself rather than
// trusting a dashboard's derived number:
//   loan_vault.amount            == market.total_supply_assets - market.total_borrow_assets   (INV-CUS-01)
//   collateral_vault.amount      == Σ(position.collateral_amount) + market.collateral_fee_accrued (INV-CUS-02)
```

A monitoring system can itself have a bug (stale cache, wrong market address, a race between two
reads). Confirm the mismatch with two independent, freshly-fetched reads before treating it as real.

## Step 2 — Immediate: guardian pauses `SUPPLY`, `BORROW`, `WITHDRAW`

```ts
const ix = buildSetProtocolPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: guardianKeypair.address, protocol: protocolAddress },
  { flags: currentProtocolPausedBits | PAUSE_SUPPLY | PAUSE_BORROW | PAUSE_WITHDRAW }, // NOT PAUSE_LIQUIDATE
);
```

**Leave `LIQUIDATE` active** unless the bug is specifically in `liquidate`'s own accounting path —
liquidations reduce risk exposure and should continue unless there is a specific reason to believe
they would worsen the exact inconsistency being investigated.

**`repay` and `deposit_collateral` remain open regardless** — they are structurally unpausable
(INV-ADM-04) and no pause bit combination changes that; do not treat their continued availability
as a residual gap to close.

Prefer scoping to `set_protocol_pause` (protocol-wide) only if the bug's cause is not yet isolated
to one market; if it clearly is, use `set_market_pause` on that market alone to preserve other
markets' availability (isolation, ADR-0004).

## Step 3 — Post-action verification

- Re-fetch `protocol.paused` (or the specific market's `paused`); confirm the three bits are set
  and `LIQUIDATE` is not.
- Confirm a real (or observed) `supply`/`borrow`/`withdraw` now fails with `OperationPaused`.
- Confirm `repay`/`deposit_collateral`/`absorb_bad_debt`/`close_position` still succeed.

## Step 4 — Reproduce locally before writing any fix

1. Clone this repository at the exact commit the deployed program was built from
   (`solana-verify get-program-hash`, `docs/project-status.md` Phase 12 §6, gives the verified
   commit/hash pairing).
2. Reproduce the exact on-chain state that triggered the violation — via a real instruction
   sequence if possible, or via the established `seed_borrow_state`-style fixture injection
   technique if the state is otherwise unreachable in a test environment
   (`crates/aegis-test-kit/src/state_injection.rs`).
3. **Write a failing test first** (`AGENTS.md` §7.2: "never weaken or delete a test to make it
   pass" — the corollary here is "never write the fix before the test that proves the bug exists").
4. Fix the root cause. Confirm the new test passes and the full offline suite (`make test`) is
   otherwise unaffected.
5. Record the finding in `docs/security/findings.md`, following the existing F-10-02/F-13-01
   format (severity, root cause, minimized reproduction, fix, scope check for the same pattern
   elsewhere).

## Recovery

Unpause (admin only) once the fix is deployed (a program upgrade — see the upgrade-authority
process, `governance.md` §5) and re-verified on-chain against the specific scenario that triggered
the incident.

```ts
const ix = buildSetProtocolPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: adminKeypair.address, protocol: protocolAddress },
  { flags: currentProtocolPausedBits & ~(PAUSE_SUPPLY | PAUSE_BORROW | PAUSE_WITHDRAW) },
);
```

## Logging / evidence

Record: the exact accounts and values that violated the identity, the two independent
confirmation reads, the pause transaction signature, the minimized reproduction test, the fix
commit, the regression test, and the unpause transaction signature.

## Escalation

If the fix requires a program upgrade, this becomes an upgrade-authority action
(`governance.md` §5, T-30) — the single largest residual risk in the system. Treat the upgrade
itself with the same rigor as any other release: full regression, no unrelated changes bundled in.

## Do not

- Do not hand-edit any account's on-chain data to "correct" the accounting. There is no
  instruction for this and there should never be one (INV-ADM-01's spirit extends to operational
  practice, not just on-chain code).
- Do not pause `LIQUIDATE` by default — only if the bug is specifically implicated there.
- Do not skip writing the failing test before the fix.
- Do not deploy the fix as part of a larger, unrelated upgrade.
