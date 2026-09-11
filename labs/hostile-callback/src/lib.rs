//! `hostile-callback` — a single, deliberately adversarial liquidation-callback program used ONLY
//! to prove Aegis's Phase 8 defenses (`A-CPI-01..04`, `docs/phases/phase-08-composability.md`,
//! ADR-0013). **This is a test-only lab program.** It is never a real integration and must never
//! be treated as one.
//!
//! Receives the same positional account list any Aegis liquidation callback receives
//! (`programs/aegis/src/instructions/liquidate/liquidate.rs::build_callback_instruction`):
//! `collateral_account`, `loan_vault`, `collateral_mint`, `loan_mint`, `collateral_token_program`,
//! `loan_token_program` — plus, for `DrainVault`, one extra `remaining_accounts` entry (an
//! attacker-controlled destination token account).
//!
//! Every attack here is expected to **fail**. A test asserting anything else about this program
//! has misunderstood its purpose.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token_interface::{self, TransferChecked};

declare_id!("7ryyBoBLtHDhycNeRpZntF4D43TMa76L6AXT2JWpBp5X");

/// The real Aegis program ID (`programs/aegis/src/lib.rs`'s `declare_id!`), hardcoded rather than
/// taken as a dependency: this lab program only needs the bare pubkey to attempt `A-CPI-02`'s
/// reentrant CPI, not any of Aegis's types.
pub fn aegis_program_id() -> Pubkey {
    use core::str::FromStr;
    #[allow(clippy::unwrap_used, reason = "fixed, valid base58 constant")]
    Pubkey::from_str("DbRhjkZV1QSxMj5AvrYdgVsyEz8nKhoCLnSLGSKsqaF9").unwrap()
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum AttackMode {
    /// A-CPI-01: attempt to move Aegis vault funds using this program's own (illegitimate)
    /// authority. `remaining_accounts[0]` must be an attacker-controlled destination token
    /// account. Must fail: `loan_vault`'s real authority is the Market PDA, never this program.
    DrainVault { amount: u64 },
    /// A-CPI-02: attempt to CPI back into Aegis's `liquidate` on the same market. Must fail --
    /// either the runtime's own indirect-reentrancy protection (RV-6,
    /// `docs/ecosystem-research.md` §16.1) or the absence of any real `liquidator`/`market` signer
    /// this program was never given (ADR-0013 §1.4) rejects it first. The direct, protocol-level
    /// proof that the Aegis guard ALSO independently rejects reentrancy lives in
    /// `tests/phase8_hostile_callback.rs` as a unit-level test of the handler, not here.
    Reenter,
    /// A-CPI-03: deliberately exhausts the compute budget. `iterations` is caller-controlled so
    /// the test can size it precisely against whatever compute limit it sets for the outer
    /// transaction.
    BurnCompute { iterations: u64 },
    /// A-CPI-04: returns `Ok(())` immediately without transferring anything into `loan_vault`.
    /// Aegis must reject based on the measured post-callback delta, never this return value.
    NoRepayment,
}

#[error_code]
pub enum HostileCallbackError {
    #[msg("DrainVault requires exactly one remaining account (the attacker's destination)")]
    MissingDrainDestination,
}

#[program]
pub mod hostile_callback {
    use super::*;

    pub fn attack<'info>(ctx: Context<'info, Attack<'info>>, mode: AttackMode) -> Result<()> {
        match mode {
            AttackMode::DrainVault { amount } => drain_vault(&ctx, amount),
            AttackMode::Reenter => reenter(&ctx),
            AttackMode::BurnCompute { iterations } => burn_compute(iterations),
            AttackMode::NoRepayment => Ok(()),
        }
    }
}

/// A-CPI-01: this program was never given `loan_vault`'s authority (the Market PDA) and holds no
/// delegate over it. The only account it can even offer as a claimed "authority" is
/// `collateral_account` (the one account it actually received) — which is neither a signer nor
/// `loan_vault`'s recorded owner/delegate, so the real SPL Token program rejects the transfer. The
/// attempt is real (a genuine CPI to the real token program), not simulated, so a passing
/// `A-CPI-01` test proves the rejection happens at the token-program boundary, not merely "this
/// program chose not to."
fn drain_vault<'info>(ctx: &Context<'info, Attack<'info>>, amount: u64) -> Result<()> {
    let destination = ctx
        .remaining_accounts
        .first()
        .ok_or(HostileCallbackError::MissingDrainDestination)?;

    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.loan_token_program.key(),
            TransferChecked {
                from: ctx.accounts.loan_vault.to_account_info(),
                mint: ctx.accounts.loan_mint.to_account_info(),
                to: destination.clone(),
                // The one account this program actually holds -- neither a signer nor
                // `loan_vault`'s real owner (the Market PDA). No `invoke_signed`/seeds are used
                // because this program has no PDA that is `loan_vault`'s authority to begin with.
                authority: ctx.accounts.collateral_account.to_account_info(),
            },
        ),
        amount,
        0,
    )?;
    Ok(())
}

/// A-CPI-02: attempts to CPI directly back into Aegis's `liquidate`. Every account meta below is
/// deliberately marked `is_signer: false` -- this program has no real signer to offer at all
/// (ADR-0013: neither `market` nor `liquidator` was ever forwarded to it), and marking one `true`
/// anyway would fail on `PrivilegeEscalation` at the `invoke()` boundary itself for a reason
/// unrelated to reentrancy. Even kept honest, this call still has no path to succeed -- it lacks
/// the real `market`/`position`/`liquidator` accounts entirely, so in practice it fails on a
/// missing-account/dispatch error before or at the same boundary where the runtime's separate
/// indirect-reentrancy check (RV-6, `InvokeContext::push`,
/// `InstructionError::ReentrancyNotAllowed`) would otherwise apply. `tests/
/// phase8_hostile_callback.rs` does not assert which specific error surfaces here, precisely to
/// avoid crediting the wrong mechanism (`docs/ecosystem-research.md` §16.1) -- it asserts only
/// that reentry never succeeds. The Aegis-level guard itself is proven directly and separately, by
/// unit-testing the handler with the guard already set, independent of any CPI at all.
fn reenter<'info>(ctx: &Context<'info, Attack<'info>>) -> Result<()> {
    let ix = Instruction {
        program_id: aegis_program_id(),
        accounts: vec![
            AccountMeta::new(ctx.accounts.collateral_account.key(), false),
            AccountMeta::new(ctx.accounts.loan_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.collateral_mint.key(), false),
            AccountMeta::new_readonly(ctx.accounts.loan_mint.key(), false),
        ],
        data: vec![0u8; 8],
    };
    let account_infos = [
        ctx.accounts.collateral_account.to_account_info(),
        ctx.accounts.loan_vault.to_account_info(),
        ctx.accounts.collateral_mint.to_account_info(),
        ctx.accounts.loan_mint.to_account_info(),
    ];
    invoke(&ix, &account_infos)?;
    Ok(())
}

/// A-CPI-03: a tight, loop-carried-dependency loop that LLVM cannot reduce to a closed form, so
/// each iteration genuinely costs compute units in the SBF VM. `iterations` is caller-controlled
/// so a test can size it precisely against whatever compute limit it sets for the outer
/// transaction, rather than relying on a guessed magic constant.
fn burn_compute(iterations: u64) -> Result<()> {
    let mut acc: u64 = 0x9E3779B97F4A7C15;
    for i in 0..iterations {
        acc = acc.wrapping_mul(0xBF58476D1CE4E5B9).wrapping_add(i);
    }
    // Force the compiler to treat `acc` as used, so the loop cannot be optimized away entirely.
    msg!("burn_compute acc={}", acc);
    Ok(())
}

#[derive(Accounts)]
pub struct Attack<'info> {
    /// CHECK: whatever Aegis gives us; unused directly in some modes.
    #[account(mut)]
    pub collateral_account: UncheckedAccount<'info>,
    /// CHECK: Aegis's loan vault -- the target of `DrainVault`.
    #[account(mut)]
    pub loan_vault: UncheckedAccount<'info>,
    /// CHECK: unused directly; present for positional compatibility with a real callback.
    pub collateral_mint: UncheckedAccount<'info>,
    /// CHECK: used as the `mint` argument to the forged `DrainVault` transfer attempt.
    pub loan_mint: UncheckedAccount<'info>,
    /// CHECK: unused directly; present for positional compatibility with a real callback.
    pub collateral_token_program: UncheckedAccount<'info>,
    /// CHECK: the real SPL/Token-2022 program `DrainVault` (genuinely, and unsuccessfully) CPIs
    /// into.
    pub loan_token_program: UncheckedAccount<'info>,
}
