# Aegis — Composability Strategy

**Status: FROZEN (Phase 0). Implementation in Phase 8.**

> **Rule: an external integration enters Aegis only when it solves a product problem Aegis actually
> has.** Breadth of integrations is not evidence of skill; a well-motivated integration with a clearly
> stated trust boundary is.

---

## 1. The product problem

To liquidate, a liquidator must already hold the loan asset. That is real capital friction, and it
directly weakens the protocol's most important safety mechanism: if liquidation is capital-intensive,
fewer actors participate, liquidations are slower, and bad debt rises.

The fix is to let a liquidator **seize collateral, swap it, and repay within one transaction**, with no
pre-funding. That requires calling out to external liquidity — genuine composability, with a genuine
security surface.

This is the only external integration Aegis needs, and it is why Phase 8 exists.

---

## 2. Design: an optional liquidation callback

`liquidate` gains an **optional** callback. After collateral is seized and before repayment is
required, Aegis CPIs into a liquidator-specified program, which may swap the collateral and fund the
repayment. Aegis then re-verifies everything.

```mermaid
sequenceDiagram
    participant L as Liquidator
    participant A as Aegis
    participant C as Callback program (UNTRUSTED)
    participant D as DEX / router

    L->>A: liquidate(repay_assets, callback = C)
    A->>A: accrue, validate oracle, check HF < 1, compute seizure
    A->>A: transfer seized collateral -> liquidator
    A->>C: CPI (NO signer forwarded)
    C->>D: swap collateral -> loan asset
    D-->>C: loan asset
    C-->>A: return
    A->>A: RE-READ all state; RE-VERIFY all post-conditions
    A->>A: pull repayment (measured delta); assert exact amount received
    A->>A: assert INV-CUS-01, INV-CUS-02
```

### Security posture: the callback is trusted for nothing

| Defense | Detail |
|---|---|
| **No signer forwarded** | Neither the market PDA's signature nor the liquidator's reaches the callback (INV-AUTH-07). The classic drain vector (T-15) is closed by construction. |
| **All state re-read after the callback** | Any value cached before the CPI is discarded. This holds **regardless** of the runtime's reentrancy semantics, which is why it is mandatory rather than conditional on RV-6. |
| **All post-conditions re-verified** | Repayment measured by vault delta; position state re-checked; INV-CUS-01/02 asserted. |
| **Opt-in per transaction** | Omitting the callback reproduces Phase 6 behavior byte-for-byte (`I-LIQ-CB-02`). |
| **State-machine guard flag** | Blocks a nested `liquidate` on the same market during the callback, independent of runtime behavior. |

"Read state before the CPI, act on it after" is exactly the bug pattern that makes callbacks
dangerous, which is why the re-read rule is stated as a rule rather than an optimization.

**RV-6** — whether the Solana runtime permits `A → B → A` CPI reentrancy — must be closed with a
primary source before implementation. Aegis must not *depend* on the answer, but the documentation
must state it correctly.

---

## 3. Integration inventory and trust boundaries

| Integration | Phase | Trusted for | NOT trusted for | Required offline? |
|---|---|---|---|---|
| **SPL Token / Token-2022** | 2+ | Executing transfers correctly; enforcing mint and decimals in `transfer_checked` | Reporting balances matching our expectations — we always measure deltas | **Yes** |
| **Pyth pull oracle** | 5 | Publishing a price with an honest confidence interval, verified by Wormhole | Availability, freshness, correctness, or being the feed we asked for — all four are checked (O-1..O-11) | **Yes** (account injection) |
| **Liquidation callback target** | 8 | **Nothing** | Everything | **Yes** (local example) |
| **Jupiter** | 8 | Nothing on-chain — it is reached only *through* an untrusted callback | Everything | **No — optional tier** |
| System program | 2+ | Account creation and rent | — | **Yes** |

Aegis CPIs into exactly three program classes: the two token programs, the system program, and the
Phase 8 callback (INV-RES-07). Anything else is a bug.

**Aegis never CPIs into Pyth.** Reading a pull price is an account read, which is what makes offline
determinism possible at all (ADR-0008).

---

## 4. Jupiter: integrated, never depended upon

Jupiter is the natural liquidity router for liquidation, but it needs real mainnet liquidity — which
would break NFR-4 if it were required. The resolution:

- **Required path:** the local example callback swaps at a deterministic local price. Fully offline,
  fully deterministic, and it exercises the entire callback code path and every adversarial case.
- **Optional path:** a Surfpool **mainnet-fork** test performs a real Jupiter route against real
  mainnet state, fetched just-in-time. Tagged `#[ignore]`/`--network` and excluded from `make test`.

Surfpool's JIT mainnet fetching is what makes even the optional tier free — it needs an RPC endpoint
but no paid service and no deployed capital.

**The rule:** if a README claim depends on the optional tier, either the claim is wrong or the tier is
misclassified.

---

## 5. Rejected integrations

| Rejected | Why |
|---|---|
| Building an AMM inside Aegis | Non-goal. Claiming "AMM knowledge" by building one is the padding the brief forbids. Aegis *uses* liquidity; it does not manufacture it. |
| Generic flash loans | The scoped liquidation callback covers the interesting composability and reentrancy surface **with a product justification**. A general flash-loan facility has none. |
| Transfer-hook execution | Unbounded CU and arbitrary failure would make liquidation DoS-able (T-27) and guarantee bad debt. Rejected in `token-compatibility.md`; a hook **allowlist** is the honest v2. |
| Cross-chain messaging | Enormous trust surface, no product reason. |
| Lending-aggregator integrations | Aegis should be the integrated protocol, not the integrator, at this stage. |
| Yield-strategy routing for idle collateral | Would reintroduce re-hypothecation and destroy INV-CUS-02 (ADR-0005). |
| An oracle aggregator across several providers | Genuinely valuable and a named v2 — but v1 should demonstrate one oracle integration done rigorously rather than three done shallowly. |

---

## 6. What Aegis exposes to integrators

Aegis is designed to be composed *with*, not only to compose:

- **Deterministic addresses.** Markets and positions are content-addressed PDAs, derivable offline
  with no registry lookup.
- **Complete event coverage.** One typed event per state transition, sufficient to reconstruct
  protocol state off-chain (FR-19).
- **Permissionless liquidation and bad-debt absorption.** No allowlist, no privileged keeper.
- **A typed SDK** with read models and transaction builders (Phase 9).
- **`accrue_interest` as a public instruction**, so integrators can refresh state before reading it.

Tokenized supply shares — which would make Aegis positions composable *as assets* elsewhere — are the
most valuable next step and are deliberately deferred (ADR-0006).
