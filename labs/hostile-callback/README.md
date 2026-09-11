# `hostile-callback`

A **test-only, adversarial** liquidation-callback program used exclusively to prove Aegis's Phase 8
defenses (`A-CPI-01..04`, `docs/phases/phase-08-composability.md`, ADR-0013,
`tests/phase8_hostile_callback.rs`).

**This is never a real integration.** Every attack it implements is expected to fail. A test
asserting anything else about this program has misunderstood its purpose.

## Attack modes (`AttackMode`, selected via instruction data)

| Mode | Proves | Expected result |
|---|---|---|
| `DrainVault { amount }` | `A-CPI-01`: a hostile callback cannot move Aegis vault funds | Fails — this program has no authority (owner/delegate) over `loan_vault`; the real SPL Token/Token-2022 program rejects the transfer. |
| `Reenter` | `A-CPI-02`: a hostile callback cannot CPI back into `liquidate` | Fails — this program was never given `market`'s or `liquidator`'s signature (ADR-0013), and the current Solana runtime additionally rejects indirect CPI reentrancy on its own (RV-6, `docs/ecosystem-research.md` §16.1). The exact error surfaced is not asserted (see the source doc comment on `reenter()`) to avoid crediting the wrong mechanism; the direct, protocol-level proof of the Aegis-level guard is a separate, non-CPI unit test in `tests/phase8_hostile_callback.rs`. |
| `BurnCompute { iterations }` | `A-CPI-03`: compute exhaustion fails cleanly, atomically | Fails — the transaction runs out of compute; Solana's own atomicity leaves zero partial state. |
| `NoRepayment` | `A-CPI-04`: a callback returning `Ok(())` is not proof of repayment | The callback instruction itself succeeds, but the *outer* `liquidate` transaction fails on the measured `loan_vault` delta. |

## Account contract

Identical position-for-position to any real Aegis liquidation callback (see
`labs/example-liquidator/README.md`) — this program deliberately receives nothing more than a
legitimate callback would, precisely so its failures demonstrate what an attacker *without* extra
privilege can and cannot do.

## Running the attacks

`tests/phase8_hostile_callback.rs` drives all four modes against a real, deployed `.so` (via
LiteSVM), with a before/after state snapshot proving atomic rollback for each — not merely that the
transaction returned an error.
