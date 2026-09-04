# Aegis — Account, PDA and Custody Model

**Status: FROZEN (Phase 0). Changing seeds, authorities, or field semantics requires an ADR.**

---

## 1. Design principles

1. **Minimize shared writable state.** A single global writable account would serialize the entire
   protocol. Aegis has exactly one global account and it is **never written by a user instruction**.
2. **Isolation by construction.** Two different markets share no writable account. This is what makes
   the Sealevel claim testable rather than rhetorical.
3. **Every account is derivable.** No counters, no registries, no "next index". Market and position
   addresses are pure functions of their inputs, so any client can derive them offline.
4. **Authority is unambiguous.** Exactly one PDA signs for protocol-held tokens, and it is the market
   itself.
5. **Defense in depth on identity.** Every relationship is checked *twice* — once by PDA seed
   derivation and once by a stored-pubkey `has_one` — so a single missing constraint is not fatal.

---

## 2. Account inventory

Four program-owned data accounts and two token accounts. That is the whole protocol.

| # | Account | Owner | Count | Hot-writable? |
|---|---|---|---|---|
| A1 | `Protocol` | Aegis | 1 (singleton) | **No** — admin-only writes |
| A2 | `Market` | Aegis | 1 per market | Yes (supply/withdraw/borrow/repay/liquidate) |
| A3 | `Position` | Aegis | 1 per (market, owner) | Yes, but per-user |
| A4 | `collateral_vault` | Token program | 1 per market | Yes |
| A5 | `loan_vault` | Token program | 1 per market | Yes |

Rejected candidate accounts, and why:

| Rejected | Reason |
|---|---|
| `OracleConfig` (separate) | Folded into `Market`. A shared oracle config would couple markets — a bad config update would contaminate every market referencing it, directly contradicting the isolation thesis. Folding also removes one account from every priced instruction. See ADR-0008. |
| `InterestState` / `Reserve` | Folded into `Market`. The IRM is stateless (ADR-0007), so there is nothing to store separately. |
| `ProtocolFeeVault` | Unnecessary. Loan-side fees accrue as ordinary supply shares to the fee recipient's `Position`; collateral-side fees accrue as a `u64` on `Market` and stay physically in the collateral vault. Zero extra accounts, zero extra contention. |
| `MarketRegistry` / `market_count` | A counter is a hot-writable global. Markets are content-addressed by seeds instead. |
| `UserRegistry` | Positions are derivable from `(market, owner)`. |
| Separate `market_authority` PDA | The `Market` PDA is itself the vault authority. A second PDA would add a derivation and an account for no security benefit. |
| Mock-oracle program | Rejected in favor of test-fixture account injection. See ADR-0008. |

---

## 3. `Protocol` (A1)

**Purpose:** singleton configuration and the root of administrative authority.

- **Seeds:** `[b"protocol"]`, canonical bump stored in the account.
- **Owner:** Aegis program.
- **Authority:** `admin`.
- **Lifecycle:** created once by `initialize_protocol`; never closed.
- **Concurrency:** **read-only in every user instruction.** Only `set_*` admin instructions write it.
  This is the single most important parallelism decision in the protocol — see §8.

```rust
pub struct Protocol {
    pub admin: Pubkey,             // 32  full authority
    pub pending_admin: Pubkey,     // 32  two-step transfer; Pubkey::default() = none
    pub guardian: Pubkey,          // 32  pause-only authority
    pub fee_recipient: Pubkey,     // 32  default fee recipient for new markets
    pub paused: u8,                // 1   global pause bitflags
    pub bump: u8,                  // 1
    pub _reserved: [u8; 64],       // 64  forward compatibility
}
// 8 (discriminator) + 194 = 202 bytes
```

`_reserved` exists so that additive fields do not require a realloc in Phase 12. It is checked to be
all-zero on write, so it cannot be used as a covert channel.

**Pause bitflags** (`paused`):
`0b0001 SUPPLY`, `0b0010 BORROW`, `0b0100 WITHDRAW`, `0b1000 LIQUIDATE`.
`repay`, `deposit_collateral` and `absorb_bad_debt` are **unpausable by design** — see INV-ADM-04.

---

## 4. `Market` (A2)

**Purpose:** one isolated lending venue. Holds risk parameters, the oracle configuration, the interest
model parameters, and the pool accounting scalars.

- **Seeds:**
  `[b"market", collateral_mint.key(), loan_mint.key(), &config_id.to_le_bytes()]`
  where `config_id: u16`.
- **Why this shape:** content-addressed (no counter), and `config_id` allows several risk
  configurations for the same asset pair (e.g. a conservative 70% LLTV market and an aggressive 80%
  one) without any registry. Three seeds of ≤32 bytes plus a literal — well within limits.
- **Owner:** Aegis program.
- **Authority:** `Protocol.admin` for parameters; `Protocol.guardian` for pausing. The account is
  **also the token authority for both vaults**.
- **Lifecycle:** created by `create_market`; never closed (closing a market with outstanding positions
  would be catastrophic, and there is no safe time to close one).
- **Concurrency:** hot-writable for supply/withdraw/borrow/repay/liquidate/accrue. Explicitly
  **read-only** for `deposit_collateral` and `withdraw_collateral`.

```rust
pub struct Market {
    // --- identity (immutable after creation) ---
    pub collateral_mint: Pubkey,        // 32
    pub loan_mint: Pubkey,              // 32
    pub collateral_token_program: Pubkey,// 32  pinned: SPL Token or Token-2022
    pub loan_token_program: Pubkey,     // 32
    pub collateral_vault: Pubkey,       // 32
    pub loan_vault: Pubkey,             // 32
    pub fee_recipient: Pubkey,          // 32  snapshot at creation; market-local
    pub config_id: u16,                 // 2
    pub collateral_decimals: u8,        // 1   cached from mint (immutable in both token programs)
    pub loan_decimals: u8,              // 1

    // --- oracle config (admin-mutable, market-local) ---
    pub oracle_kind: u8,                // 1   0 = Pyth pull
    pub collateral_feed_id: [u8; 32],   // 32
    pub loan_feed_id: [u8; 32],         // 32
    pub max_price_age_secs: u32,        // 4
    pub max_conf_bps: u16,              // 2

    // --- risk params (admin-mutable, bounds-checked) ---
    pub max_ltv: u128,                  // 16  WAD
    pub liq_threshold: u128,            // 16  WAD
    pub liq_bonus: u128,                // 16  WAD
    pub close_factor: u128,             // 16  WAD
    pub full_liq_hf: u128,              // 16  WAD
    pub liq_protocol_fee: u128,         // 16  WAD
    pub fee: u128,                      // 16  WAD (interest fee)
    pub min_debt: u64,                  // 8   loan base units

    // --- IRM params (stateless) ---
    pub base_rate_ps: u128,             // 16  per-second WAD
    pub slope1_ps: u128,                // 16
    pub slope2_ps: u128,                // 16
    pub u_kink: u128,                   // 16  WAD
    pub max_rate_ps: u128,              // 16

    // --- accounting (hot) ---
    pub total_supply_assets: u64,       // 8
    pub total_supply_shares: u128,      // 16
    pub total_borrow_assets: u64,       // 8
    pub total_borrow_shares: u128,      // 16
    pub collateral_fee_accrued: u64,    // 8
    pub last_accrual_ts: i64,           // 8

    // --- flags / bumps ---
    pub paused: u8,                     // 1   same bitflags as Protocol
    pub flags: u8,                      // 1   bit0 = ack_freeze_authority, bit1 = collateral_has_transfer_fee
    pub bump: u8,                       // 1
    pub collateral_vault_bump: u8,      // 1
    pub loan_vault_bump: u8,            // 1
    pub _reserved: [u8; 64],            // 64
}
// 8 + ~633 ≈ 641 bytes. Well under 10 KB; single-allocation, no realloc needed.
```

**Size note:** at Agave 4.2's reduced rent (`lamports_per_byte` 6960 → 696), ~641 bytes costs roughly
a tenth of what it did. This validates keeping fields explicit and readable rather than bit-packing
them — the performance strategy records this as a deliberate non-optimization.

**Why `u128` for WAD parameters rather than `u64`:** every one of these is consumed directly by
`mul_div_*` on `u128`. Storing them as `u64` would force a widening cast at every use site and invite
a truncation bug on the write path. The bytes are cheap; the class of bug is not.

---

## 5. `Position` (A3)

**Purpose:** one user's entire relationship with one market — lending, borrowing, and collateral.

- **Seeds:** `[b"position", market.key(), owner.key()]`
- **Owner:** Aegis program.
- **Authority:** `owner` signs for withdrawals, borrows, and closing. Anyone may *increase* a
  position's safety (repay, deposit collateral) without signing as owner — see §5.1.
- **Lifecycle:** created by `init_position`; closable by `close_position` when fully empty.
- **Concurrency:** writable, but keyed to one user — so different users in the same market contend only
  on `Market`, and not at all for collateral operations.

```rust
pub struct Position {
    pub market: Pubkey,        // 32  has_one target
    pub owner: Pubkey,         // 32
    pub supply_shares: u128,   // 16
    pub borrow_shares: u128,   // 16
    pub collateral_amount: u64,// 8
    pub bump: u8,              // 1
    pub _reserved: [u8; 32],   // 32
}
// 8 + 137 = 145 bytes
```

**One account for all three roles** (lender, borrower, collateral holder) rather than three accounts:
a user acting in several roles touches one account instead of three, the account count per instruction
stays low, and there is exactly one PDA to derive per (market, user).

### 5.1 Signer policy — deliberate asymmetry

| Operation | Requires `owner` signature? | Rationale |
|---|---|---|
| `supply`, `borrow`, `withdraw`, `withdraw_collateral`, `close_position` | **Yes** | Moves value out or creates an obligation. |
| `repay` | **No** | Repaying someone else's debt is a gift, not an attack. Blocking it would break liquidator bots and third-party keepers. |
| `deposit_collateral` | **No** | Strictly risk-reducing for the position. |
| `init_position` | No (payer signs) | Creating an empty position for another user is harmless and improves UX. |
| `liquidate`, `absorb_bad_debt`, `accrue_interest` | Liquidator/caller signs; position owner does not | Permissionless by design. |

This asymmetry is a security *property*, not laxity: the rule is **a signature is required exactly when
an action can reduce the position's safety or extract value**. It is asserted as INV-AUTH-03.

---

## 6. Vaults (A4, A5)

- **`collateral_vault`** — seeds `[b"cvault", market.key()]`, mint `collateral_mint`, owned by
  `collateral_token_program`, **authority = the `Market` PDA**.
- **`loan_vault`** — seeds `[b"lvault", market.key()]`, mint `loan_mint`, owned by
  `loan_token_program`, **authority = the `Market` PDA**.

### 6.1 Why explicit PDA token accounts and not ATAs (ADR-0005)

| Consideration | Decision |
|---|---|
| Derivability | Both are trivially derivable from the market; the ATA program adds nothing. |
| Dependency surface | Explicit PDAs remove a dependency on the Associated Token Account program entirely. |
| Token-2022 | ATA derivation includes the token program in its seeds, so ATA addresses shift between token programs; our seeds do not, which keeps derivation uniform. |
| Pre-creation griefing | An attacker can pre-create an ATA for `(market, mint)` with an unexpected state. Our seeds are program-specific and `init` is performed by us with exact parameters. |
| Defense in depth | The vault pubkey is *also* stored in `Market` and checked with `has_one`, so both the seed derivation and the stored address must agree. |

### 6.2 Signing

All vault outflows use `invoke_signed` with the market's seeds:

```
[b"market", collateral_mint, loan_mint, &config_id.to_le_bytes(), &[market.bump]]
```

There is exactly **one** signer PDA in the protocol and it is the market. This makes INV-CUS-01 — *only
the market PDA may move tokens out of its own vaults* — a statement about a single code path that can
be reviewed exhaustively.

### 6.3 Token movement paths (complete enumeration)

| Direction | Instruction | Source authority |
|---|---|---|
| user → loan_vault | `supply`, `repay`, `liquidate` (repayment) | user signature |
| loan_vault → user | `withdraw`, `borrow` | market PDA |
| user → collateral_vault | `deposit_collateral` | user signature |
| collateral_vault → user | `withdraw_collateral` | market PDA |
| collateral_vault → liquidator | `liquidate` (seizure) | market PDA |
| collateral_vault → admin | `withdraw_collateral_fees` | market PDA |

**Six paths. That is the complete custody surface.** Any code path moving tokens that is not on this
list is a bug. This table is duplicated in the security review checklist for Phase 13.

### 6.4 Measured-delta accounting (mandatory)

Because a Token-2022 mint may charge a transfer fee, **Aegis never assumes `amount_transferred ==
amount_received`.** Every inbound transfer to a vault is accounted as:

```
before = vault.amount        // reload
transfer_checked(...)
after  = vault.amount        // RELOAD the account after CPI
credited = after − before    // this is what the protocol records
```

The account **must be reloaded** after the CPI; relying on the pre-CPI deserialized value is the
classic stale-account-after-CPI bug (T-15). Outbound transfers debit exactly the recorded amount, and
the recipient bears any fee — which keeps the vault reconciliation exact in both directions.

For SPL Token (no fees) this reduces to `credited == amount`, so it costs one extra account read and
buys correctness under Token-2022. A test asserts both paths (`U-TOK-01`, `U-TOK-02`).

---

## 7. PDA and bump handling rules

1. **Canonical bumps only.** Every PDA is derived with `find_program_address` at creation, the
   canonical bump is stored in the account, and later derivations use the stored bump via
   `seeds = [...], bump = account.bump`. Non-canonical bumps are never accepted.
2. **No seed sharing across account types.** Every seed set begins with a distinct literal prefix
   (`protocol`, `market`, `position`, `cvault`, `lvault`). No two account types can ever collide.
3. **No user-controlled bytes in a seed position that could alias another type.** All variable seeds
   are 32-byte pubkeys or a fixed-width `u16`, so no length-ambiguity concatenation attack exists.
4. **Discriminators.** Anchor's account discriminator is checked on every deserialization; combined
   with owner checks this makes type confusion impossible.

---

## 8. Sealevel / parallelism analysis

Solana locks accounts per transaction: writable accounts are exclusive, read-only accounts are shared.
Aegis's write sets:

| Instruction | Writable accounts |
|---|---|
| `deposit_collateral` | `Position`, `collateral_vault`, user token account |
| `withdraw_collateral` | `Position`, `collateral_vault`, user token account |
| `supply` / `withdraw` | `Market`, `Position`, `loan_vault`, user token account |
| `borrow` / `repay` | `Market`, `Position`, `loan_vault`, user token account |
| `liquidate` | `Market`, `Position`, both vaults, two liquidator token accounts |
| `absorb_bad_debt` | `Market`, `Position`, `fee_position` |
| `accrue_interest` | `Market` |
| admin `set_*` | `Protocol` or `Market` |

Consequences, stated as claims to be **measured** in Phase 11 rather than asserted:

- **C1 — Markets are fully independent.** No writable account is shared between two markets, so
  transactions in different markets never conflict. `Protocol` is read-only in every user instruction,
  which is what makes this true; had it held a counter or aggregate, every instruction in the protocol
  would serialize on it.
- **C2 — Collateral operations parallelize within a market.** `deposit_collateral` and
  `withdraw_collateral` do not write `Market`. Different users can deposit or withdraw collateral in
  the *same* market concurrently, contending only on the shared `collateral_vault`.
- **C3 — `Market` is the residual contention point** for supply/borrow/repay/liquidate. This is
  inherent to pooled lending: the pool's totals are shared state. It is bounded per market, which is
  the best achievable without abandoning the pooled model.
- **C4 — The vaults are also contention points** for their respective instructions, since token
  accounts must be writable to move value. Unavoidable.

C2 depends entirely on `withdraw_collateral` using `accrue_view` rather than `accrue_mut`
(economic-model §4.5). **Any future change that makes `withdraw_collateral` write `Market` silently
destroys C2** — that is recorded as a code comment requirement and a Phase 11 regression check.

---

## 9. Initialization and rent

| Account | Created by | Payer | Space |
|---|---|---|---|
| `Protocol` | `initialize_protocol` | admin | 202 |
| `Market` | `create_market` | admin | ~641 |
| `collateral_vault` | `create_market` | admin | token account (165, or larger with extensions) |
| `loan_vault` | `create_market` | admin | token account |
| fee `Position` | `create_market` | admin | 145 |
| `Position` | `init_position` | any payer | 145 |

`create_market` initializes the fee recipient's `Position` so that `absorb_bad_debt` can always
require it (economic-model §8.2). This removes an entire "what if the fee position doesn't exist"
branch from the most delicate instruction in the protocol.

**Token-2022 vault sizing:** a Token-2022 account's size depends on its extensions
(e.g. `ImmutableOwner`). `create_market` must compute the required length via
`ExtensionType::try_calculate_account_len` rather than hardcoding 165. Hardcoding is a common bug and
is called out in the Phase 2 spec.

**No `init_if_needed` anywhere.** It is the standard reinitialization footgun. `init_position` is an
explicit instruction and the SDK bundles it into the first user transaction (Phase 9).

---

## 10. Close behavior

Only `Position` is closable, via `close_position`:

```
require position.supply_shares == 0
require position.borrow_shares == 0
require position.collateral_amount == 0
require owner is signer
```

Closure uses Anchor's `close = owner` (lamports to owner, discriminator zeroed, account defunded), not
a manual pattern — `CLOSED_ACCOUNT_DISCRIMINATOR` was removed in Anchor 1.0 and hand-rolled closes are
a known revival-attack vector. Because the position PDA is deterministic, a closed position can be
re-initialized later by `init_position`, which is correct and safe: it can only ever be recreated
empty.

`Market` and `Protocol` are never closable. There is no safe moment to close a market — a market with
any outstanding position or vault balance must persist, and one with none costs a trivial amount of
rent.

---

## 11. Account-model invariant summary

| ID | Invariant |
|---|---|
| INV-ACCT-01 | Exactly one `Protocol` account exists, at `PDA([b"protocol"])`. |
| INV-ACCT-02 | Every `Market` is at `PDA([b"market", collateral_mint, loan_mint, config_id])` with the canonical bump. |
| INV-ACCT-03 | Every `Position` is at `PDA([b"position", market, owner])` with the canonical bump, and `position.market == market.key()`. |
| INV-ACCT-04 | `market.collateral_vault` and `market.loan_vault` equal their canonical PDAs and are owned by the pinned token programs with the pinned mints. |
| INV-ACCT-05 | Both vaults' token authority is the `Market` PDA and nothing else. |
| INV-ACCT-06 | `market.collateral_decimals` / `loan_decimals` equal the respective mints' decimals for the market's whole lifetime. |
| INV-ACCT-07 | No instruction other than an admin `set_*` writes `Protocol`. |
| INV-ACCT-08 | `deposit_collateral` and `withdraw_collateral` do not declare `Market` as writable. |
| INV-ACCT-09 | `_reserved` bytes are zero in every persisted account. |
