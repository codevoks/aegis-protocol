# Aegis Protocol

**A risk-first, isolated-market, overcollateralized lending protocol on Solana.**

> **STATUS: PHASE 5 — ORACLE. NO LIQUIDATION OR BAD-DEBT LOGIC EXISTS.**
> Aegis is under construction. Phase 4 added the economic core on top of Phase 3's custody flows:
> `supply`/`withdraw` (share-based accounting with virtual-offset inflation defense), `repay`
> (no oracle, no owner signature, unpausable, clamped to actual debt), and permissionless
> `accrue_interest` (utilization-driven piecewise-linear rates, Taylor-series compounding,
> protocol fees minted as supply shares). Phase 5 adds the real oracle: `programs/aegis/src/
> oracle` implements checks O-1..O-11 against the real `pyth-solana-receiver-sdk` 2.0.0
> ([ADR-0008](docs/adr/0008-oracle-abstraction-no-mock-program.md) — no mock provider, no mock
> program, ever), and the Phase 3/4 hard gates are removed: `borrow` and the debt path of
> `withdraw_collateral` are now real, oracle-validated, LTV-checked instructions that fail closed
> on any oracle failure. Risk-reducing operations (`deposit_collateral`, `repay`, debt-free
> `withdraw_collateral`) still require no oracle at all. There is still no health-factor-driven
> liquidation or bad-debt socialization — those begin at Phase 6. See
> [`docs/project-status.md`](docs/project-status.md) for the authoritative state of every
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

**Right now (Phase 5):** on top of everything Phase 2/3/4 shipped, `programs/aegis` implements the
real oracle: `oracle::require_valid_price` enforces checks O-1..O-11 (owner, discriminator, feed
identity, full verification, staleness in unix seconds, future-skew, price positivity, confidence
bound, sanity bounds, exponent-safe scaling, distinct feed accounts) against real
`pyth-solana-receiver-sdk` 2.0.0 `PriceUpdateV2` accounts — an account read, never a CPI, so the
Pyth program is never deployed. `borrow` is now real: oracle-validated, accrual-aware, and
LTV-checked (`debt_value <= collateral_value * max_ltv / WAD`, collateral priced at the confidence
lower bound floored, debt at the upper bound ceiled). `withdraw_collateral`'s debt path now
performs the same post-withdrawal health check; a debt-free withdrawal still reads no oracle at
all. `repay`/`deposit_collateral`/`supply`/`withdraw` remain fully oracle-independent — proven by
`A-ORACLE-01`/`A-ORACLE-02` against a maximally broken oracle. `A-SHARE-01` demonstrates the
first-depositor share-inflation attack succeeding without the virtual offsets and becoming a net
*loss* for the attacker with them. There is still no health-factor-driven liquidation or bad debt,
and no SDK/app yet.

```bash
make setup   # verify the pinned toolchain (Solana CLI, Anchor, Surfpool, Node) is installed
make build   # anchor build — compiles `programs/aegis` and generates its IDL
make test    # cargo test --workspace — offline, no network, no secrets (the load-bearing command)
make demo    # borrow succeeds against a real, valid oracle price; the oracle goes stale so borrow
             # and debt-bearing withdraw_collateral fail closed while repay and deposit_collateral
             # keep working; the oracle recovers at a new price and the recomputed health factor
             # is printed — offline against an in-process LiteSVM, byte-exact PriceUpdateV2
             # fixtures via the real pyth-solana-receiver-sdk, no Hermes
             # (see docs/phases/phase-05-oracle.md "Demo")
```

`make fuzz`, `make bench`, and `make app` exist as stubs that name the phase that implements them
(10, 11, and 9 respectively) — they are not yet functional. The full lending/liquidation/bad-debt
demo scenario in [`docs/zero-cost-demo.md`](docs/zero-cost-demo.md) §5 ships in Phase 13, once those
instructions exist.

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
