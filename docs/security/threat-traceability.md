# Threat Traceability Matrix (Phase 10)

**Status: current as of Phase 10.** Built by reading `docs/threat-model.md` §2 (the frozen,
Phase-0 threat catalogue, T-01..T-32) directly and cross-checking every cited test ID against the
actual test sources with `grep`, not from memory or a prior session. Where a cell says "existing
(early coverage)", the test was built during an earlier phase — this repository's established
practice of exercising a mitigation as soon as the underlying mechanism exists, documented at the
time in `docs/project-status.md`'s per-phase evidence sections, not newly discovered here.

Every row ends with at least one **named, specific-error-asserting** test — never a bare
`.is_err()`. "Non-vacuous" means the mitigation-removal procedure in
`docs/security/mutation-report.md` (or, for the nine `[GLOBAL]` invariants, the fuzzer's own
mutation-validation table) was run against that exact test and the test failed for the expected
reason with the mitigation removed.

| Threat | Attack (short) | Mitigation | Expected error | Test(s) | Status |
|---|---|---|---|---|---|
| T-01 Missing signer check | Move funds without the owner's signature | `Signer` + `has_one = owner` (INV-AUTH-02/03) | `NotPositionOwner` | `A-AUTH-02` (`tests/phase2_adversarial.rs` family) | Existing (Phase 2/3), non-vacuous by construction (Anchor's own `Signer` check) |
| T-02 Missing/incorrect account owner validation | Attacker-owned look-alike `Market`/`Position` | `Account<'info,T>` owner+discriminator check | Anchor's `AccountOwnedByWrongProgram` / `AccountDiscriminatorMismatch` | `A-AUTH-06` | Existing (Phase 2), non-vacuous by construction |
| T-03 Arbitrary/substituted account | Redirect a vault to an attacker account | Canonical PDA **and** `has_one` (double validation) | `VaultMismatch` / Anchor seed-constraint error | `A-CUS-01` | Existing (Phase 2/3) |
| T-04 Wrong mint | Deposit a worthless mint, borrow real value | Pinned mint + `transfer_checked` | `VaultMintMismatch` | `A-CUS-06` | Existing (Phase 3) |
| T-05 Wrong token program | Present a Token-2022 account under the legacy program | `market.*_token_program` pinned + `require_keys_eq!` | `TokenProgramMismatch` | `A-TOK-08`, `A-TOK-09` | Existing (Phase 3) |
| T-06 Fake oracle account | Arbitrary attacker-supplied price account | O-1/O-2 (owner + discriminator) | `OracleAccountOwnerMismatch` / `OracleAccountInvalidData` | `A-ORACLE-06` (+ `A-ORACLE-07..12` for the rest of the O-checks) | Existing (Phase 5) |
| T-07 Mismatched oracle feed | Pass a $1 asset's feed as a $100k asset's | O-3 (feed ID identity) + O-11 (distinct accounts) | `OracleFeedMismatch` / `OracleDuplicatePriceAccounts` | `A-ORACLE-07`, `A-ORACLE-12` | Existing (Phase 5) |
| T-08 Stale price | Borrow/liquidate on a stale price | O-5, unix-seconds | `OraclePriceStale` | `A-ORACLE-03`, `a_oracle_03_boundary_age_exactly_at_threshold_vs_plus_one` | Existing (Phase 5) — the boundary test `threat-model.md`/`oracle-design.md` §7 call for already existed; verified present, not a gap |
| T-09 PDA seed collision | One account type aliasing another | Distinct literal seed prefixes, fixed-width variable seeds | `U-LIFE-02` (property), `A-LIFE-03` (Anchor seed-constraint error) | `U-LIFE-02`, `A-LIFE-03` | Existing (Phase 2) |
| T-10 Non-canonical bump | Multiple valid addresses for one account | Canonical bump stored + reused (`bump = acct.bump`) | Anchor's `ConstraintSeeds` | `A-LIFE-03` | Existing (Phase 2) |
| T-11 Duplicate mutable accounts | `position == fee_position` to double-count | Anchor 1.0 default dup-mutable rejection, PDA-derived `fee_position` | Anchor's `AccountDuplicateReuse` | `A-ACC-01` | Existing (Phase 4/6) |
| T-12 Account reinitialization | Reset an indebted position to zero | No `init_if_needed`; Anchor `init` fails on an existing account | Anchor's `AccountAlreadyInUse`/`0x0` init error | `A-LIFE-01`; `CI-NOINITIF` | Existing (Phase 2) |
| T-13 Unsafe close/revival | Close with debt, or revive with stale data | Exact-zero close precondition + Anchor `close =` | `PositionNotEmpty` | `U-LIFE-01`, `A-LIFE-02` | Existing (Phase 3) |
| T-14 Stale account state after CPI | Credit requested, not received, amount on a fee mint | Mandatory `vault.reload()` + measured delta | (no error — the POSITIVE test: credited < requested, proven by assertion, not a revert) | `U-TOK-02`, `A-TOK-10`, `A-TOK-11` | Existing (Phase 3/7) |
| T-15 Privilege propagation via CPI | Callback drains the liquidator's/vault's tokens | No signer forwarded to the callback (INV-AUTH-07) | `LiquidationCallbackRepaymentShortfall` / atomic revert | `A-CPI-01..04`, `A-AUTH-07` | Existing (Phase 8), **already proven atomic** (zero state diff, not just "reverted") |
| T-16 Integer overflow/truncation | Wrapped values → free money | `overflow-checks=true`, `mul_div_*` 256-bit intermediates | Rust panic → transaction abort (`ArithmeticOverflow` where reachable via a checked path) | `P-ARITH-1..3` (`crates/aegis-math/tests/property.rs`) | Existing (Phase 1/2) |
| T-17 Exploitable rounding | Extract 1 base unit per round-trip until drained | Rounding law (14 directions, protocol-favoring) + virtual offsets | (property assertion, not a revert) | `P-SHARE-1..4`; **Phase 10 adds:** the stateful fuzzer's value-creation search (`tests/fuzz/ledger.rs`) and `A-DUST-01` (`tests/adversarial/dust_debt.rs`) | Existing (Phase 4) + **new this phase — the value-creation search found and this phase fixed a real T-17 bug, `docs/security/findings.md` F-10-02** |
| T-18 First-depositor share inflation | Inflate share price against the next depositor | `VIRTUAL_SHARES`/`VIRTUAL_ASSETS` offsets + INV-CUS-08 | (property: attack unprofitable with offsets enabled) | `A-SHARE-01` | Existing (Phase 4) |
| T-19 Decimal mismatch | Value a 9dp asset as 6dp → 1000x mispricing | Decimals cached from mints, applied explicitly | (property: correct valuation across the decimals matrix) | `P-VAL-1` | Existing (Phase 5) |
| T-20 Oracle price manipulation (real market) | Move the real underlying market | Outside Aegis's control; `max_conf_bps` + conservative LTV | N/A — **documented accepted residual risk**, not testable in-protocol | — | Not testable by design (`threat-model.md` §2/§4); no test expected |
| T-21 Oracle downtime → bad debt | Outage blocks liquidation while price moves | Fail-closed + `absorb_bad_debt` needs no oracle | (positive test: bad debt recognized correctly across the outage) | `A-ORACLE-10` | Existing (Phase 5) |
| T-22 Self-liquidation | Owner liquidates their own position | Not blocked; analyzed as unprofitable relative to repaying | (property: self-liquidation strictly worse than repay) | `U-LIQ-07` | Existing (Phase 6) |
| T-23 Liquidation griefing | Repeated maximal partial liquidations | `close_factor` cap + HF-must-improve (INV-LIQ-05) + `min_debt` | `RepayExceedsMaxRepay` (over-cap attempt) | `U-LIQ-04`, `P-LIQ-1` | Existing (Phase 6) |
| T-24 Death-spiral liquidation | Partial liquidation reduces HF further | Derived bound `LT·(1+b) < WAD` enforced at `create_market`/`set_market_params` | `LiquidationBonusExceedsThresholdBound` | `P-LIQ-1`, `P-LIQ-4`, `A-ADM-04` | Existing (Phase 2/6) |
| T-25 Unliquidatable dust | Many sub-liquidatable positions accumulate bad debt | `min_debt` floor on borrow and partial liquidation | `DebtBelowMinimum` | `U-BORROW-02`, `U-LIQ-04` | Existing (Phase 4/6) |
| T-26 Hostile token extension | Permanent delegate / pausable / close-authority mint | Positive allowlist at `create_market` | `UnsupportedTokenExtension` | `A-TOK-01..05` | Existing (Phase 2/7) |
| T-27 Compute exhaustion / DoS | An instruction too expensive to ever land | No unbounded loops; fixed account counts; transfer hooks rejected | N/A (structural — see Gap G-1) | `A-TOK-01` (hook rejection); full CU regression is **explicitly Phase 11 scope** (INV-RES-01/04) | Existing (hook-rejection facet); CU-benchmark facet correctly deferred, not skipped |
| T-28 Account contention DoS | Spam `accrue_interest` to starve a market | Isolation (INV-RES-03) + collateral ops don't write `Market` (INV-RES-02) | Anchor account-meta writability assertion | `A-PAR-01`, `A-PAR-02` | Existing (Phase 3/6) |
| T-29 Unsafe admin configuration | `max_ltv=99%` or `liq_bonus=90%` | On-chain bounds validated on every write (INV-ADM-05) | `InvalidMaxLtvOrThreshold` / `InvalidLiqBonus` / etc. | `A-ADM-04` | Existing (Phase 2) |
| T-30 Compromised upgrade authority | Total loss via a malicious program upgrade | Not a code problem; progressive hardening (`governance.md`) | N/A — **documented accepted residual risk**, not testable in-protocol | — | Not testable by design; no test expected |
| T-31 Malicious external integration (Phase 8) | Reentrant/hostile liquidation callback | No signer forwarded; guard; post-condition re-verification | `LiquidationCallbackReentrancy` / `LiquidationCallbackRepaymentShortfall` | `A-CPI-01..04` | **Already implemented and tested (Phase 8)** — threat-model.md's own header says so |
| T-32 Front-running protocol initialization | Attacker calls `initialize_protocol` first | Deploy-then-initialize as one operational step; post-deploy admin assertion | (positive test: admin is the deploying admin) | `I-DEPLOY-01` | Existing (Phase 2) |

## Gaps closed this phase

- **G-1 (T-27):** the CU-quantified facet of T-27 (`INV-RES-01`, `B-CU-*`) is formally assigned to
  Phase 11 in `docs/invariants.md`'s own per-phase column, and `docs/phase-roadmap.md` lists CU
  benchmarking under Phase 11, not Phase 10. This is not a Phase 10 gap being silently dropped —
  it is out of this phase's scope by the frozen roadmap's own division of labor, exactly the way
  `docs/project-status.md` already treats `INV-RES-01` for every earlier phase. What Phase 10 adds
  instead: the stateful fuzzer's `AccrueInterest`/`Liquidate` actions run with the same production
  compute-unit budgets `aegis-test-kit` already uses (`LIQUIDATE_COMPUTE_UNIT_LIMIT`, etc.), so a
  regression that made an instruction blow its existing, measured budget under adversarial
  argument choices would already surface as an unexpected transaction failure during the campaign
  — a structural safety net, not a quantified CU claim.
- **G-2 (A-SOLV-01 citation):** `docs/invariants.md`'s INV-SOLV-01 row cites `A-SOLV-01`, which
  had never been literally cited anywhere in source (the actual on-chain enforcement existed and
  was tested under descriptive names since Phase 5, and `docs/project-status.md` already recorded
  this naming divergence explicitly rather than silently). Closed by adding an `A-SOLV-01` doc
  comment at both of INV-SOLV-01's enforcement sites (`programs/aegis/src/instructions/borrow/
  borrow.rs`, `programs/aegis/src/instructions/collateral/withdraw_collateral.rs`) — a citation
  fix, not a new test, since the behavior was already tested.
- **G-3 (F-INV-01..07 citations):** `docs/invariants.md` cites `F-INV-01..07` for the nine
  `[GLOBAL]` invariants — fuzz-test IDs that could not exist before this phase built the fuzzer.
  Closed by citing each ID in `crates/aegis-test-kit/src/invariants.rs`'s corresponding checker
  function, next to the exact code that implements it.

## Non-vacuity (mitigation-removal) evidence

Per-threat mitigation-removal results rest on two layers of evidence:

1. **Historical.** Every existing test's non-vacuity was already established in the phase that
   introduced it — the invariant-preservation rule in `AGENTS.md` §8 already requires this for
   every test in this repository, and it is not re-litigated in full for all 32 threats here.
2. **Fresh, this phase.** Three representative threats were spot-checked by live mitigation
   removal against the rebuilt on-chain artifact (the same `anchor build -p aegis` → run the exact
   test → observe the exposed vulnerability → revert → rebuild procedure
   `docs/security/mutation-report.md` uses for the nine `[GLOBAL]` invariants), chosen to span
   different families rather than one:
   - **T-01** (`has_one = owner` removed from `withdraw_collateral`): `non_owner_withdraw_fails`
     failed — a non-owner's withdrawal succeeded, exposing exactly T-01's impact.
   - **T-06 / INV-LIQ-01** (`is_liquidatable(hf_before)` check removed from `liquidate`):
     `a_liq_01_healthy_position_cannot_be_liquidated_and_nothing_mutates` failed — a strictly
     healthy position was liquidated.
   - **T-06 / O-1** (the Pyth account owner check removed from `oracle::pyth::PythPull::
     read_price`): `a_oracle_06_wrong_owner_account_is_rejected` failed — a wrong-owner price
     account was accepted and `borrow` succeeded against it.

   All three were reverted immediately after and the baseline reconfirmed clean.

The **nine `[GLOBAL]` invariants** get a further, independent non-vacuity proof this phase via the
stateful fuzzer rather than a single hand-written scenario — see `docs/security/mutation-report.md`
for the full table (mutation, seed, ops-to-detect, minimized trace).
