//! `example-liquidator` — a minimal, deterministic, **honest** implementation of the Aegis Phase 8
//! liquidation-callback interface (`docs/instruction-catalogue.md` §17, `docs/composability.md`,
//! ADR-0013). This is a **lab/example, not production exchange infrastructure**: it swaps at a
//! fixed, caller-supplied exchange rate against its own pre-funded reserve — no AMM, no order
//! book, no price discovery. Its only purpose is to prove the composability path end-to-end in
//! `I-LIQ-CB-01`, fully offline and fully deterministic.
//!
//! ## The account contract this program expects (must match Aegis's callback CPI exactly)
//!
//! Aegis (`programs/aegis/src/instructions/liquidate/liquidate.rs::build_callback_instruction`)
//! invokes this program's `handle_liquidation` with, positionally:
//!
//! 1. `collateral_account` (mut) — this program's own token account, owned by its `authority` PDA,
//!    already funded with the seized collateral by the time this instruction runs.
//! 2. `loan_vault` (mut) — Aegis's loan vault; this program must end by transferring the exact
//!    computed repayment into it.
//! 3. `collateral_mint` (readonly)
//! 4. `loan_mint` (readonly)
//! 5. `collateral_token_program` (readonly)
//! 6. `loan_token_program` (readonly)
//!
//! Then, as `remaining_accounts` on the OUTER `liquidate` call (forwarded verbatim by Aegis) —
//! this program's own two accounts, which Aegis never inspects:
//!
//! 7. `authority` — this program's own PDA (`seeds = [b"authority"]`), signs internally.
//! 8. `loan_reserve` (mut) — this program's own pre-funded loan-asset reserve, owned by
//!    `authority`, the source of the repayment.
//!
//! `handle_liquidation`'s instruction data is exactly what the liquidator supplied as
//! `callback_data` to Aegis's `liquidate` — a `u128` WAD-scaled exchange rate (loan-asset base
//! units per one whole collateral-asset unit, decimals-adjusted below). Aegis never inspects or
//! constrains this data; it is entirely this program's own interface.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

declare_id!("CyexeWx6KSzkD4HtCges24DYWMt8ny4wnsjvE39iao1v");

const WAD: u128 = 1_000_000_000_000_000_000;

#[error_code]
pub enum ExampleLiquidatorError {
    #[msg("no collateral was received before this callback ran")]
    NothingReceived,
    #[msg("computed repayment exceeds this program's loan-asset reserve")]
    InsufficientReserve,
    #[msg("rate_wad must be nonzero")]
    ZeroRate,
    #[msg("arithmetic overflow computing the deterministic swap output")]
    Overflow,
}

/// output = seized * rate_wad / WAD, adjusted for the collateral/loan decimals difference.
/// Pure, checked, no floating point — same discipline as `aegis-math`, even though this lab
/// program is deliberately independent of that crate (it is not part of the audited protocol).
fn compute_swap_output(
    seized: u64,
    rate_wad: u128,
    collateral_decimals: u8,
    loan_decimals: u8,
) -> Result<u64> {
    require!(rate_wad > 0, ExampleLiquidatorError::ZeroRate);

    let mut numerator = (seized as u128)
        .checked_mul(rate_wad)
        .ok_or(ExampleLiquidatorError::Overflow)?;

    if loan_decimals >= collateral_decimals {
        let scale = 10u128
            .checked_pow((loan_decimals - collateral_decimals) as u32)
            .ok_or(ExampleLiquidatorError::Overflow)?;
        numerator = numerator
            .checked_mul(scale)
            .ok_or(ExampleLiquidatorError::Overflow)?;
    } else {
        let scale = 10u128
            .checked_pow((collateral_decimals - loan_decimals) as u32)
            .ok_or(ExampleLiquidatorError::Overflow)?;
        numerator /= scale;
    }

    let output = numerator / WAD;
    u64::try_from(output).map_err(|_| error!(ExampleLiquidatorError::Overflow))
}

#[program]
pub mod example_liquidator {
    use super::*;

    pub fn handle_liquidation(ctx: Context<HandleLiquidation>, rate_wad: u128) -> Result<()> {
        // Measured, not trusted from instruction data: the seized amount is whatever Aegis
        // actually transferred into `collateral_account` before this CPI ran.
        let seized = ctx.accounts.collateral_account.amount;
        require!(seized > 0, ExampleLiquidatorError::NothingReceived);

        let output = compute_swap_output(
            seized,
            rate_wad,
            ctx.accounts.collateral_mint.decimals,
            ctx.accounts.loan_mint.decimals,
        )?;

        require!(
            ctx.accounts.loan_reserve.amount >= output,
            ExampleLiquidatorError::InsufficientReserve
        );

        let bump = ctx.bumps.authority;
        let signer_seeds: &[&[u8]] = &[b"authority", &[bump]];

        token_interface::transfer_checked(
            CpiContext::new(
                ctx.accounts.loan_token_program.key(),
                TransferChecked {
                    from: ctx.accounts.loan_reserve.to_account_info(),
                    mint: ctx.accounts.loan_mint.to_account_info(),
                    to: ctx.accounts.loan_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            )
            .with_signer(&[signer_seeds]),
            output,
            ctx.accounts.loan_mint.decimals,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct HandleLiquidation<'info> {
    #[account(mut)]
    pub collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Aegis's loan vault. A plain SPL destination for `transfer_checked` — no ownership
    /// check needed on this side; Aegis itself measures the resulting delta.
    #[account(mut)]
    pub loan_vault: UncheckedAccount<'info>,

    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,
    pub loan_mint: Box<InterfaceAccount<'info, Mint>>,

    pub collateral_token_program: Interface<'info, TokenInterface>,
    pub loan_token_program: Interface<'info, TokenInterface>,

    /// CHECK: this program's own PDA; signs internally for `loan_reserve`'s outgoing transfer.
    #[account(seeds = [b"authority"], bump)]
    pub authority: UncheckedAccount<'info>,

    // Mint correctness for both token accounts is enforced by `transfer_checked` itself (the SPL
    // Token program rejects a mismatched `mint` argument) -- no redundant Anchor-level constraint
    // needed here, and `collateral_mint`/`loan_mint` are declared after this struct's first two
    // fields, so a forward-referencing constraint on them would not compile.
    #[account(mut)]
    pub loan_reserve: Box<InterfaceAccount<'info, TokenAccount>>,
}
