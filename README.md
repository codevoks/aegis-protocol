# Aegis Protocol

**A risk-first, isolated-market, overcollateralized lending protocol on Solana.**

> **STATUS: PHASE 9 — SDK, CLIENT & UI.**
> Aegis is under construction. Phase 9 adds a typed TypeScript SDK (`sdk/ts/`, `@aegis/sdk`) built
> on `@solana/kit` v8.x and `@anchor-lang/core` — never `@coral-xyz/anchor` or `@solana/web3.js` —
> with IDL codegen deriving every discriminator, layout, and account order directly from
> `target/idl/aegis.json`; PDA/account/read helpers; a `bigint` client-side math port validated
> bit-for-bit against vectors the real Rust `aegis-math` emits (`I-SDK-01`); a transaction builder
> for every user-facing instruction, each proven to fit the classic 1232-byte legacy limit with no
> address lookup table, `liquidate`'s callback variant included (`I-TX-01`); and a Next.js app
> (`app/`) covering the full local user journey — market view, position health, deposit/borrow/
> repay/withdraw, and a scripted demo that warps time to show interest accruing and scripts a price
> drop into a liquidation — against a real local Surfpool validator, entirely offline. No on-chain
> code changed this phase. See [`docs/project-status.md`](docs/project-status.md) for the
> authoritative state of every component.

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
| [`docs/adr/`](docs/adr/) | 13 architecture decision records |

Contributor rules: [`AGENTS.md`](AGENTS.md) (engineering constitution) and [`CLAUDE.md`](CLAUDE.md)
(Claude session workflow).

## Planned stack

Anchor 1.x · Rust · SPL Token & Token-2022 · Pyth pull oracle ·
LiteSVM / Mollusk / Surfpool · `@solana/kit` v8 · Next.js.
Native Solana Rust and Pinocchio appear in scoped, benchmarked labs — not in production
([ADR-0003](docs/adr/0003-native-pinocchio-as-labs.md)).

## Quickstart

**Right now (Phase 9):** on top of everything Phase 2-8 shipped (no on-chain code changed this
phase), `sdk/ts/` (`@aegis/sdk`) and `app/` (a Next.js application) exist. The SDK's typed
instruction builders, PDA derivation, and account decoders are generated from the real
Anchor-build IDL; its client-side math is validated bit-for-bit against vectors the real Rust
`aegis-math` emits; every instruction's realistic transaction — including `liquidate` with a
callback — fits the classic 1232-byte limit with no address lookup table. The app runs the full
local user journey (market view, deposit, borrow, health-factor tracking, repay, withdraw) plus a
scripted demo (warp time to show interest accruing, crash a price, liquidate) against a local
Surfpool validator, entirely offline.

```bash
make setup        # verify the pinned toolchain (Solana CLI, Anchor, Surfpool, Node) is installed
make build        # anchor build (aegis) + cargo build-sbf (the two Phase 8 labs/ programs)
make test         # cargo test --workspace — offline, no network, no secrets (the load-bearing command)
make demo         # Phase 8's Rust liquidation-callback demo (docs/phases/phase-08-composability.md)
make sdk-install  # npm install in sdk/ts/
make app-install  # npm install in app/ (resolves @aegis/sdk via a local file: path)
make sdk-test     # SDK unit tests: cross-language math (I-SDK-01), PDA parity (I-SDK-03),
                  # transaction size (I-TX-01) -- no running validator required
make sdk-test-e2e # I-SDK-02: build/sign/send/confirm/decode for every instruction, against a
                  # real local Surfpool validator this test starts and tears down itself
make app          # starts a local Surfpool, deploys aegis.so, runs the Next.js dev server at
                  # http://localhost:3000 -- Ctrl+C stops both. Visit /demo to seed a market and
                  # run the scripted interest-accrual + liquidation demo.
```

`make fuzz` and `make bench` exist as stubs that name the phase that implements them (10 and 11
respectively) — they are not yet functional.

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
