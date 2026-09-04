# ADR-0012 — Progressive upgrade-authority hardening and bounded admin power

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

## Context

The upgrade authority of a Solana program can replace the entire program and take every asset it
custodies. This is the largest risk in any upgradeable protocol (T-30), and no in-program mitigation
exists for it. Separately, the *admin* role — parameter management — is a risk that **can** be bounded
structurally, and should be.

## Decision

Two distinct commitments.

### 1. Admin power is structurally bounded (INV-ADM-01, INV-ADM-04)

- **There is exactly one admin-initiated token movement in the protocol**:
  `withdraw_collateral_fees`, bounded by `market.collateral_fee_accrued`, which increases only inside
  `liquidate` by exactly `protocol_cut`.
- No instruction lets any authority transfer from a vault to an arbitrary destination, alter a
  position's balances, change a market's mints or vaults, or mint/burn/reassign shares.
- Loan-side protocol fees have **no** privileged withdrawal path at all — the fee recipient calls
  `withdraw` like any lender. One fewer privileged code path.
- **Four operations can never be paused by anyone**: `repay`, `deposit_collateral`,
  `absorb_bad_debt`, `close_position`. A pause must never trap a user's funds or prevent them from
  reducing their own risk.
- **Guardian asymmetry**: the guardian may only *set* pause bits; only the admin may clear them. An
  emergency key that can stop but not restart is safe to hold hot — compromising it causes an outage,
  not a loss.
- **Tighten/loosen asymmetry** (Phase 12): risk-*reducing* parameter changes apply immediately;
  risk-*increasing* changes are timelocked. A uniform timelock would be actively harmful, because it
  would prevent emergency de-risking.

### 2. Upgrade authority hardens progressively

| Stage | Authority | When |
|---|---|---|
| 0 | Local dev keypair | Phases 1–11 |
| 1 | Hardware-backed single key | First devnet deploy |
| 2 | Multisig (m-of-n) | Before holding real value |
| 3 | Multisig + upgrade timelock | Meaningful TVL |
| 4 | Revoked (immutable) | Only after audits and a long stable period |

**Aegis v1 reaches Stage 1 and documents the rest.** Claiming stages 2–4 without implementing them
would be exactly the unsupported assertion this repository forbids.

Builds are verifiable via the OtterSec registry (`verify.osec.io`) — `apr.dev` is defunct, and Anchor
1.1.1 reimplemented `verifiedBuild` against OtterSec. Without a verifiable build, "the source is
public" says nothing about what is deployed.

## Alternatives considered

**Immutable from day one.** Rejected: bugs would be unfixable, and immutability before an audit is
recklessness dressed as rigor. Immutability is a trade, not a virtue.

**On-chain token governance.** Rejected: governance theatre without a real stakeholder set. A multisig
is the honest answer at this stage.

**An emergency fund-migration ("rescue") instruction.** Rejected outright, and this is the decision
most often gotten wrong in practice. It feels prudent, and it converts a non-custodial protocol into a
custodial one with extra steps. Any instruction able to move user funds under an emergency condition
is precisely the backdoor INV-ADM-01 exists to prevent.

**Single-step admin transfer.** Rejected: a transfer to a typo'd or non-existent key permanently
bricks governance. Two-step is standard and cheap.

## Consequences

**Positive**
- "Non-custodial" becomes a structural property with a test (`A-ADM-02` attempts to withdraw user
  collateral via the fee path and must fail), not a claim.
- Emergency response is possible without granting fund access.
- Users can always exit or de-risk, whatever the operator does.

**Negative**
- The upgrade authority remains an unmitigated total risk at v1. **Stated plainly** in
  `governance.md` §5 and `threat-model.md` T-30 rather than minimized: anyone evaluating a deployed
  Solana protocol should check the upgrade authority before reading the code.
- Bounded admin power means some legitimate interventions are impossible. That is the intended
  trade.
