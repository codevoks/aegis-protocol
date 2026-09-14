# Aegis — Operational Runbooks (Phase 13)

Five runbooks, one per scenario named in [`docs/governance.md` §7](../governance.md). Each expands
that section's trigger/immediate/decide/recovery sketch into an executable procedure: exact
authority, exact commands, validation before and after, rollback where one exists, and explicit
"do not" items. No runbook ID or content here was invented beyond what `governance.md` §7 already
names — this document only adds the executable detail that section deliberately left out.

| ID | Scenario | Authority |
|---|---|---|
| [R-1](R-1-oracle-degradation.md) | Oracle degradation | Guardian (pause), admin (unpause) |
| [R-2](R-2-accounting-bug.md) | Suspected accounting bug | Guardian (pause), engineering (fix) |
| [R-3](R-3-bad-debt-event.md) | Bad debt event | Anyone (permissionless), admin (review) |
| [R-4](R-4-admin-key-compromise.md) | Suspected admin key compromise | Guardian (pause), admin/upgrade authority (recovery) |
| [R-5](R-5-hostile-market-params.md) | Hostile market parameters detected | Guardian (pause), admin (review) |

## How these commands are issued

Aegis has no dedicated admin CLI as of v0.1.0 — a real, stated limitation, not an oversight. Every
runbook below issues its instruction through `@aegis/sdk`'s typed instruction builders
(`sdk/ts/src/ix.ts`), the same functions the web app and the test suite already use, run from a
short Node/ts-node script or a REPL. This is deliberate: the runbook commands below are not
aspirational pseudocode, they are the exact function names that exist and are tested today
(`sdk/ts/src/ix.ts`'s exports, exercised by `I-SDK-02`).

```ts
// Common preamble every runbook below assumes (values are illustrative; substitute the real
// cluster, protocol/market addresses, and the operator's own keypair):
import { createSolanaRpc } from "@solana/kit";
import {
  AEGIS_PROGRAM_ADDRESS,
  buildSetProtocolPauseInstruction,
  buildSetMarketPauseInstruction,
  buildSetGuardianInstruction,
  buildSetPendingAdminInstruction,
  buildAcceptAdminInstruction,
} from "@aegis/sdk";

const rpc = createSolanaRpc(process.env.AEGIS_RPC_URL!); // devnet or mainnet endpoint, operator-supplied
// `operatorKeypair` is loaded from the operator's own secure key storage -- never from a file
// committed to this repository or typed into a chat session (AGENTS.md §19).

// The pause bitflags (`account-model.md` §3, `programs/aegis/src/constants.rs`) are not currently
// exported by `@aegis/sdk` (they exist only as private constants in `sdk/ts/src/read.ts`) --
// defined here literally rather than imported, to stay honest about what the package exports
// today. Flagged as a real SDK-completeness gap in this phase's report, not silently worked around.
const PAUSE_SUPPLY = 0b0001;
const PAUSE_BORROW = 0b0010;
const PAUSE_WITHDRAW = 0b0100;
const PAUSE_LIQUIDATE = 0b1000;
const PAUSE_ALL_BITS = PAUSE_SUPPLY | PAUSE_BORROW | PAUSE_WITHDRAW | PAUSE_LIQUIDATE;
```

**Signature note:** every generated builder below takes **`(programId, accounts, args?)`** as
separate positional parameters — `accounts` is the accounts object (signer/PDA addresses), `args`
(omitted for instructions with no arguments, e.g. `absorb_bad_debt`) is the instruction's own data.
This is the real, generated signature (`sdk/ts/src/generated/instructions.ts`), re-exported
unchanged from `ix.ts` and the package root — not a simplification.

Every command below is a **transaction the operator constructs, signs, and submits themselves**,
using a keypair they alone control. Nothing in this repository, this document, or any tool acting
on it ever holds, requests, or transmits that keypair.

## What every runbook shares

- **Validate before acting.** Read current on-chain state (`fetch_protocol`/`fetch_market`
  equivalents in the SDK's `accounts.ts`) before submitting any transaction — never act from
  memory of what the state "should" be.
- **Verify after acting.** Re-fetch the same account and confirm the field actually changed to the
  expected value. A submitted transaction that lands is not the same as a confirmed state change.
- **Log the evidence.** Record the transaction signature, the before/after account state, and the
  operator's identity (which keypair signed) in the incident record. On-chain events
  (`ProtocolPauseSet`, `MarketParamsUpdated`, etc.) are the permanent, public audit trail — the
  incident record should cite the exact event and slot, not paraphrase it.
- **Never do the things §"Explicit forbidden actions" below lists**, in any runbook, under any
  pressure.

## Explicit forbidden actions (all runbooks)

These are restated once here rather than in each runbook, because they are absolute, not
scenario-specific:

- **Never** move, redirect, or "rescue" user funds administratively. No instruction in this
  program can do this (INV-ADM-01) — if an operator ever believes one is needed, that belief is
  itself the incident, not a solution to it (see R-4).
- **Never** bypass the parameter timelock (`governance.md` §4). If a change is urgent enough to
  feel like it needs bypassing, it is risk-reducing (tightening) and already applies immediately,
  or it is risk-increasing and the timelock is exactly the protection working as designed.
- **Never** share, copy, or transmit a private key — admin, guardian, or otherwise — through chat,
  email, a ticket, or any channel other than the operator's own signing device.
- **Never** disable, comment out, or weaken an on-chain check to "unblock" an incident. Every check
  in this program exists because removing it was analyzed and rejected (`AGENTS.md` §7).
- **Never** modify account state manually (e.g., via a privileged RPC method or a patched
  validator) to work around a stuck transaction. If a legitimate instruction cannot reach the
  desired state, that is a bug to fix and test, not a state to hand-edit.
- **Never** skip the post-action verification step "to restore service faster." An unverified
  pause/unpause/parameter change is not a completed action.
