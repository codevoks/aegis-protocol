# Phase 2 — State, PDAs and Custody Primitives

**Status: NOT STARTED.** **Prerequisite: Phase 1 complete and tagged.**

## Scope
1. `Protocol`, `Market`, `Position` account structs exactly as specified in `account-model.md` §3–5.
2. `initialize_protocol`, `create_market`, `init_position`.
3. Vault creation (both PDA token accounts) with the `Market` PDA as authority.
4. The **Token-2022 extension policy engine** (`token/policy.rs`) enforced at `create_market`.
5. Full parameter bounds validation, including the derived `LT·(WAD+b) < WAD` constraint.
6. The single `AegisError` enum with the banded code layout from `architecture.md` §8.
7. `events.rs` with `ProtocolInitialized`, `MarketCreated`, `PositionInitialized`.
8. `aegis-test-kit`: LiteSVM bootstrap, SPL and Token-2022 mint factories.

## Explicit NON-scope
No token transfers. No deposits, withdrawals, borrowing, lending, interest, oracle, or liquidation.
No `set_*` admin mutation instructions (Phase 12) beyond what `create_market` needs. No share math.
`Market`'s accounting scalars are initialized to zero and otherwise untouched.

## Files
`programs/aegis/src/{lib.rs, constants.rs, error.rs, events.rs}` ·
`state/{protocol.rs, market.rs, position.rs}` ·
`instructions/admin/{initialize_protocol.rs, create_market.rs}` ·
`instructions/position/init_position.rs` · `token/policy.rs` · `guards.rs` ·
`crates/aegis-test-kit/src/{svm.rs, mints.rs, market.rs}`

## Concepts demonstrated
Solana account model · PDA derivation and canonical bumps · Anchor account constraints and `has_one` ·
program ownership and discriminators · rent and account allocation · Token-2022 TLV extension parsing ·
`ExtensionType::try_calculate_account_len` · content-addressed account design · parameter validation
as a security boundary.

## Dependencies
Phase 1 (`mul_div_*` for bound checks; workspace; CI).

## Security work
- Every account constraint from `instruction-catalogue.md` §1, §6, §9.
- The **positive** extension allowlist — reject unknown extension discriminants by default.
- `ack_freeze_authority` handling.
- `collateral_mint != loan_mint`.
- Mint owner matches the passed token program.
- Vault length computed from extensions, never hardcoded to 165.
- No `init_if_needed` (CI-enforced).

## Tests
`U-ACCT-01` (`_reserved` zero), `U-ACCT-02` (no realloc needed), `U-LIFE-02` (seed prefixes distinct),
`A-AUTH-01` (non-admin `create_market` fails), `A-AUTH-06` (attacker-owned accounts rejected),
`A-LIFE-01` (reinit fails), `A-LIFE-03` (non-canonical bump fails), `A-ADM-04` (out-of-bounds
parameter sweep, **including a parameter set violating `LT·(1+b) < WAD`**), `A-CUS-03` (vault authority
is the `Market` PDA), `A-TOK-01..05` (extension rejections + unknown-extension case),
`A-TOK-07` (freeze-authority acknowledgement), `I-DEPLOY-01` (post-deploy admin assertion).

## Demo
`make demo` prints: protocol initialized, one SPL market and one Token-2022 market created with their
full parameter snapshots, positions initialized, and a table of rejected mints with the specific
rejection reason.

## Documentation
Update `project-status.md`. Add ADR only if a deviation occurs.

## Acceptance criteria
- [ ] All three account structs match `account-model.md` byte-for-byte in field order and size.
- [ ] `create_market` rejects every Tier C extension and every unknown extension.
- [ ] A transfer-fee mint is rejected as the loan asset and accepted as collateral.
- [ ] All parameter bounds enforced, including the derived liquidation bound.
- [ ] Vaults created with the `Market` PDA as authority and the correct Token-2022 length.
- [ ] The fee `Position` is created by `create_market`.
- [ ] Two markets can be created for the same asset pair with different `config_id`.
- [ ] INV-ACCT-01..09, INV-LIFE-01/04/05/06, INV-ADM-05, INV-LIQ-06 tested.
- [ ] Universal checklist (`phase-roadmap.md`) satisfied.
- [ ] Tag `phase-02-state`.

## Evidence
Test output; the `MarketCreated` event for both market types; the rejection table; account sizes.

**STOP after this phase.**
