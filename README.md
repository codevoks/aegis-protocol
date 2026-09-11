# Aegis Protocol

**A risk-first, isolated-market, overcollateralized lending protocol on Solana.**

> **STATUS: PHASE 6 — HEALTH, LIQUIDATION AND BAD DEBT.**
> Aegis is under construction. Phase 5 added the real oracle (`programs/aegis/src/oracle`, checks
> O-1..O-11 against the real `pyth-solana-receiver-sdk` 2.0.0 —
> [ADR-0008](docs/adr/0008-oracle-abstraction-no-mock-program.md) — no mock provider, ever) and
> made `borrow`/debt-bearing `withdraw_collateral` real, oracle-validated, LTV-checked
> instructions. Phase 6 adds `liquidate`, `absorb_bad_debt` and `withdraw_collateral_fees`:
> `crates/aegis-math/src/liquidation.rs` implements the close-factor/dust-rule `max_repay`,
> seizure, bonus, protocol cut and the collateral clamp with upward-rounded repay recomputation
> exactly per `economic-model.md` §7, with a **strict** `HF < WAD` liquidatability gate (`HF ==
> WAD` is never liquidatable). Bad debt is recognized once a position's collateral is fully
> exhausted (`collateral_amount == 0` exactly): protocol fee shares are burned first, and only the
> residual is socialized across the market's lenders — permissionlessly, with no oracle dependency
> and no pause path, ever. `withdraw_collateral_fees` lets the admin withdraw only the protocol's
> own accrued collateral fee, structurally bounded so it can never reach user collateral
> (`A-ADM-02`, the concrete proof of INV-ADM-01). Cross-market isolation (`I-ISO-01`) and the
> absence of any shared writable account between markets (`A-PAR-02`) are both directly tested.
> See [`docs/project-status.md`](docs/project-status.md) for the authoritative state of every
> component.

---

## What Aegis is

Each Aegis market is an independent lending venue defined by exactly one collateral asset, one loan
asset, one oracle configuration, and one frozen risk parameter set. Lenders supply the loan asset and
earn utilization-driven interest. Borrowers escrow collateral — which is **never lent out** — and
borrow against it. Positions breaching their liquidation threshold are liquidated permissionlessly for
a bonus. Losses are contained inside the market that produced them, absorbed first by protocol fees
and only then socialized across that market's lenders.

The organizing principle is that **risk must be bounded, named, and localized** — in the account
model, in the economics, and in the failure modes.

## The three decisions that shape it

1. **Isolated two-asset markets, not a cross-collateral money market** ([ADR-0004](docs/adr/0004-isolated-markets.md)).
   Distinct markets share no writable account, so they parallelize by construction, solvency is a
   bounded two-asset computation, and bad debt provably cannot cross markets.
2. **Collateral is escrowed and never lent** ([ADR-0005](docs/adr/0005-collateral-escrow-and-vault-design.md)).
   This makes custody an exact, assertable identity and removes withdrawal-liquidity crunches
   entirely — at a deliberate cost in capital efficiency.
3. **Oracle failure is fail-closed for risk-increasing operations and fail-open for risk-reducing
   ones** ([ADR-0008](docs/adr/0008-oracle-abstraction-no-mock-program.md)).
   Borrowing and liquidation stop when prices are untrustworthy; repaying, topping up collateral, and
   recognizing bad debt never do.

## Planned properties

- Everything runs **offline and free** — no RPC, no API key, no faucet, no paid service.
- Deterministic prices via **byte-exact Pyth account injection**, so tests exercise the real
  deserialization path and the production program contains **no mock oracle code**.
- **87 invariants**, nine of them asserted after every instruction by a stateful fuzzer, with mutation
  testing proving the fuzzer can actually falsify them.
- **32 threats** enumerated, each with a named test that must fail when its mitigation is removed.
- Every performance claim backed by committed before/after compute measurements.

## Documentation

Read in this order:

| Document | Contents |
|---|---|
| [`docs/product.md`](docs/product.md) | Thesis, the product critique that reshaped it, non-goals, requirements |
| [`docs/architecture.md`](docs/architecture.md) | System and module structure |
| [`docs/economic-model.md`](docs/economic-model.md) | **All formulas, units, rounding, worked examples** |
| [`docs/account-model.md`](docs/account-model.md) | Accounts, PDAs, custody, parallelism analysis |
| [`docs/instruction-catalogue.md`](docs/instruction-catalogue.md) | Every instruction, accounts, preconditions, attacks |
| [`docs/oracle-design.md`](docs/oracle-design.md) | Price validation and failure policy |
| [`docs/token-compatibility.md`](docs/token-compatibility.md) | SPL Token / Token-2022 policy |
| [`docs/invariants.md`](docs/invariants.md) | The 87 invariants |
| [`docs/threat-model.md`](docs/threat-model.md) | Trust boundaries, 32 threats, accepted residual risks |
| [`docs/testing-strategy.md`](docs/testing-strategy.md) | The five-tier test pyramid |
| [`docs/performance-strategy.md`](docs/performance-strategy.md) | Compute and contention strategy |
| [`docs/zero-cost-demo.md`](docs/zero-cost-demo.md) | How everything runs free and offline |
| [`docs/governance.md`](docs/governance.md) | Roles, bounded admin power, upgrade progression |
| [`docs/composability.md`](docs/composability.md) | External integration strategy |
| [`docs/coverage-matrix.md`](docs/coverage-matrix.md) | Topic coverage and honest gap analysis |
| [`docs/ecosystem-research.md`](docs/ecosystem-research.md) | Dated toolchain research and open verification gates |
| [`docs/phase-roadmap.md`](docs/phase-roadmap.md) | The 13 implementation phases |
| [`docs/project-status.md`](docs/project-status.md) | **Current state of everything** |
| [`docs/adr/`](docs/adr/) | 12 architecture decision records |

Contributor rules: [`AGENTS.md`](AGENTS.md) (engineering constitution) and [`CLAUDE.md`](CLAUDE.md)
(Claude session workflow).

## Planned stack

Anchor 1.x · Rust · SPL Token & Token-2022 · Pyth pull oracle ·
LiteSVM / Mollusk / Surfpool · `@solana/kit` v8 · Next.js.
Native Solana Rust and Pinocchio appear in scoped, benchmarked labs — not in production
([ADR-0003](docs/adr/0003-native-pinocchio-as-labs.md)).

## Quickstart

**Right now (Phase 6):** on top of everything Phase 2-5 shipped, `programs/aegis` implements
`liquidate` (strict `HF < WAD`; close-factor/dust-rule `max_repay`; seizure with the collateral
clamp; the liquidation bonus; the protocol's cut taken from the bonus only, never from
principal-equivalent collateral), `absorb_bad_debt` (permissionless, no oracle, unpausable,
requires `collateral_amount == 0` exactly; burns the protocol's own fee shares before socializing
any residual across lenders), and `withdraw_collateral_fees` (admin withdrawal bounded by
`market.collateral_fee_accrued`, structurally unable to reach user collateral). Self-liquidation is
permitted and proven economically unprofitable versus a plain `repay` (`U-LIQ-07`). Cross-market
isolation is proven directly: bad debt recognized in one market leaves a second, independent
market byte-identical (`I-ISO-01`), and no writable account is shared between the two for any
Phase 6 instruction (`A-PAR-02`). There is still no SDK/app yet.

```bash
make setup   # verify the pinned toolchain (Solana CLI, Anchor, Surfpool, Node) is installed
make build   # anchor build — compiles `programs/aegis` and generates its IDL
make test    # cargo test --workspace — offline, no network, no secrets (the load-bearing command)
make demo    # SOL crashes to $95.00: a position is liquidated for the exact economic-model.md
             # §7.5 figures (seizure, bonus, protocol cut). A second position is crashed to
             # $40.00, its collateral fully seized by the clamp with debt remaining; the resulting
             # bad debt is absorbed with real protocol fee shares burned FIRST, the residual is
             # socialized, and a lender withdrawal realizes the loss directly — offline against an
             # in-process LiteSVM, byte-exact PriceUpdateV2 fixtures, no Hermes
             # (see docs/phases/phase-06-liquidation.md "Demo")
```

`make fuzz`, `make bench`, and `make app` exist as stubs that name the phase that implements them
(10, 11, and 9 respectively) — they are not yet functional.

The exact install commands, pinned versions, and verification steps are recorded in
[`docs/phases/phase-01-foundation.md`](docs/phases/phase-01-foundation.md) §3 and
[`docs/ecosystem-research.md`](docs/ecosystem-research.md). Everything `.gitignore` excludes
(`target/`, `node_modules/`, `.anchor/`, local validator ledgers, build caches) is mechanically
regenerated by these commands — never hand-crafted, and never required to understand or review the
project.

## Security status

**Aegis is not audited and must not be deployed to mainnet with real user capital.**

The engineering rigor is real; the risk calibration is not. Specifically: risk parameters are
illustrative rather than researched, the oracle is single-source, there are no supply caps, and the
upgrade authority is an unmitigated total risk. See
[`docs/economic-model.md` §11](docs/economic-model.md) for the v1 simplifications and
[`docs/threat-model.md` §4](docs/threat-model.md) for the accepted residual risks — both are stated
plainly rather than buried.

## License

[Apache-2.0](LICENSE).
