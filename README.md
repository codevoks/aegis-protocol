# Aegis Protocol

**A risk-first, isolated-market, overcollateralized lending protocol on Solana.**

> **STATUS: PHASE 7 — TOKEN-2022 COMPLETION.**
> Aegis is under construction. Phase 6 added `liquidate`, `absorb_bad_debt` and
> `withdraw_collateral_fees` — close-factor/dust-rule liquidation, the collateral clamp, and
> bad-debt socialization with protocol first-loss, exactly per `economic-model.md` §7-8. Phase 7
> closes **RV-5**, the research gate asking for the complete current Token-2022 extension list:
> the resolved `spl-token-2022-interface` version (2.1.0) is enumerated and classified extension
> by extension in [`docs/token-compatibility.md`](docs/token-compatibility.md) §0, including
> `Pausable` and `ScaledUiAmount` — extensions added after older, commonly-remembered lists. The
> positive-allowlist policy engine, `ImmutableOwner` vault sizing, and measured-delta accounting
> were already complete from Phases 2/3; Phase 7's new work is the full protocol lifecycle
> (supply → deposit → borrow → accrue → liquidate → bad debt → fee withdrawal) proven correct on a
> real transfer-fee Token-2022 collateral market (`A-TOK-10`), a fee rate raised mid-lifecycle via
> the real `SetTransferFee` instruction and Token-2022's genuine 2-epoch activation delay,
> without breaking accounting because Aegis never caches a fee rate anywhere (`A-TOK-11`), and a
> concrete proof that `ImmutableOwner` blocks vault-authority reassignment even by the account's
> genuine owner. Aegis's supported Token-2022 surface did not broaden: zero lines changed in
> `programs/aegis/src` this phase. See [`docs/project-status.md`](docs/project-status.md) for the
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
| [`docs/adr/`](docs/adr/) | 12 architecture decision records |

Contributor rules: [`AGENTS.md`](AGENTS.md) (engineering constitution) and [`CLAUDE.md`](CLAUDE.md)
(Claude session workflow).

## Planned stack

Anchor 1.x · Rust · SPL Token & Token-2022 · Pyth pull oracle ·
LiteSVM / Mollusk / Surfpool · `@solana/kit` v8 · Next.js.
Native Solana Rust and Pinocchio appear in scoped, benchmarked labs — not in production
([ADR-0003](docs/adr/0003-native-pinocchio-as-labs.md)).

## Quickstart

**Right now (Phase 7):** on top of everything Phase 2-6 shipped, RV-5 is closed — the complete
current Token-2022 extension list (27 real `ExtensionType` variants in the resolved
`spl-token-2022-interface` 2.1.0) is enumerated and classified in
[`docs/token-compatibility.md`](docs/token-compatibility.md) §0, with `Pausable` and
`ScaledUiAmount` (extensions shipped after older, commonly-remembered lists) both verified rather
than assumed. The full protocol lifecycle — supply, deposit, borrow, accrual, liquidation, bad
debt, protocol first-loss, fee withdrawal — is proven correct on a real transfer-fee Token-2022
collateral market with `INV-CUS-01`/`INV-CUS-02` asserted after every instruction (`A-TOK-10`), and
a fee rate raised mid-lifecycle via the real `SetTransferFee` instruction (respecting Token-2022's
genuine 2-epoch activation delay) does not break accounting, because Aegis never caches a fee rate
anywhere (`A-TOK-11`). `ImmutableOwner` is proven to actually block vault-authority reassignment,
not merely to be present. No line in `programs/aegis/src` changed this phase — the positive-
allowlist policy engine, vault sizing, and measured-delta accounting were already complete from
Phases 2/3. There is still no SDK/app yet.

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
