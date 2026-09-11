# Aegis — SPL Token / Token-2022 Compatibility Policy

**Status: FROZEN (Phase 0). RV-5 closed in Phase 7 (2026-09-11) — see §0.**

> Token-2022 is not a feature to check off. Each extension changes an assumption that a custody
> protocol depends on. Aegis's position is that **an arbitrary Token-2022 mint is not safe collateral**,
> and the protocol must prove a mint is acceptable before it will ever hold it.

---

## 0. RV-5 resolution (Phase 7, 2026-09-11)

**Question:** the complete current Token-2022 mint/account extension list for the exact
`spl-token-2022-interface` version resolved in this workspace, including any extension shipped
after the pre-2024 lists this document's early drafting anticipated (e.g. `Pausable`,
`ScaledUiAmount`), with discriminants and a Tier A/B/C classification for each.

**Sources, in the order actually used:**
1. This workspace's own `Cargo.lock`, inspected directly: `grep -A3 'name = "spl-token-2022' Cargo.lock`.
2. The resolved crate's own source, vendored locally by Cargo at
   `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/spl-token-2022-interface-2.1.0/src/extension/mod.rs`
   — the `pub enum ExtensionType { ... }` definition (line 1056) is the single authoritative list;
   its directory listing (`src/extension/`) was also inspected directly to confirm which
   extensions have their own submodule (i.e. are constructible via a real instruction, not merely
   a filler discriminant).
3. crates.io registry metadata, cross-checked for the same version string (`2.1.0`).

**Resolved dependency:** `spl-token-2022-interface = "2.1.0"` (workspace `Cargo.lock`, resolved
transitively through `anchor-spl`'s `token_interface` re-export). This is the interface-only crate
(instruction builders, state layouts, TLV parsing) — LiteSVM embeds the real, matching Token-2022
program bytecode for on-chain execution, so every test in this repository (Phase 2 onward) runs
against actual program behavior, not a reimplementation.

**Finding:** the exact resolved version's `ExtensionType` enum has 27 real variants (excluding
`Uninitialized`, a padding discriminant with no data of its own, and three `#[cfg(test)]`-gated
variants internal to the crate's own test suite that cannot appear in any account this program
will ever see). All 27 are enumerated in the table below, each mapped to Mint or Account
applicability, Tier, and reasoning. **`Pausable` and `ScaledUiAmount` — this document's own
examples of "extensions added after older common lists" — are both present in this exact resolved
version**, confirming the concern RV-5 was opened to check was real and is now closed, not
assumed away.

**Result: no contradiction with the frozen policy model.** Every Tier A/B extension already
enumerated below (§2) before this phase remains correctly classified; two extensions absent from
the pre-Phase-7 table (`ConfidentialMintBurn`, and the account-level companions of extensions
already covered by name) are added below for completeness. `Pausable` was already anticipated and
already correctly classified as Tier C before this phase began (§2's rationale predates Phase 7);
Phase 7 confirms that classification against the real crate rather than a remembered list. **No
existing Tier classification changed, and Aegis's supported surface did not broaden** — RV-5's
resolution is a verification result, not a policy change.

### 0.1 Complete extension inventory (Mint-applicable)

`programs/aegis/src/token/policy.rs::evaluate_mint` parses a **Mint** account's TLV extension
list — only extensions applicable to a Mint can ever appear there, which is exactly the set below.
The positive-allowlist `match` explicitly names every Tier A/B entry; every Tier C entry (and
anything not in this table at all) falls through the same catch-all rejection arm, proven generic
by `A-TOK-05`'s genuinely-unrecognized discriminant.

| # | Extension (Mint) | Tier | Changes transfer semantics? | External CPI/hook? | Changes balance/accounting interpretation? | Changes authority semantics? | Changes account sizing? | Safe as collateral? | Safe as loan asset? | Reasoning |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `TransferFeeConfig` | **B** | Yes (fee withheld at destination) | No | No (raw base units; delta-measured) | No | Yes (account needs `TransferFeeAmount`) | **Yes** | **No** | §4 |
| 2 | `MintCloseAuthority` | **C** | No | No | No | **Yes** (mint can be closed+reinit) | No | No | No | Invalidates every check made at creation |
| 3 | `ConfidentialTransferMint` | **C** | Yes (encrypted amounts) | No | **Yes** (balances encrypted) | No | Yes | No | No | Breaks INV-CUS-01/02 |
| 4 | `DefaultAccountState` | **C** | No (gates account creation, not transfer) | No | No | No | No | No | No | New vault/liquidator accounts could start frozen; Aegis rejects the extension's presence outright rather than only its `Frozen` value — simpler and at least as safe, since the value is mutable post-creation via the mint's freeze authority |
| 5 | `NonTransferable` | **C** | Yes (blocks all transfers) | No | No | No | No | No | No | Cannot enter or leave a vault at all |
| 6 | `InterestBearingConfig` | **A** | No (UI-scaling only) | No | No (raw base units unaffected) | No | No | **Yes** | **Yes** | §2; SDK must never feed a UI amount back on-chain |
| 7 | `PermanentDelegate` | **C** | Yes (delegate moves any amount, any account) | No | No | **Yes** | No | No | No | Catastrophic, unmitigable |
| 8 | `TransferHook` | **C** | Yes (arbitrary CPI on every transfer) | **Yes** | No | No | No | No | No | Unbounded CU, DoS on liquidation, `ExtraAccountMetaList` surface |
| 9 | `ConfidentialTransferFeeConfig` | **C** | Yes | No | **Yes** | No | Yes | No | No | Confidential-transfer family; §7's "Explicitly deferred" |
| 10 | `MetadataPointer` | **A** | No | No | No | No | No | **Yes** | **Yes** | Display-only |
| 11 | `TokenMetadata` | **A** | No | No | No | No | Yes (variable-length) | **Yes** | **Yes** | Display-only |
| 12 | `GroupPointer` | **A** | No | No | No | No | No | **Yes** | **Yes** | Organizational metadata only |
| 13 | `TokenGroup` | **A** | No | No | No | No | No | **Yes** | **Yes** | Organizational metadata only |
| 14 | `GroupMemberPointer` | **A** | No | No | No | No | No | **Yes** | **Yes** | Organizational metadata only |
| 15 | `TokenGroupMember` | **A** | No | No | No | No | No | **Yes** | **Yes** | Organizational metadata only |
| 16 | `ConfidentialMintBurn` | **C** | No (mint/burn, not transfer) | No | **Yes** (confidential supply) | No | No | No | No | Confidential-transfer family (missing from the pre-Phase-7 table by name; covered by the same wildcard rejection as every other Tier C row; added here for completeness per RV-5 item 19) |
| 17 | `ScaledUiAmount` | **A** | No (UI-scaling only, same shape as `InterestBearingConfig`) | No | No (raw base units unaffected — verified, not assumed: §0.2) | No | No | **Yes** | **Yes** | RV-5 resolved this: present in the exact resolved crate, UI-only by construction (`UiAmountToAmount`/`AmountToUiAmount` are the only instructions that touch the scale; ordinary `TransferChecked` moves raw `u64` base units untouched) |
| 18 | `Pausable` | **C** | Yes (mint authority halts all transfers) | No | No | No | No | No | No | RV-5's own flagged example: added after older extension lists; mint authority can block liquidation exactly when needed |

### 0.2 Account-applicable extensions (never reach `evaluate_mint`; documented for completeness)

These extension types are only ever present in an **Account**'s TLV data, never a **Mint**'s, so
`StateWithExtensions::<Mint>::unpack(...).get_extension_types()` — the call `evaluate_mint` makes —
structurally cannot return any of them; the code path proves this by construction, not merely by
convention (`spl-token-2022-interface` 2.1.0 only ever writes these in response to an
account-level init instruction, on an Account-shaped buffer). Listed for RV-5 completeness:

| Extension (Account) | Tier / handling | Reasoning |
|---|---|---|
| `TransferFeeAmount` | Consequence of `TransferFeeConfig` | Withheld fees; blocks closure, which Aegis never does (§2) |
| `ConfidentialTransferAccount` | N/A — Aegis never opts a vault in | Confidential-transfer family |
| `ImmutableOwner` | **Aegis sets this itself** on every Token-2022 vault | Strengthens custody; §5.4, verified in Phase 7 (§9 below) |
| `MemoTransfer` | Never set on vaults | Would break inbound deposits |
| `CpiGuard` | Not set on vaults; irrelevant to a user's own ATA | §2 |
| `NonTransferableAccount` | Consequence of a `NonTransferable` mint | The mint itself is already rejected (row 5 above) |
| `TransferHookAccount` | Consequence of a `TransferHook` mint | The mint itself is already rejected (row 8 above) |
| `ConfidentialTransferFeeAmount` | N/A | Confidential-transfer family |
| `PausableAccount` | Consequence of a `Pausable` mint | The mint itself is already rejected (row 18 above) |

### 0.3 Verified-not-assumed: `ScaledUiAmount` is raw-unit-safe on-chain

RV-5 specifically flagged `ScaledUiAmount` as "pending" before this phase because a UI-scaling
extension is exactly the kind of thing a careless implementation could misinterpret as changing
the *economic* balance rather than only its *display*. Verified directly against the resolved
crate: the extension's own instruction set (`extension::scaled_ui_amount::instruction`) is limited
to `Initialize`/`UpdateMultiplier`/`AmountToUiAmount`/`UiAmountToAmount` — none of these is in
Aegis's call graph anywhere (`transfer_checked`, the only transfer primitive Aegis ever CPIs into,
takes and returns raw `u64` base units regardless of any multiplier extension present on the
mint). §21's requirement ("Aegis accounting must remain in raw integer token base units... never
accidentally consume scaled UI values on-chain") is therefore satisfied structurally, the same way
`InterestBearingConfig` already was — confirmed rather than merely asserted by grep: no call site
in `programs/aegis/src` references `amount_to_ui_amount`, `ui_amount_to_amount`, or any
`scaled_ui_amount` module path.

## 1. Policy summary

Support is decided **at `create_market`**, per mint, per role. A mint that fails the check can never
enter the protocol, because there is no other way to create a vault for it.

| Tier | Meaning | Enforcement |
|---|---|---|
| **A — Fully supported** | Behaves identically to classic SPL Token for our purposes | Accepted as collateral or loan asset |
| **B — Supported with constraints** | Alters transfer amounts, but handled correctly by measured-delta accounting | Accepted as **collateral only** |
| **C — Unsupported** | Breaks a custody, solvency, or liveness assumption | `create_market` **rejects** |

Two mints, two roles, two independent checks. The loan asset is held to a stricter standard than
collateral because loan-asset accounting drives share pricing for *every* lender in the market, while
collateral accounting is per-position.

---

## 2. Extension classification

| Extension | Tier | Reasoning | Aegis handling |
|---|---|---|---|
| *(none — classic SPL Token)* | **A** | Baseline | Fully supported both roles |
| `MetadataPointer`, `TokenMetadata` | **A** | Display-only; no transfer semantics | Accepted |
| `GroupPointer`, `GroupMemberPointer`, `TokenGroup*` | **A** | Organizational metadata only | Accepted |
| `ImmutableOwner` (account-level) | **A** | *Strengthens* custody — vault ownership cannot be reassigned | Accepted; Aegis **sets it on its own vaults** where supported |
| `InterestBearingConfig` | **A** | Scales only the **UI** amount; raw base units are unchanged. Aegis accounts exclusively in raw base units, so accounting is unaffected | Accepted; the SDK must render UI amounts correctly and must never feed a UI amount back on-chain |
| `ScaledUiAmount` | **A** | Same rationale as interest-bearing: UI-only. RV-5 resolved (§0.3): verified against the resolved crate, not assumed | Accepted |
| `CpiGuard` (account-level) | **A** for our vaults | Prevents certain CPI-driven actions on an account. Irrelevant to vaults we own and sign for | Not set on vaults; a *user's* ATA having it is their concern and does not affect us (we never act as their delegate) |
| `MemoTransfer` (account-level) | **A**, but not set | Requires a memo on incoming transfers; if set on our vault it would break deposits | Never set on vaults. A user's own account having it does not affect inbound transfers to us |
| `TransferFeeConfig` | **B** | Fee is deducted from the **recipient's** amount, so `sent ≠ received`. Correct only with measured-delta accounting | **Collateral only.** Rejected as loan asset (§4) |
| `TransferHook` | **C** | Arbitrary program invoked on every transfer: unbounded CU, arbitrary failure (DoS on liquidation → insolvency), extra accounts via `ExtraAccountMetaList`, and a control-flow surface the Solana account model otherwise avoids. Hook program and authority can change post-creation | **Rejected** |
| `PermanentDelegate` | **C** | A delegate can transfer **any** amount from **any** account of that mint — including our vault — without our consent. Catastrophic and unmitigable | **Rejected** |
| `MintCloseAuthority` | **C** | The mint can be closed and **reinitialized at the same address with different extensions**, invalidating every check performed at market creation | **Rejected** |
| `DefaultAccountState = Frozen` | **C** | Newly created accounts start frozen; our vault could be unusable, and liquidators' destination accounts could fail, blocking liquidation | **Rejected** |
| `Pausable` | **C** | The mint authority can halt all transfers. Liquidation becomes impossible exactly when it is needed most, converting a market risk into guaranteed bad debt | **Rejected** |
| `NonTransferable` | **C** | Cannot be moved into or out of a vault | **Rejected** |
| `ConfidentialTransfer*` | **C** | Balances are encrypted; `vault.amount` no longer reconciles with internal accounting, destroying INV-CUS-01/02 | **Rejected** |
| `TransferFeeAmount` (account-level, withheld) | — | Consequence of `TransferFeeConfig`; withheld fees block account closure | Handled: Aegis never closes vaults |
| Unknown / future extension | **C** | Fail closed on anything not explicitly allowlisted | **Rejected** |

**The allowlist is positive, not negative.** `create_market` enumerates the mint's TLV extension list
and rejects any type not in the Tier A/B set. A new extension shipped by Token-2022 tomorrow is
rejected by default until Aegis explicitly evaluates it. A blocklist would silently accept it — this
distinction is the whole point and is asserted by `A-TOK-05` (a mint carrying an unrecognized
extension discriminant must be rejected).

---

## 3. `freeze_authority` — a separate axis

`freeze_authority` is a base-mint field, present in **both** token programs, independent of extensions.
A mint with a freeze authority can freeze our vault or a liquidator's account, blocking seizure and
guaranteeing bad debt.

Rejecting it outright would exclude essentially every real stablecoin (USDC has one). Pretending it is
harmless would be dishonest. Aegis therefore makes it an **explicit, recorded, acknowledged risk**:

- `create_market` requires `ack_freeze_authority == true` if either mint has a freeze authority.
- The acknowledgement is persisted in `market.flags` bit 0 and emitted in `MarketCreated`.
- The SDK and UI surface it as a named market risk.

This is the honest engineering answer: the risk is real, unavoidable for useful assets, and therefore
must be *visible* rather than either hidden or used as an excuse to reject the asset.

---

## 4. Why transfer-fee mints are collateral-only

With measured-delta accounting, a fee-bearing collateral asset is handled correctly:

- **Deposit:** credit `received = vault_after − vault_before`. The user's position reflects what the
  vault actually holds. INV-CUS-02 holds exactly.
- **Withdraw / seize:** debit exactly the recorded amount from the vault; the recipient absorbs the
  fee. INV-CUS-02 still holds exactly.

As a **loan** asset it is rejected, for three reasons:

1. **Borrowers receive less than they owe.** Borrow 1000, receive 995, owe 1000. Defensible but
   user-hostile, and it makes the borrow-then-immediately-repay round-trip property (`P-SHARE-4`)
   lossy in a way that complicates every solvency proof.
2. **Liquidation repayment shrinks in transit.** The liquidator sends `repay_assets`; the vault
   receives less. Crediting the received amount means the seizure was computed against a larger
   repayment than actually arrived — the liquidator gets collateral for value the pool never received.
   Correcting this requires computing the fee *before* the transfer and inverting it, which
   `TransferFeeConfig` makes possible but which introduces a second source of truth about the fee rate
   that can change between quote and execution.
3. **Share pricing is shared state.** A collateral accounting error harms one position; a loan-asset
   accounting error mis-prices shares for every lender in the market.

The asymmetry is deliberate and is the concrete answer to "does Aegis actually understand Token-2022,
or is it a checkbox?" — the same extension is safe in one role and unsafe in another, and Aegis draws
the line at the correct place.

---

## 5. Implementation requirements

### 5.1 Interfaces
Use `anchor_spl::token_interface` (`InterfaceAccount<'info, TokenAccount>`,
`InterfaceAccount<'info, Mint>`, `Interface<'info, TokenInterface>`) so both token programs are
accepted, **and** pin the concrete program per market:

```
require_keys_eq!(collateral_token_program.key(), market.collateral_token_program);
require_keys_eq!(loan_token_program.key(),       market.loan_token_program);
```

The interface types alone are **not** sufficient — they accept either program. Without the explicit
equality check, an attacker could present a same-address account under the wrong program. (T-11.)

### 5.2 Always `transfer_checked`
Never `transfer`. `transfer_checked` validates the mint and decimals at the token-program level,
which closes an entire class of mint-substitution bugs and is mandatory for Token-2022 anyway.

### 5.3 Measured-delta accounting (mandatory, both token programs)
```
let before = vault.amount;
transfer_checked(...)?;
vault.reload()?;                       // MANDATORY — pre-CPI data is stale
let credited = vault.amount.checked_sub(before).ok_or(VaultAccountingError)?;
```
The `reload()` is the load-bearing line. Reading the pre-CPI deserialized `amount` after a CPI is the
classic stale-account bug (T-15) and would silently credit the wrong amount for fee-bearing mints.
Applied uniformly to *both* token programs so there is one code path, not two.

### 5.4 Vault creation sizing
Vault length must be computed from the extensions the vault itself will carry
(`ExtensionType::try_calculate_account_len`), never hardcoded to 165. Aegis sets `ImmutableOwner` on
Token-2022 vaults where supported.

### 5.5 Decimals
Cached in `Market` at creation. Mint decimals are immutable in both programs, so the cache cannot go
stale — this is verified once at creation and asserted in `U-TOK-03`.

---

## 6. Verification procedure at `create_market`

```
1. Assert mint account owner == the passed token program.
2. If SPL Token (legacy): no extensions possible → Tier A. Go to 5.
3. If Token-2022: parse the TLV extension list.
4. For each extension type found:
     - not in the allowlist                                  → reject (UnsupportedTokenExtension)
     - TransferFeeConfig and this mint is the loan asset      → reject (TransferFeeNotAllowedForLoanAsset)
     - TransferFeeConfig and this mint is collateral          → set flags.collateral_has_transfer_fee
5. If mint.freeze_authority.is_some() && !ack_freeze_authority → reject (FreezeAuthorityNotAcknowledged)
6. Cache decimals; create the vault with correct length and Market as authority.
7. Emit MarketCreated including the full extension inventory that was accepted.
```

Step 7 matters: the emitted event is the permanent audit record of exactly which extension set was
approved for that market, which is what makes the policy reviewable after the fact.

---

## 7. Tests (Phase 7 acceptance)

| ID | Test |
|---|---|
| `U-TOK-01` | Deposit/withdraw of a classic SPL Token collateral: `credited == amount` |
| `U-TOK-02` | Deposit of a transfer-fee Token-2022 collateral: `credited == amount − fee`; INV-CUS-02 holds exactly |
| `U-TOK-03` | Cached decimals equal mint decimals for every supported mint |
| `A-TOK-01` | `create_market` rejects a `TransferHook` mint (both roles) |
| `A-TOK-02` | `create_market` rejects a `PermanentDelegate` mint |
| `A-TOK-03` | `create_market` rejects a `MintCloseAuthority` mint |
| `A-TOK-04` | `create_market` rejects a `DefaultAccountState = Frozen` mint |
| `A-TOK-05` | `create_market` rejects a mint carrying an **unrecognized** extension discriminant (positive allowlist) |
| `A-TOK-06` | `create_market` rejects a transfer-fee mint as the **loan** asset, accepts it as **collateral** |
| `A-TOK-07` | `create_market` rejects a freeze-authority mint when `ack_freeze_authority == false`, accepts it when `true`, and records the flag |
| `A-TOK-08` | Passing the wrong token program for a market's mint fails, even though both are valid token programs |
| `A-TOK-09` | Passing a Token-2022 mint's account with the legacy SPL Token program (and vice versa) fails |
| `A-TOK-10` | Full lifecycle (supply → borrow → liquidate → bad debt) on a transfer-fee collateral market; INV-CUS-01/02 asserted after **every** instruction |
| `A-TOK-11` | A transfer-fee rate raised by the fee authority mid-lifecycle does not break accounting (delta accounting absorbs it) |

`A-TOK-11` is the one most likely to be skipped and most likely to catch a real bug: a fee rate that
changes between deposit and withdrawal is exactly the scenario where hardcoded fee assumptions fail.

### 7.1 Evidence — Phase 7 (`tests/phase7_token2022.rs` unless noted)

| ID | Test function | Result |
|---|---|---|
| `U-TOK-01` | `tests/phase3_collateral.rs::spl_deposit_credits_exact_amount` (Phase 3) | ✅ |
| `U-TOK-02` | `tests/phase3_collateral.rs::token2022_transfer_fee_deposit_credits_net_of_fee` (Phase 3) | ✅ |
| `U-TOK-03` | `u_tok_03_cached_decimals_match_mint_for_every_supported_configuration` | ✅ |
| `A-TOK-01` | `tests/phase2_token_policy.rs::transfer_hook_mint_rejected_as_collateral` (Phase 2) | ✅ |
| `A-TOK-02` | `tests/phase2_token_policy.rs::permanent_delegate_mint_rejected` (Phase 2) | ✅ |
| `A-TOK-03` | `tests/phase2_token_policy.rs::mint_close_authority_mint_rejected` (Phase 2) | ✅ |
| `A-TOK-04` | `tests/phase2_token_policy.rs::default_account_state_frozen_mint_rejected` (Phase 2) | ✅ |
| `A-TOK-05` | `tests/phase2_token_policy.rs::unrecognized_extension_mint_rejected` (Phase 2) | ✅ |
| `A-TOK-06` | `tests/phase2_token_policy.rs::transfer_fee_mint_accepted_as_collateral_rejected_as_loan_asset` (Phase 2) | ✅ |
| `A-TOK-07` | `tests/phase2_token_policy.rs::freeze_authority_requires_acknowledgement` (Phase 2) | ✅ |
| `A-TOK-08` | `tests/phase3_adversarial.rs::wrong_token_program_for_spl_market_is_rejected` (Phase 3) | ✅ |
| `A-TOK-09` | `tests/phase3_adversarial.rs::wrong_token_program_for_token2022_market_is_rejected` (Phase 3) | ✅ |
| `A-TOK-10` | `a_tok_10_full_lifecycle_on_transfer_fee_collateral_market` | ✅ |
| `A-TOK-11` | `a_tok_11_fee_rate_change_mid_lifecycle_does_not_break_accounting` | ✅ |

Supplementary RV-5 evidence, beyond the frozen `A-TOK-01..11`/`U-TOK-01..03` list — added because
RV-5's own text specifically calls out extensions absent from older lists, and item 10 of the
phase specification requires proof, not assertion, that a reassigned vault owner is impossible:

| Test function | Proves |
|---|---|
| `pausable_mint_rejected_as_collateral` | `Pausable` (RV-5's flagged example) is rejected, using a real `InitializePausableConfig`-carrying mint, not a synthetic byte pattern |
| `non_transferable_mint_rejected` | `NonTransferable` is rejected |
| `immutable_owner_blocks_reassignment_even_by_the_genuine_current_owner` | `ImmutableOwner` blocks `SetAuthority(AccountOwner)` even when the real, correctly-signing current owner attempts it |

Every remaining Tier C row in §0.1 not listed above (`ConfidentialTransferMint`,
`ConfidentialTransferFeeConfig`, `ConfidentialMintBurn`) is covered by the **structural** argument
`A-TOK-05` already establishes: `evaluate_mint`'s `match` names every Tier A/B arm explicitly and
rejects via one shared catch-all arm for everything else, so no extension-specific carve-out exists
that could accidentally admit any of them — the same code path five different, unrelated Tier C
extensions (`TransferHook`, `PermanentDelegate`, `MintCloseAuthority`, `DefaultAccountState`, an
arbitrary unrecognized discriminant, and now `Pausable`/`NonTransferable`) are already proven to hit.
Constructing a real confidential-transfer mint requires ElGamal/Pedersen key material with no
proportionate additional assurance over this structural argument, so no dedicated fixture was built
for that family; this is recorded here explicitly rather than silently claimed as tested.

---

## 8. Explicitly deferred

| Item | Status | Reason |
|---|---|---|
| Transfer-hook support with an allowlisted hook program | **Not in v1** | Would require CU budgeting for arbitrary hook execution and an `ExtraAccountMetaList` resolution path in liquidation. A defensible v2 with a *hook allowlist*, never open-ended |
| Confidential transfers | **Never** (for vault assets) | Irreconcilable with INV-CUS-01/02 |
| Transfer-fee loan assets | **Not in v1** | §4 |
| Tokenized supply shares (an Aegis-issued mint) | **Not in v1** | ADR-0006; would add mint-authority custody surface for composability v1 does not need |
| Mints with a permanent delegate under a trusted-issuer allowlist | **Not in v1** | Neodyme's mitigation ("only support mints from trusted creators") is a governance answer, not a technical one, and v1 has no governance to make that judgement |
