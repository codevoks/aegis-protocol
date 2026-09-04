# Aegis — SPL Token / Token-2022 Compatibility Policy

**Status: FROZEN (Phase 0). Research gate RV-5 must be closed in Phase 7.**

> Token-2022 is not a feature to check off. Each extension changes an assumption that a custody
> protocol depends on. Aegis's position is that **an arbitrary Token-2022 mint is not safe collateral**,
> and the protocol must prove a mint is acceptable before it will ever hold it.

---

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
| `ScaledUiAmount` | **A** (pending RV-5) | Same rationale as interest-bearing: UI-only | Accepted after Phase 7 verifies the semantics |
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

---

## 8. Explicitly deferred

| Item | Status | Reason |
|---|---|---|
| Transfer-hook support with an allowlisted hook program | **Not in v1** | Would require CU budgeting for arbitrary hook execution and an `ExtraAccountMetaList` resolution path in liquidation. A defensible v2 with a *hook allowlist*, never open-ended |
| Confidential transfers | **Never** (for vault assets) | Irreconcilable with INV-CUS-01/02 |
| Transfer-fee loan assets | **Not in v1** | §4 |
| Tokenized supply shares (an Aegis-issued mint) | **Not in v1** | ADR-0006; would add mint-authority custody surface for composability v1 does not need |
| Mints with a permanent delegate under a trusted-issuer allowlist | **Not in v1** | Neodyme's mitigation ("only support mints from trusted creators") is a governance answer, not a technical one, and v1 has no governance to make that judgement |
