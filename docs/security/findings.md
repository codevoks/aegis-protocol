# Security Findings (Phase 10, updated Phase 13)

**Status honesty, up front** (per `docs/security/README.md`'s standing rule #4 and `AGENTS.md`
§59): Aegis is **not audited**, has **not been formally verified**, and this document does not
claim it is "secure." What follows is the result of one adversarial campaign — a completed
mitigation-removal proof for all 32 cataloged threats, nine mutation-validated global invariants,
manual review of the highest-risk instructions, and a 100,000+-operation stateful fuzz campaign —
run against a codebase that had already built its own adversarial test suite incrementally across
nine prior phases, not one being audited cold for the first time. That context matters for reading
the result below honestly.

## Summary

| ID | Severity | Component | Status |
|---|---|---|---|
| F-13-01 | Informational | `absorb_bad_debt` / `withdraw` (supply-side share dust) | **Confirmed harmless this phase.** Real, reproducible INV-ACC-06 violation, found by the Phase 13 release demo. Proven, not merely argued, to carry zero economic consequence at any future deposit size. No fix — see below. |
| F-10-02 | Low | `programs/aegis/src/instructions/borrow/repay.rs` | **Fixed (Phase 10).** Real, reproducible INV-ACC-06 violation found by the extended fuzz campaign against the unmutated program. Regression test + fix committed. |
| F-10-01 | Informational | `withdraw`/`borrow` free-liquidity checks | Confirmed redundant defense-in-depth, not a vulnerability — no fix needed |

## F-13-01 — bad-debt event can leave `fee_position` holding provably-worthless "ghost" supply shares

- **Severity:** Informational. Confirmed **not** exploitable and **not** merely bounded-and-small
  like F-10-02 — the implied value of the residual shares is proven to be exactly zero for a
  future deposit of *any* size, including `u64::MAX`, not just realistic ones.
- **Affected component:** the interaction between `absorb_bad_debt`'s protocol-first-loss step
  (`programs/aegis/src/instructions/liquidate/absorb_bad_debt.rs`) and `withdraw`'s
  `to_assets_down` share redemption (`programs/aegis/src/instructions/lend/withdraw.rs`,
  `crates/aegis-math/src/shares.rs`).
- **Threat / invariant:** T-17 (exploitable rounding) family / **INV-ACC-06** `[GLOBAL]`
  (`total_supply_shares == 0 ⟺ total_supply_assets == 0`).
- **Discovered via:** the Phase 13 release demo (`crates/aegis-test-kit/examples/phase13_demo.rs`,
  step 15/16) — not the fuzzer. A genuine bad-debt event (steps 10–13) burns most of
  `fee_position`'s supply shares in step 14's `absorb_bad_debt`, leaving a small nonzero dust
  remainder; the market's sole other lender then withdraws their entire share balance (step 15)
  and the demo's own final invariant check (step 16) caught the violation directly, rather than
  panicking uncaught.
- **Root cause:** `withdraw`'s `assets`-from-`shares` path computes `to_assets_down(shares,
  total_supply_assets, total_supply_shares)` (floored). When one lender holds effectively all of
  `total_supply_shares` (the rest being `fee_position`'s post-absorption dust), that lender's
  floor-rounded redemption can equal the market's *entire* remaining `total_supply_assets` exactly,
  because the dust shares' true fractional entitlement is below one base unit. The result:
  `total_supply_assets` reaches exactly `0` while `fee_position.supply_shares` (and therefore
  `total_supply_shares`) stays nonzero — INV-ACC-06's literal text is violated.
- **Why this is provably harmless, not merely small (verified, not assumed):** the dust shares'
  implied value after any future deposit `D` is `to_assets_down(dust, D, D·(dust + VIRTUAL_SHARES)
  + dust)`. For `dust < VIRTUAL_SHARES` (`1_000_000` — true by construction, since dust is
  whatever survived being *burned down* from a real share balance, always far below the virtual
  offset in every reachable case), this expression is bounded above by the asymptotic ratio
  `dust / (dust + VIRTUAL_SHARES) < 1` for every finite `D`, so the floor is `0` **for every
  possible future deposit, without exception** — not "usually," not "for realistic amounts."
  `tests/adversarial/orphaned_fee_shares.rs` verifies this two ways: a closed-form check swept
  across `D ∈ {1, u64::MAX}` for several dust magnitudes, and the same check re-run against the
  *real* on-chain dust value produced by a real `create_market → supply → seed debt → accrue →
  absorb_bad_debt → withdraw` sequence. No lender is ever diluted; no value is ever created or
  destroyed; the shares are a permanent, inert bookkeeping artifact.
- **Minimized reproduction:** `tests/adversarial/orphaned_fee_shares.rs::
  f_13_01_bad_debt_dust_in_fee_position_never_regains_value_at_any_future_deposit_size`.
- **Recommendation:** **no code change**, matching F-10-01's precedent for a confirmed-harmless
  finding. A future v2 could have `absorb_bad_debt` sweep any residual `fee_position` dust to
  exactly zero when `total_supply_assets` reaches zero, purely for bookkeeping cleanliness — never
  a correctness or safety requirement, since the value is already, and always will be, zero. Not
  implemented in Phase 13: it would touch `economic-model.md`'s frozen settlement formula
  (`§8.2`) for a purely cosmetic gain, which is exactly the kind of unforced formula change
  `AGENTS.md` §4 requires an ADR for, and Phase 13's mandate is reconciliation, not redesign.
- **Status:** Confirmed, documented, permanently regression-tested. Not fixed — not a
  vulnerability.

One genuine, previously-unknown protocol bug (F-10-02) was found and fixed this phase — see below
for the full account. The remaining question the phase's own acceptance rule raises ("is zero
*further* findings credible?") is addressed in "Why no additional protocol bugs is plausible here."

## F-10-02 — `repay` can permanently strand borrow-share "dust" with zero backing assets

- **Severity:** Low (the stranded amount is bounded to a sub-base-unit fraction of the loan asset
  in every case this phase observed; the defect is the *permanence* of the resulting invariant
  violation, not the size of any single instance).
- **Affected component:** `programs/aegis/src/instructions/borrow/repay.rs`.
- **Threat / invariant:** T-17 (exploitable rounding) / **INV-ACC-06** `[GLOBAL]`
  (`total_borrow_shares == 0 ⟺ total_borrow_assets == 0`).
- **Discovered via:** the stateful fuzzer's extended campaign (`tests/fuzz/`, NOT a mutation —
  found against the correct, unmutated on-chain artifact), seed 21, step 3628, across a
  100,000-operation run. This is exactly the class of bug `docs/testing-strategy.md` §5 says the
  fuzzer exists to find: "emergent multi-step bugs no hand-written test would think to write."
- **Root cause:** `repay`'s `assets`-denominated path computes `requested_shares =
  to_shares_down(assets, total_borrow_assets, total_borrow_shares)` (floored), clamps it to the
  position's real share balance, then recomputes `assets_to_pull = to_assets_up(clamped_shares,
  total_borrow_assets, total_borrow_shares)` (ceiled) from the *clamped* share count. When
  `total_borrow_assets` is already very small (a market repaid down close to zero) and a position
  holds effectively all of `total_borrow_shares`, `crates/aegis-math/src/shares.rs`'s
  `VIRTUAL_SHARES`/`VIRTUAL_ASSETS` offsets (`1_000_000`/`1`) dominate the ratio at these tiny
  magnitudes: the floored `requested_shares` can land strictly below the position's true
  remaining share balance, while the ceiled `assets_to_pull` recomputed from that smaller share
  count still equals the market's *entire* remaining `total_borrow_assets`. Net effect:
  `total_borrow_assets` reaches exactly `0` while a small, nonzero "dust" share remainder survives
  on the position and in `total_borrow_shares`.
- **Why this is permanent, not transient:** once `total_borrow_assets == 0`,
  `aegis_math::utilization()` reads `0` regardless of the dust shares outstanding, so
  `borrow_rate()` is `0` and no future `accrue_interest` call ever adds assets back —
  there is no code path that heals this state. The position's real, if tiny, remaining obligation
  becomes permanently uncollectible and INV-ACC-06 stays violated for the rest of the market's
  life.
- **Minimized reproduction:** `tests/adversarial/dust_debt.rs::
  a_dust_01_repay_of_the_last_borrower_never_strands_shares_without_assets` — seeds a sole
  borrower holding all `1_006_700` outstanding borrow shares against `total_borrow_assets = 1`
  (the exact state the fuzz trace reached), then repays `1` asset unit. Before the fix: leaves
  `total_borrow_shares = 3350` with `total_borrow_assets = 0`. Confirmed to fail against the
  pre-fix code, per the bug-fix-order discipline (`docs/security/mutation-report.md`'s
  procedure, applied here to a real finding rather than a deliberate mutation).
- **Fix:** when the repaying position holds *all* of `market.total_borrow_shares` (so no other
  position's claim is affected) and the computed `assets_to_pull` would already consume all of
  `market.total_borrow_assets`, `repay` now clamps to the position's *full* share balance instead
  of the floored `requested_shares`. `assets_to_pull` — what the payer is actually charged — is
  unchanged; only the bookkeeping write-off of already-worthless dust changes. Verified: the
  regression test now passes, the full offline suite (240 tests) passes unchanged, and a fresh
  100,000-operation extended campaign was re-run after the fix (see
  `docs/security/mutation-report.md`'s campaign-statistics section for that run's result).
- **Scope check — is the same pattern reachable elsewhere?** `programs/aegis/src/instructions/
  liquidate/liquidate.rs` computes `repay_shares` via a superficially similar
  `to_shares_down`-then-clamp step, but read directly (not assumed from the similarity alone), its
  accounting update is structurally different from `repay`'s: `apply_liquidation_accounting`
  decrements `total_borrow_assets` by `outcome.repay_assets` itself — the liquidation math's own
  output — never by a value *recomputed* from the clamped share count the way `repay`'s
  `assets_to_pull` is. The specific mechanism behind F-10-02 (an asset amount ceiling-recomputed
  from an already-floored, clamped share count, at magnitudes where the virtual offsets dominate)
  does not have a direct analog on this path, and `outcome.repay_assets` is independently bounded
  by `max_repay(debt_assets, ...)`, which is itself derived from `position.borrow_shares` — so the
  clamp against `position.borrow_shares` should rarely if ever actually bind. This is a narrower
  and more reassuring conclusion than "the same pattern exists," reached by reading the code
  rather than by pattern-matching on the presence of `to_shares_down`; it is not a proof of
  absence, and `liquidate`'s share/asset pairing deserves the same numeric-boundary scrutiny in a
  future pass regardless.
- **Status:** Fixed and regression-tested this phase.

## F-10-01 — `withdraw`'s and `borrow`'s free-liquidity `require!` checks are redundant with the token program's own balance enforcement

- **Severity:** Informational (confirms a defense-in-depth property; not a vulnerability).
- **Affected component:** `programs/aegis/src/instructions/lend/withdraw.rs` (the
  `assets_out <= free_liquidity` check) and `programs/aegis/src/instructions/borrow/borrow.rs::
  compute_borrow` (the analogous check for `borrow`).
- **Threat / invariant:** Adjacent to T-17 (rounding/liquidity exploits); the checks exist to
  protect INV-CUS-01/INV-ACC-03.
- **Discovered via:** mutation validation (`docs/security/mutation-report.md`, "Finding 2"). The
  literal mutations `docs/phases/phase-10-security.md`'s own table describes — "skip the
  free-liquidity check in `withdraw`" and "remove the borrow liquidity check" — were applied,
  rebuilt into the real on-chain artifact, and run against the full 20,000-operation fuzz budget.
  Neither produced a detectable invariant violation.
- **Analysis:** both checks gate an *outbound* token transfer (`transfer_checked_out`), which CPIs
  into the real SPL Token / Token-2022 program. That program independently refuses to transfer more
  than the source account's actual balance. Whenever INV-CUS-01 already holds going into the
  instruction (which this fuzzer continuously re-verifies after every operation, and which every
  successful prior instruction is required to preserve), `loan_vault.amount` is *exactly*
  `total_supply_assets - total_borrow_assets` — precisely the quantity each removed `require!` was
  bounding the request against. Removing Aegis's own check therefore changes nothing about what can
  actually leave the vault under normal operation: the state mutations that follow the removed
  check (decrementing `total_supply_assets`/incrementing `total_borrow_assets`) are staged in
  memory, but if the subsequent `transfer_checked_out` CPI fails (which it must, once the request
  exceeds the vault's real balance), Solana's own transaction atomicity discards every staged
  mutation along with it — nothing commits.
- **Impact if this analysis is wrong:** none identified. To be thorough, both checks were also
  tested by directly corrupting the accounting instead of the precondition (double-crediting
  `total_borrow_assets` in `borrow`, and skipping the `total_supply_assets` decrement in
  `withdraw`) — both were caught immediately (464 and 115 operations respectively). This confirms
  the invariants themselves (INV-CUS-01, INV-ACC-03) are soundly protected; only the specific,
  literal mutation of "remove the precondition `require!`" is inert given the redundant CPI-level
  backstop.
- **Recommendation:** **no code change.** These checks should stay exactly as they are — removing
  them would trade a clear, specific `AegisError::InsufficientLiquidity` for an opaque SPL Token
  program error on the same rejected transaction, which is a worse error-reporting experience for
  callers even though it is not a security regression. Documented here so a future contributor does
  not mistake "the fuzzer couldn't prove this check matters" for "this check doesn't matter" — it
  matters for error clarity and for defense-in-depth against any future change to the CPI-level
  backstop (e.g. a hypothetical future token program with weaker balance enforcement).
- **Status:** Closed — informational, no fix required. Recorded for transparency per the "zero
  findings is not credible" standing rule; this is the phase's positive finding in that spirit.

## Why the search was not shallow, and why F-10-02 is the only protocol bug found

`docs/phases/phase-10-security.md` correctly warns that a security document reporting zero findings
"usually means the search was too shallow." This campaign found exactly one genuine, previously
unknown protocol bug (F-10-02, above) — not zero, and not many. Both halves of that claim need
evidence, stated plainly so the reader can judge for themselves rather than take it on faith:

1. **The search was demonstrably not shallow.** F-10-02 itself is the strongest evidence: it
   surfaced on an extended campaign run after the fuzzer's own market configuration (interest
   rates) and liquidation-targeting had already been strengthened for the mutation-validation work
   — a first extended run with the fuzzer's initial configuration had found nothing. Separately,
   nine deliberate mutations were injected directly into the real on-chain artifact and all nine
   were caught (two only after genuine fuzzer improvements, honestly recorded in
   `docs/security/mutation-report.md` rather than smoothed over) — proof the invariant net actually
   detects real violations, not merely that it never fires.
2. **This was not the first adversarial pass on this codebase.** `docs/project-status.md`'s own
   phase-by-phase evidence shows that every prior phase (2 through 9) built and ran its own
   `A-*`/`U-*`/`P-*` adversarial and property tests for the mechanisms it introduced, *as part of*
   implementing them — not deferred to a single end-of-project audit. By the time Phase 10 began,
   all 32 threats in `docs/threat-model.md` and all 87 invariants in `docs/invariants.md` already
   had a mapped, passing, specific-error-asserting test (verified directly in this phase — see
   `docs/security/threat-traceability.md`). That F-10-02 still existed under that much prior
   scrutiny is itself informative: it was reachable only through a genuinely emergent, multi-step,
   cross-instruction sequence — exactly the class of bug `docs/testing-strategy.md` §5 says a
   stateful fuzzer exists to find and a hand-written scenario would not think to construct.
3. **After the fix, a fresh extended campaign was re-run** before this document was finalized
   (`docs/security/mutation-report.md`'s campaign-statistics section has the exact result) to check
   for further violations.

None of this makes "no further undiscovered bugs" a guarantee — an unaudited protocol remains
unaudited, and this document says so throughout. It is the reason this specific result (one real,
fixed finding; nothing further found on re-run) is a documented, evidenced outcome rather than an
unexamined default.

## Findings from earlier phases, for context

`docs/project-status.md` records two real bugs found and fixed during Phase 9's own UI exercise (a
duplicate-mutable-account error from reusing an address, and an oracle-staleness failure from a
wall-clock/validator-clock mismatch) — both already documented and fixed there, not repeated here
since they predate Phase 10 and are not this phase's discovery. Mentioned only so a reader of this
document sees that "bugs get found and fixed" is this project's established pattern, not something
Phase 10 introduces for the first time.
