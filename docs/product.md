# Aegis — Product Thesis, Critique, and Scope

**Status: FROZEN (Phase 0). Changing anything here requires an ADR.**

---

## 1. Final product thesis

> **Aegis is a risk-first, isolated-market, overcollateralized lending protocol on Solana.**
>
> Each Aegis market is an independent, permissionlessly-usable lending venue defined by exactly one
> collateral asset, one loan asset, one oracle configuration, and one frozen risk parameter set.
> Lenders supply the loan asset and earn utilization-driven interest. Borrowers escrow collateral —
> which is **never lent out** — and borrow against it. Positions that breach their liquidation
> threshold are liquidated permissionlessly for a bonus. Losses are contained inside the market that
> produced them, absorbed first by protocol fees and only then socialized across that market's lenders.
>
> The protocol's organizing principle is that **risk must be bounded, named, and localized** — in the
> account model, in the economics, and in the failure modes.

One sentence: *Aegis is Morpho-Blue-shaped isolated lending, designed natively for Sealevel, with
oracle safety and solvency accounting treated as the primary engineering problem.*

---

## 2. Product critique — challenging the brief

The brief proposed "a risk-first overcollateralized lending protocol." That direction survives, but
**three material changes** were made after critique. Each is recorded as an ADR.

### 2.1 Is overcollateralized lending the right core product?

**Assessed honestly: yes, but only in a sharpened form.**

Arguments *for* lending as the core:

- It is the one DeFi primitive that *forces* the engineer through every hard subject simultaneously:
  fixed-point arithmetic, interest accrual, share accounting, oracle safety, custody and vault
  authority, liquidation game theory, solvency invariants, and adversarial testing. No other single
  product produces that density honestly.
- The domain is well understood, so a reviewer can distinguish a competent implementation from a
  shallow one. Novelty would hide weakness; familiarity exposes it. For evidence-of-skill purposes
  that is a feature.
- Failure modes are economically meaningful and testable, which makes an invariant catalogue and a
  property-testing campaign *real work* rather than ceremony.

Arguments *against*, taken seriously:

- **"Another lending protocol" is the single most-cloned portfolio project on Solana.** A generic
  Solend/Aave clone is a liability, not an asset. The differentiation must come from architecture and
  rigor, not from the category.
- Classic **cross-collateral pooled money markets** (one global user account referencing N reserves)
  have an enormous design surface (e-mode, isolation mode, siloed borrowing, per-reserve caps) that is
  easy to enumerate and very hard to do well. Attempting it produces breadth-shaped shallowness.
- Cross-collateral designs are also **structurally hostile to Sealevel**: a user account touching many
  reserves serializes execution across otherwise-unrelated assets.

**Resolution:** keep lending, reject the cross-collateral money-market shape, adopt isolated markets.

### 2.2 CHANGE 1 — Isolated two-asset markets, not a cross-collateral money market

*(ADR-0004)*

Every market is `(collateral_mint, loan_mint, oracle_config, risk_params)`. A position exists inside
exactly one market and has exactly one collateral asset and one debt asset.

Why this is a genuine improvement, not a simplification:

1. **It makes the Sealevel claim demonstrable rather than rhetorical.** Distinct markets share no
   writable state, so they execute in parallel by construction. A cross-collateral design cannot make
   that claim. We can *measure* this in Phase 11.
2. **Solvency becomes tractable and bounded.** Health is a two-asset function with at most two oracle
   reads. Compute is bounded, the account list is short and fixed, and the oracle failure surface per
   transaction is minimal.
3. **Risk isolation becomes real.** Bad debt in one market provably cannot touch another. That is the
   literal meaning of "risk-first," expressed in the type system and the account graph rather than in
   a README adjective.
4. **It reflects where the market actually went.** Morpho Blue and Euler v2 moved to isolated markets;
   monolithic pools are the older design. Choosing isolation shows the engineer is current.

Cost accepted: worse capital efficiency and worse UX for multi-collateral borrowers. Documented, not
hidden. A "cross-market aggregation / vault curation layer" is the honest v2 answer and is named as
out-of-scope, not pretended away.

### 2.3 CHANGE 2 — Collateral is escrowed and never lent

*(ADR-0005)*

In Aave-style designs, deposited collateral is simultaneously supply liquidity and can be borrowed by
others. Aegis rejects this. Collateral sits idle in a vault; only the loan asset supplied by lenders
is lent.

Why:

- **Custody accounting becomes exactly reconcilable.** `collateral_vault.amount` equals the sum of
  position collateral plus accrued collateral fees — an invariant a test can assert byte-for-byte
  after every operation. Under re-hypothecation that identity does not exist.
- It **removes an entire failure class**: withdrawal liquidity crunches, where a solvent lender cannot
  exit because their assets were lent out.
- It makes liquidation deterministic: seized collateral is always physically present.

Cost accepted: collateral earns no yield, so Aegis is less capital-efficient than Aave. This is the
correct trade for a protocol whose thesis is bounded risk, and it is stated plainly rather than framed
as an oversight.

### 2.4 CHANGE 3 — Peer-to-pool lending with share accounting, not a fictional liquidity source

*(ADR-0006)*

An early temptation is to model only the borrow side and let loan liquidity appear from a treasury.
That is economically incoherent and would make the interest model decorative.

Aegis implements a real supply side: lenders deposit the loan asset, receive **internal scaled
shares** (not a token), and earn interest driven by utilization. Shares — rather than a separate
`supply_index`/`borrow_index` pair — because share math has cleaner, directly assertable conservation
invariants and makes every rounding direction explicit at the call site.

Deliberately **not** tokenizing supply shares in v1: an SPL share mint would add mint authority,
custody, and Token-2022 surface for composability Aegis does not yet need. Recorded as a plausible v2.

### 2.5 Is the product economically coherent?

Checked explicitly:

| Question | Answer |
|---|---|
| Where does loan liquidity come from? | Lenders, who receive shares and interest. Real. |
| Why would a lender supply? | Utilization-driven yield, with the protocol taking a bounded fee. |
| Why would a borrower borrow? | Leverage / liquidity without selling collateral. Standard and real. |
| Why would a liquidator act? | A bonus paid in seized collateral, permissionless and immediate. |
| Who eats losses? | Protocol fee shares first, then that market's lenders. Named, bounded, implemented. |
| Can the protocol be solvent-by-construction? | No — and Aegis says so. Overcollateralized lending is solvent *under assumptions* about oracle quality and liquidation liquidity. Those assumptions are enumerated in `economic-model.md` §10. |

### 2.6 Can this evolve beyond a portfolio project?

Yes, and the seams are deliberate:

- Isolated markets + permissioned creation → permissionless creation with allowlisted parameter sets.
- Internal shares → tokenized shares → curated vaults aggregating markets (the Morpho/MetaMorpho path).
- Stateless IRM → adaptive IRM behind the same interface.
- Single upgrade authority → multisig → timelocked params → immutable core.

Each is an additive step behind an existing interface, not a rewrite. That is the test of whether an
architecture can evolve, and Aegis passes it.

---

## 3. Non-goals (explicit, permanent for v1)

Aegis will **not** contain these, regardless of topic-coverage temptation:

| Not building | Why not |
|---|---|
| An AMM / DEX | Unrelated to lending. Building one to claim "AMM knowledge" is exactly the padding the brief forbids. Swap needs are met by *integrating* with existing liquidity in Phase 8. |
| Perpetuals / derivatives | A different protocol with a different risk engine. |
| A stablecoin / CDP mint | Would replace the peer-to-pool thesis with a seigniorage thesis. Coherent, but a different product. |
| NFT anything | No product reason. |
| Staking / liquid staking | No product reason. Yield-bearing LSTs may later be *collateral*; issuing them is out of scope. |
| Governance token, emissions, incentives | Tokenomics theatre; adds no engineering evidence Aegis lacks. |
| Generic flash loans | The scoped liquidation callback (Phase 8) covers the interesting composability and reentrancy surface with a real product justification. A general flash-loan facility does not. |
| Cross-collateral positions / e-mode | Contradicts ADR-0004. Named as the v2 aggregation layer. |
| Confidential-transfer support | Opaque balances are irreconcilable with our custody invariants. |
| Adaptive/PID interest curves | v2 behind the same IRM interface; a stateless curve is fully sufficient and deterministically testable in v1. |
| Cross-chain / bridging | Enormous trust surface, no product reason. |
| On-chain order-book liquidation auctions | Fixed-bonus liquidation is the correct v1; Dutch auctions are a documented v2. |

**Rule:** anything on this list may only enter Aegis through an ADR that states the *product* reason.
"Demonstrates X" is not a product reason.

---

## 4. Personas and use cases

### P1 — Lender ("Sofia", yield-seeking holder of the loan asset)
Wants predictable yield on an asset she already holds, with legible and bounded risk.
- U1: Supply loan asset to a chosen market; see the current supply APY and utilization.
- U2: Observe her accrued interest.
- U3: Withdraw principal + interest, subject to available (unborrowed) liquidity.
- **What she must be able to see:** exactly which collateral backs her loans, that market's LLTV, its
  oracle, and its bad-debt history. Isolation is the product feature she is buying.

### P2 — Borrower ("Ravi", holder of a volatile collateral asset)
Wants liquidity without selling.
- U4: Initialize a position in a market.
- U5: Deposit collateral.
- U6: Borrow the loan asset up to `max_ltv`.
- U7: Monitor health factor and liquidation price.
- U8: Repay partially or fully.
- U9: Withdraw collateral, subject to remaining solvency.
- **Critical guarantee:** risk-*reducing* actions (deposit collateral, repay) must never be blocked by
  oracle unavailability. This is a first-class requirement, not a nicety (see FR-15).

### P3 — Liquidator ("Keeper", automated, adversarial, self-interested)
Wants profit; the protocol depends on that self-interest.
- U10: Scan positions and compute health off-chain.
- U11: Liquidate an unhealthy position, repaying debt and seizing collateral at a bonus.
- U12: (Phase 8) Do so without pre-funding, via a liquidation callback that swaps seized collateral to
  repay in one transaction.
- **Design consequence:** if liquidation is unprofitable or unavailable, Aegis accrues bad debt. The
  liquidator's incentive is a *protocol safety mechanism*, and is analyzed as such.

### P4 — Risk operator / admin ("Protocol steward")
- U13: Create a market with a frozen, bounds-checked parameter set.
- U14: Tighten risk parameters (immediately) or loosen them (delayed, Phase 12).
- U15: Pause a market or the protocol via a guardian key.
- U16: Withdraw accrued protocol fees.
- **Constraint:** the admin must be structurally incapable of seizing user funds. This is enforced by
  the account model, not by policy (see INV-ADM-*).

### P5 — Integrating engineer
- U17: Use a typed TypeScript SDK to read markets/positions and build transactions.
- U18: Run the entire protocol locally, free, deterministically, in minutes.

---

## 5. Functional requirements

| ID | Requirement |
|---|---|
| FR-1 | Admin can initialize a singleton protocol config defining admin, guardian, and fee recipient. |
| FR-2 | Admin can create an isolated market from `(collateral_mint, loan_mint, oracle config, risk params, config_id)`; parameters are bounds-checked at creation and the market's identity is a deterministic PDA over those inputs. |
| FR-3 | Market creation rejects any mint whose token-extension set is outside the supported policy (`token-compatibility.md`). |
| FR-4 | A user can initialize exactly one position per (market, owner) via an explicit instruction. |
| FR-5 | A lender can supply loan assets and receive supply shares. |
| FR-6 | A lender can withdraw loan assets by burning supply shares, limited by unborrowed liquidity. |
| FR-7 | A borrower can deposit collateral. **This must require no oracle.** |
| FR-8 | A borrower can borrow loan assets if the resulting LTV ≤ `max_ltv` and available liquidity suffices. |
| FR-9 | A borrower can repay debt in whole or in part. **This must require no oracle.** |
| FR-10 | A borrower can withdraw collateral if the resulting LTV ≤ `max_ltv`. |
| FR-11 | Anyone can liquidate a position whose health factor < 1, repaying up to the close factor of its debt and seizing collateral at a bonus. |
| FR-12 | Below a configured health floor, or when a partial liquidation would leave dust debt, liquidation of 100% of debt is permitted. |
| FR-13 | Anyone can trigger `absorb_bad_debt` on a position with zero collateral and non-zero debt; loss is charged to protocol fee shares first, then socialized across that market's lenders. |
| FR-14 | Interest accrues continuously in time (per-second), driven by a stateless utilization curve, and is applied lazily on any state-changing interaction plus a standalone permissionless `accrue_interest`. |
| FR-15 | Oracle failure (stale, wide-confidence, wrong feed, missing) must **fail closed** for borrow, withdraw collateral, and liquidate — and must **not block** deposit collateral, repay, supply, or withdraw of loan assets. |
| FR-16 | Guardian can pause a market or the protocol; pausing must never block repay or deposit-collateral. |
| FR-17 | Admin can withdraw accrued collateral-side liquidation fees; loan-side fees are withdrawn as ordinary supply shares. |
| FR-18 | A fully-emptied position can be closed and its rent reclaimed by its owner. |
| FR-19 | Every state transition emits a typed event sufficient to reconstruct protocol state off-chain. |
| FR-20 | A typed TypeScript SDK exposes read models (market, position, health, liquidation price, APYs) and transaction builders for every user-facing instruction. |

## 6. Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-1 | **No floating-point arithmetic anywhere on-chain.** All economics are integer/fixed-point. |
| NFR-2 | Every arithmetic operation is checked or provably bounded; overflow must abort, never wrap. Release builds must enable `overflow-checks`. |
| NFR-3 | Rounding always resolves in favor of the protocol; every rounding direction is explicit at its call site and individually tested. |
| NFR-4 | The full core test suite runs offline, with no RPC, no faucet, no API key, and no paid service. |
| NFR-5 | The core suite completes fast enough for tight iteration (target: LiteSVM suite < 60s on a laptop). |
| NFR-6 | No user-facing instruction may exceed the default 200k CU budget; each has a documented, benchmarked CU figure. |
| NFR-7 | Distinct markets must share no writable account, so they parallelize. Collateral deposit/withdraw must not write the `Market` account. |
| NFR-8 | Every account passed to every instruction is validated for owner, discriminator, PDA derivation with canonical bump, and relational consistency (`has_one`). |
| NFR-9 | The admin must be structurally unable to move user funds; parameter changes are bounds-checked on-chain. |
| NFR-10 | Every invariant in `invariants.md` maps to at least one falsifying test. |
| NFR-11 | No secret, keypair, or `.env` value is ever committed. |
| NFR-12 | Every performance claim is backed by committed before/after measurements. |
| NFR-13 | All time-based logic uses unix seconds from `Clock`, never slot counts (SIMD-0525 makes slots non-uniform). |
| NFR-14 | Token amounts are `u64` base units; internal economic quantities are `u128` WAD; all multiply-divide uses 256-bit intermediates. |

---

## 7. What "done" means for v1

Aegis v1 is complete when a reader can, on a laptop with no accounts and no money:

1. Clone the repository and run one command that builds and runs the full test suite offline.
2. Read `docs/` and understand the economics, the account graph, the threat model, and the invariants
   before reading any code.
3. Run a local demo that creates a market, supplies, borrows, moves the price deterministically,
   liquidates, and produces bad debt — and see each invariant checked at every step.
4. Point at any protocol claim and find the test, benchmark, or ADR that substantiates it.
