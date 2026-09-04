# Phase 8 — Composability and Liquidation Routing

**Status: NOT STARTED.** **Prerequisite: Phase 7 complete and tagged.**
**Research gates RV-6 and RV-8 must be closed first. RV-6 is a design gate, not a formality.**

> The product problem: a liquidator must pre-fund the loan asset to liquidate. That is real capital
> friction and it directly weakens the protocol's most important safety mechanism. The callback solves
> a genuine problem, which is why it is in scope while a general flash-loan facility is not.

## Scope
1. **Close RV-6:** determine authoritatively whether the Solana runtime permits `A → B → A` CPI
   reentrancy (non-self-recursive). Record the finding with a primary source. **Aegis must not depend
   on the answer** — but the design and its documentation must state it correctly.
2. Add an **optional callback** to `liquidate`: after seizing collateral and before requiring
   repayment, CPI into a liquidator-specified program so it can swap the collateral and repay in one
   transaction.
3. Re-read all state and re-verify all post-conditions after the callback returns.
4. `bots/liquidator/`: a TypeScript keeper on `@solana/kit` that scans positions, computes health
   off-chain, and submits liquidations.
5. An example callback program in `labs/` using a deterministic local price (zero-cost path).
6. **Optional, network-tagged:** Surfpool mainnet-fork test performing a real Jupiter route.

## Explicit NON-scope
No general flash loans. No AMM implementation. No Jupiter dependency in the required test path. The
callback must remain strictly optional — omitting it must leave `liquidate` behaving exactly as in
Phase 6.

## Files
`instructions/liquidate/liquidate.rs` (callback branch) · `bots/liquidator/` ·
`labs/example-liquidator/` · `tests/network/jupiter_route.rs` (`#[ignore]`)

## Concepts demonstrated
Cross-program composability · CPI into an untrusted program · reentrancy-safe design ·
post-condition verification as the primary defense · off-chain keeper architecture · transaction
composition · integrating external liquidity without depending on it.

## Security design (the heart of this phase)

The callback target is **trusted for nothing**. Defenses, in order of importance:

1. **No signer is forwarded.** Neither the market PDA's signature nor the liquidator's is passed to
   the callback (INV-AUTH-07). The callback receives only the accounts it needs to do its own work.
2. **All state is re-read after the callback returns.** Any value cached before the CPI is discarded.
   This is the defense that holds *regardless* of the RV-6 answer, which is why it is mandatory rather
   than conditional.
3. **All post-conditions are re-verified after the callback**: the loan vault received exactly the
   required repayment (measured delta), the position's state matches what was computed, and
   INV-CUS-01/02 hold.
4. **The callback is opt-in per transaction.** A liquidator who passes no callback program gets the
   Phase 6 behavior byte-for-byte.
5. **A state-machine guard flag** prevents a nested `liquidate` on the same market within the
   callback, independent of runtime reentrancy semantics.

Defense 2 is stated as a rule rather than an optimization because "read state before the CPI, act on it
after" is exactly the bug pattern that makes callbacks dangerous.

## Tests
`A-CPI-01` (hostile callback attempts to move vault funds — must fail),
`A-CPI-02` (callback attempts to reenter `liquidate` — must fail),
`A-CPI-03` (callback consumes the CU budget — transaction fails cleanly, no partial state),
`A-CPI-04` (callback returns without repaying — must fail on the post-condition),
`I-LIQ-CB-01` (honest callback: seize, swap locally, repay, all in one transaction),
`I-LIQ-CB-02` (callback omitted → behavior identical to Phase 6).
Optional/network: `N-JUP-01` (real Jupiter route on a Surfpool mainnet fork).

## Acceptance criteria
- [ ] RV-6 closed with a primary source, recorded in `ecosystem-research.md`.
- [ ] RV-8 closed (current Jupiter integration surface).
- [ ] The callback forwards no signer and re-verifies every post-condition.
- [ ] All four `A-CPI-*` attacks fail correctly with specific errors.
- [ ] Omitting the callback reproduces Phase 6 behavior exactly.
- [ ] The liquidator bot successfully liquidates in the local environment with **no network**.
- [ ] The Jupiter test is `#[ignore]`/network-tagged and excluded from `make test`.
- [ ] `make test` still passes fully offline.
- [ ] INV-AUTH-07, INV-RES-07 tested.
- [ ] Universal checklist satisfied. Tag `phase-08-composability`.

**STOP after this phase.**
