# ADR-0005 — Collateral is escrowed and never lent; vaults are explicit PDA token accounts

**Status:** Accepted · **Date:** 2026-09-04 · **Phase:** 0

Two related custody decisions.

---

## Part A — Collateral is never lent (no re-hypothecation)

### Context
In Aave-style designs, deposited collateral is simultaneously supply liquidity and can be borrowed by
others. It is more capital-efficient and it is what most lending protocols do.

### Decision
**Aegis collateral sits idle in a vault. Only the loan asset supplied by lenders is lent.**

### Consequences

**Positive**
- **Custody accounting becomes an exact, assertable identity**:
  `collateral_vault.amount == Σ position.collateral_amount + collateral_fee_accrued`.
  Under re-hypothecation this identity simply does not exist, and INV-CUS-02 could not be written.
- Removes an entire failure class: withdrawal liquidity crunches, where a solvent lender cannot exit
  because their assets were lent out.
- Liquidation is deterministic — seized collateral is always physically present.
- Collateral is per-position rather than pooled, which is what allows `deposit_collateral` and
  `withdraw_collateral` to avoid writing `Market` (claim PERF-C2).

**Negative**
- **Lower capital efficiency**: collateral earns no yield, so Aegis pays less than Aave for the same
  deposit. Stated plainly in `product.md` rather than framed as an oversight. It is the correct trade
  for a protocol whose thesis is bounded risk.
- Users wanting yield on collateral must use a yield-bearing token as collateral, which is a
  market-creation decision, not a protocol feature.

---

## Part B — Explicit PDA token accounts, not ATAs

### Context
Vaults could be Associated Token Accounts of `(Market PDA, mint)` or explicit program-derived token
accounts with our own seeds.

### Decision
Explicit PDA token accounts: `[b"cvault", market]` and `[b"lvault", market]`, **authority = the
`Market` PDA**, with the vault pubkeys **also stored in `Market`** and checked via `has_one`.

### Alternatives considered
**ATAs.** Rejected:
- Adds a dependency on the Associated Token Account program for no benefit — both are equally
  derivable from the market.
- ATA derivation includes the token program in its seeds, so addresses shift between SPL Token and
  Token-2022; our seeds do not, keeping derivation uniform across market types.
- ATAs can be pre-created by anyone, in a state we did not choose. Our seeds are program-specific and
  the account is initialized by us with exact parameters.

**A separate `market_authority` PDA.** Rejected: an extra derivation and an extra account for no
security benefit. The `Market` PDA is the single signer.

### Consequences

**Positive**
- **Exactly one signer PDA in the entire protocol**, signing only for its own two vaults. This makes
  INV-CUS-01 a statement about one reviewable code path.
- **Double validation**: canonical PDA derivation *and* stored-pubkey `has_one`. Either alone would
  suffice; both are present so a single missing constraint is not fatal.
- The complete custody surface is six enumerated token-movement paths (`account-model.md` §6.3). Any
  path not on that list is a bug, which makes the audit tractable.

**Negative**
- Clients must derive vault addresses from Aegis's seeds rather than using a standard ATA helper.
  Handled by the SDK.
- Vault length must be computed via `ExtensionType::try_calculate_account_len` for Token-2022, never
  hardcoded to 165.

### Related requirement
Every vault inflow is credited by **measured delta** after a mandatory post-CPI `reload()`, never by
the requested amount — required for Token-2022 transfer-fee mints and applied uniformly so there is
one code path, not two.
