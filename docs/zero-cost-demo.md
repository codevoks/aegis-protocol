# Aegis — Zero-Cost Local Architecture

**Status: FROZEN (Phase 0). NFR-4 is non-negotiable.**

> **Requirement:** a complete, meaningful demonstration of Aegis — including oracles, Token-2022,
> liquidation, bad debt, the SDK and the UI — must run on a laptop with **no RPC provider, no API key,
> no faucet, no airdrop, no devnet, and no money**.

---

## 1. Why this is an architectural constraint, not a convenience

It forces three properties that a devnet-dependent design would let slide:

1. **Determinism.** Prices, time, and account state are all set explicitly, so every test is
   reproducible and every failure is debuggable. Devnet-dependent oracle tests are flaky by
   construction.
2. **Adversarial reachability.** Stale prices, wide confidence, oracle outages, extreme volatility,
   and bad debt are *trivial* to produce locally and nearly impossible to produce on demand on a
   public cluster. The security campaign in Phase 10 only exists because of this constraint.
3. **Reviewability.** Anyone can `git clone && make test` and reproduce every claim in the repository
   in minutes. A protocol whose evidence cannot be independently reproduced is an assertion, not
   evidence.

---

## 2. How each external dependency is eliminated

| Dependency | Naive approach | Aegis approach | Cost |
|---|---|---|---|
| Cluster / RPC | Devnet or a paid RPC | **LiteSVM** (in-process SVM) for tiers 1–4; **Surfpool in pure local mode** for tier 5 | $0 |
| SOL for fees and rent | Faucet / airdrop | LiteSVM `airdrop` writes lamports directly; Surfpool funds local keypairs | $0 |
| Token mints | Devnet mints | Mints created locally in the harness, SPL **and** Token-2022 with chosen extensions | $0 |
| **Oracle prices** | Pyth on devnet + Hermes API key | **Byte-exact `PriceUpdateV2` account injection** (ADR-0008). Reading a pull-oracle price is an *account read, not a CPI*, so the Pyth program need not even be deployed | $0 |
| Time | Waiting | `svm.warp_to_slot` / clock manipulation — a year of interest accrual in microseconds | $0 |
| Swap liquidity (Phase 8) | Jupiter on mainnet | Core path uses a **deterministic local price** for the callback; the real Jupiter route is an **optional, network-tagged** test | $0 for the required path |
| Frontend backend | Hosted RPC | Next.js against local Surfpool | $0 |

The oracle row is the one that usually forces projects onto a network, and it is fully solved by
fixture injection. This is the practical payoff of ADR-0008 beyond its security argument.

---

## 3. What runs where

| Tier | Runtime | Network needed | In default `make test` |
|---|---|---|---|
| 1 — `aegis-math` | native Rust | no | **yes** |
| 2 — Mollusk | in-process | no | **yes** |
| 3 — LiteSVM integration/adversarial | in-process | no | **yes** |
| 4 — Invariant fuzzer | in-process | no | **yes** (short budget; long budget nightly) |
| 5 — Surfpool local | local validator | no | **yes** |
| Demo scenario | Surfpool local + UI | no | **yes** (`make demo`) |
| **Optional:** Surfpool mainnet-fork (Jupiter) | local + public RPC for account fetch | **yes** | no — `#[ignore]`, `--network` |
| **Optional:** devnet Pyth/Hermes | devnet | **yes** | no |

**Rule:** anything in the optional tier must be skippable without any test failing and without any
documented claim becoming unsupported. If a claim in the README depends on the optional tier, the
claim is wrong or the tier is misclassified.

---

## 4. Commands

```bash
make setup      # install/verify toolchain; prints exact versions (Phase 1 re-verification)
make build      # anchor build
make test       # tiers 1-5, offline, deterministic          <-- the load-bearing command
make fuzz       # extended invariant fuzz campaign
make bench      # CU benchmarks -> benchmarks/cu.json
make demo       # scripted end-to-end scenario against local Surfpool
make app        # UI against local Surfpool
make test-network  # OPTIONAL tier; requires an RPC endpoint
```

`make test` must pass on a clean clone with no configuration, no environment variables, and no network
access. **This is a Phase 1 acceptance criterion and is re-verified at every phase.**

---

## 5. The demo scenario (`make demo`)

A single scripted run that exercises the whole protocol and prints invariant checks at each step:

```
 1. Create SPL mints (USDC-like 6dp) and a Token-2022 mint with a transfer fee (collateral)
 2. Initialize protocol; create market SOL/USDC @ max_ltv 0.75, LT 0.80, bonus 0.05
 3. Inject Pyth prices: SOL = $150.00 ± $0.30, USDC = $1.0000 ± $0.0002
 4. Lender supplies 10,000 USDC                              → assert INV-CUS-01, INV-ACC-01
 5. Borrower deposits 10 SOL collateral                      → assert INV-CUS-02
 6. Borrower borrows 900 USDC (HF ≈ 1.33)                    → assert INV-SOLV-01
 7. Warp 30 days; accrue interest; show utilization and APY  → assert INV-ACC-04
 8. Attempt to borrow beyond max_ltv                         → EXPECT FAILURE
 9. Set the price stale; attempt to borrow                   → EXPECT FAILURE (fail closed)
10. With the same stale price, repay and deposit collateral  → EXPECT SUCCESS (risk-reducing)
11. Restore the price; drop SOL to $95.00 (HF ≈ 0.84)        → position becomes liquidatable
12. Liquidator liquidates; show seizure, bonus, protocol cut → assert INV-LIQ-*, INV-CUS-01/02
13. Crash SOL to $40; liquidate to zero collateral           → bad debt created
14. absorb_bad_debt: protocol fee shares burned first        → assert INV-SOLV-04/06
15. Lender withdraws; show the realized socialized loss
16. Print the full invariant report and the CU used per instruction
```

Steps 9 and 10 together are the demo's most important moment: they show the oracle failing closed for
risk-increasing operations *and* staying open for risk-reducing ones, which is the protocol's central
safety property and is invisible in a happy-path demo.

Every step prints its state transition, so the transcript is itself a readable artifact.

---

## 6. Fixture design

`crates/aegis-test-kit` is the single source of local truth:

| Module | Provides |
|---|---|
| `svm.rs` | Deploy the program; deterministic keypairs from fixed seeds; funded users |
| `mints.rs` | SPL mints; Token-2022 mints with a *chosen* extension set (including deliberately rejected ones for `A-TOK-*`) |
| `pyth_fixture.rs` | Byte-exact price-update accounts; `set_price`, `set_stale`, `set_wide_confidence`, `set_wrong_feed`, `set_absurd` |
| `market.rs` | Market creation with the reference parameter set or an override |
| `invariants.rs` | `assert_all()` — the nine **[GLOBAL]** invariants |
| `scenarios.rs` | Reusable multi-step flows shared by tests and by `make demo` |

**All keypairs are derived from fixed seeds.** No `Keypair::new()` in fixtures — a failing test must
be reproducible from its seed alone. Anything else makes fuzz failures unshrinkable.

---

## 7. Explicitly permitted paid/network options

These may exist, must never be required, and must be clearly labelled:

- A devnet deployment with real Pyth feeds, for a live demo link.
- A hosted UI pointed at devnet.
- Surfpool mainnet-fork tests for Phase 8 Jupiter routing.
- A paid RPC in a personal `.env` that is **never committed** and never referenced by a default path.

**Guard:** CI runs with no secrets configured. If a required test needs a secret, CI fails — which is
the enforcement mechanism for this whole document, rather than a promise in a README.

---

## 8. Failure modes to avoid

Recorded because each is an easy, common way to break NFR-4 without noticing:

| Anti-pattern | Why it breaks zero-cost |
|---|---|
| A test that fetches a real Pyth price account | Requires an RPC; flaky; non-deterministic |
| Hardcoding a devnet program ID or mint address | Not reproducible locally |
| A demo that needs an airdrop from a faucet | Rate-limited; frequently down |
| A frontend defaulting to a hosted RPC | Reviewer cannot run it offline |
| Tests that depend on wall-clock time | Non-deterministic; use clock warping |
| Committing a `.env` with an API key | Violates NFR-11 and normalizes secret handling |
| Random keypairs in fixtures | Unreproducible failures |
| A required test tagged `#[ignore]` "for now" | Silently removes coverage; the traceability check catches this |
