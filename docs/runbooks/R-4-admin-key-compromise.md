# R-4 — Suspected admin key compromise

**Trigger:** `governance.md` §7 — any evidence the admin keypair may be under an attacker's
control: an unexpected `set_market_params`/`set_guardian`/`set_pending_admin` transaction the
legitimate operator did not send, a phishing report against whoever holds the key, or a compromised
signing device.

## Prerequisites

- The **guardian** keypair (this runbook's entire immediate action runs through it).
- Knowledge of whether the legitimate admin still controls the key (this determines the entire
  recovery path — determine it as early as possible, even approximately).

## Step 1 — Immediate: guardian pauses everything pausable

```ts
const ix = buildSetProtocolPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: guardianKeypair.address, protocol: protocolAddress },
  { flags: currentProtocolPausedBits | PAUSE_ALL_BITS }, // SUPPLY | BORROW | WITHDRAW | LIQUIDATE
);
```

This is the one runbook where pausing `LIQUIDATE` too is correct: a compromised admin cannot move
funds directly (INV-ADM-01), but the point of this step is to freeze *all* discretionary activity
immediately while the situation is assessed, not to optimize for which specific bit matters most.

**The guardian cannot unpause — this is exactly the property wanted here.** A compromised guardian
key could only ever make things *more* paused, never less; recovery requires the (uncompromised, by
assumption at this point) admin key, or ultimately the upgrade authority.

## Step 2 — Assess: does the legitimate admin still control the key?

| Answer | Path |
|---|---|
| **Yes** (e.g., a phishing attempt was reported but not completed, or the concern was a false alarm) | Proceed to "Recovery — key retained," below. |
| **No** (the key is confirmed or strongly suspected to be under attacker control) | Proceed to "Recovery — key lost," below. This is the harder case. |
| **Unknown** | Treat as "No" until proven otherwise. Staying paused costs availability; assuming safety costs funds. |

## Recovery — key retained (rotate as a precaution)

Two-step transfer, exactly as designed for this (`instruction-catalogue.md` §2, INV-ADM-02):

```ts
// From the (retained) current admin key, to a NEW admin key generated fresh, ideally on new
// hardware if the old device is the one suspected:
const setPending = buildSetPendingAdminInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { admin: currentAdminKeypair.address, protocol: protocolAddress },
  { newAdmin: newAdminPublicKey },
);
// ... submit, confirm ...

// From the NEW admin key:
const accept = buildAcceptAdminInstruction(AEGIS_PROGRAM_ADDRESS, {
  pendingAdmin: newAdminKeypair.address,
  protocol: protocolAddress,
}); // no instruction args -- accept_admin takes none
// ... submit, confirm ...
```

Verify: re-fetch `protocol.admin`, confirm it equals the new key, and confirm
`protocol.pending_admin` is cleared (a replayed `accept_admin` must now fail, since the old pending
admin no longer matches).

Then unpause, from the new admin key:

```ts
const unpause = buildSetProtocolPauseInstruction(
  AEGIS_PROGRAM_ADDRESS,
  { authority: newAdminKeypair.address, protocol: protocolAddress },
  { flags: 0 },
);
```

## Recovery — key lost

**The admin key alone cannot recover from this.** If the legitimate operator no longer controls the
admin key, `set_pending_admin` cannot be issued from it. The only remaining recourse is the
**upgrade authority** (`governance.md` §5) — which is exactly why Stages 2-3 (multisig, then
multisig + timelock) exist as the documented progression beyond a single hardware key. If Aegis is
still at Stage 0/1 (a single key, as of this v0.1.0 release — see `governance.md` §5.1) and that key
is genuinely lost, there is no in-protocol recovery path, and this is the concrete, stated cost of
not yet having reached Stage 2. State this plainly to stakeholders rather than implying a recovery
exists that does not.

If the upgrade authority is separate from the admin key and still controlled, it can deploy a
program upgrade that changes `protocol.admin` via a migration instruction analogous to
`migrate_protocol_v2` — this is a real program upgrade, not a runbook-level action, and must go
through the same rigor as any other release (full regression, verifiable build, no unrelated
changes).

## Post-action verification

- `protocol.admin` (and, if rotated, `protocol.pending_admin`) match the intended final state.
- `protocol.paused == 0` only after the admin transfer (if any) is confirmed complete — never
  unpause before the key situation is resolved.
- The old, potentially-compromised key's authority is confirmed gone: attempt (in a test
  transaction, not against production) a `set_market_params` from the old key against a throwaway
  market and confirm it now fails `NotProtocolAdmin`, if practical to verify this way.

## Logging / evidence

Record: what triggered the suspicion, the pause transaction signature, the assessment conclusion
(retained vs. lost) and its basis, the rotation transactions (if any) and their signatures, and the
final unpause transaction signature.

## Escalation

A confirmed key loss at Stage 0/1 with no separate upgrade authority is a protocol-ending event for
that deployment — escalate to stakeholders immediately and transparently rather than attempting an
undocumented recovery.

## Do not

- Do not unpause before the admin situation is fully resolved.
- Do not attempt to move or "protect" user funds by any administrative means — no such instruction
  exists (INV-ADM-01), and none should be improvised under pressure.
- Do not share the suspected-compromised key with anyone "to investigate" — treat it as burned the
  moment compromise is suspected.
- Do not skip the two-step transfer in favor of a faster single-step change — there is no
  single-step `set_admin` instruction, by design (`no_single_step_set_admin_instruction_exists`).
