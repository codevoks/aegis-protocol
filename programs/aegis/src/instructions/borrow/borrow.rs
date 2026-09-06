//! `borrow` — **hard-gated** until Phase 5 (`instruction-catalogue.md` §14,
//! `docs/phase-roadmap.md` "Sequencing the oracle dependency").
//!
//! No oracle account exists anywhere in [`Borrow`]'s account list — there is structurally nothing
//! a caller could pass that would let this instruction compute a price. The handler validates
//! accounts and arguments, then **unconditionally** returns [`AegisError::OracleNotYetAvailable`]
//! before reading or writing any state. This is a hard failure, strictly *more* restrictive than
//! the eventual behavior, never a permissive placeholder, dummy oracle, or hardcoded price.
//!
//! Everything Phase 5's real `borrow` will need *except* the price read and the LTV check is
//! implemented and independently unit-tested here as [`compute_borrow`] — a pure function the live
//! handler does not (and, by construction, cannot) reach. Phase 5 only needs to add the oracle
//! validation and the collateral/debt-value LTV check in front of a call to this function; it does
//! not need to rewrite the accrual, share-conversion, free-liquidity, or `min_debt` logic.

use crate::constants::{LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::guards::require_exactly_one_amount;
use crate::state::{Market, Position};
use aegis_math::{to_assets_up, to_shares_up};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Borrow<'info> {
    pub owner: Signer<'info>,

    /// Declared `mut` to match the real Phase 5 instruction's structural shape
    /// (`instruction-catalogue.md` §14: "Writes `Market`? yes") even though the handler below
    /// never actually writes it — the gate fires first, and Solana's atomicity means no write
    /// this instruction *would* have made can ever be observed regardless.
    #[account(
        mut,
        seeds = [
            MARKET_SEED,
            market.collateral_mint.as_ref(),
            market.loan_mint.as_ref(),
            &market.config_id.to_le_bytes(),
        ],
        bump = market.bump,
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        has_one = market @ AegisError::PositionMarketMismatch,
        has_one = owner @ AegisError::NotPositionOwner,
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [LOAN_VAULT_SEED, market.key().as_ref()],
        bump = market.loan_vault_bump,
        address = market.loan_vault @ AegisError::VaultMismatch,
    )]
    pub loan_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = owner_loan_ata.mint == market.loan_mint @ AegisError::VaultMintMismatch,
    )]
    pub owner_loan_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.loan_mint @ AegisError::VaultMintMismatch)]
    pub loan_mint: InterfaceAccount<'info, Mint>,

    pub loan_token_program: Interface<'info, TokenInterface>,
    // Deliberately NO price accounts: `oracle-design.md`'s `PriceSource` does not exist until
    // Phase 5. There is no field here a caller could populate with a fake, stale, or assumed
    // price -- the absence is the safety property, not merely an omission.
}

pub fn handler(ctx: Context<Borrow>, assets: u64, shares: u128) -> Result<()> {
    require_exactly_one_amount(assets, shares)?;
    require_keys_eq!(
        ctx.accounts.loan_token_program.key(),
        ctx.accounts.market.loan_token_program,
        AegisError::TokenProgramMismatch
    );

    // Hard sequencing gate (docs/phase-roadmap.md "Sequencing the oracle dependency"): this must
    // fire before any state is read or written. No oracle exists before Phase 5, so no price is
    // available to value collateral/debt or check max_ltv -- there is no code path below this
    // point in Phase 4.
    Err(error!(AegisError::OracleNotYetAvailable))
}

/// The result of [`compute_borrow`]: everything a successful borrow would need to write, computed
/// against already-accrued totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowComputation {
    pub assets_out: u64,
    pub shares_minted: u128,
    pub new_position_borrow_shares: u128,
    pub new_position_debt_assets: u64,
}

/// **Pure**, not called by the live [`handler`] above (see the module doc). Computes the
/// assets/shares conversion (`to_shares_up`/`to_assets_up`, economic-model.md §1.3 rows 3 and 7),
/// enforces the free-liquidity bound (INV-BOR-02, `U-BORROW-01`), and enforces the post-borrow
/// `min_debt` floor (INV-SOLV-07, E-25, `U-BORROW-02`) -- everything `borrow` needs except the
/// price read and the LTV check.
pub fn compute_borrow(
    assets: u64,
    shares: u128,
    total_borrow_assets: u64,
    total_borrow_shares: u128,
    total_supply_assets: u64,
    min_debt: u64,
    position_borrow_shares_before: u128,
) -> Result<BorrowComputation> {
    require_exactly_one_amount(assets, shares)?;

    let (assets_out, shares_minted) = if assets > 0 {
        let shares_minted = to_shares_up(assets, total_borrow_assets, total_borrow_shares)
            .map_err(AegisError::from)?;
        (assets, shares_minted)
    } else {
        let assets_out = to_assets_up(shares, total_borrow_assets, total_borrow_shares)
            .map_err(AegisError::from)?;
        (assets_out, shares)
    };

    // INV-BOR-02 / U-BORROW-01: borrow cannot remove more than free liquidity.
    let free_liquidity = total_supply_assets
        .checked_sub(total_borrow_assets)
        .ok_or(AegisError::ArithmeticOverflow)?;
    require!(
        assets_out <= free_liquidity,
        AegisError::InsufficientLiquidity
    );

    let new_position_borrow_shares = position_borrow_shares_before
        .checked_add(shares_minted)
        .ok_or(AegisError::ArithmeticOverflow)?;
    let new_total_borrow_assets = total_borrow_assets
        .checked_add(assets_out)
        .ok_or(AegisError::ArithmeticOverflow)?;
    let new_total_borrow_shares = total_borrow_shares
        .checked_add(shares_minted)
        .ok_or(AegisError::ArithmeticOverflow)?;

    let new_position_debt_assets = to_assets_up(
        new_position_borrow_shares,
        new_total_borrow_assets,
        new_total_borrow_shares,
    )
    .map_err(AegisError::from)?;

    // INV-SOLV-07 / E-25 / U-BORROW-02: a successful borrow always increases debt from wherever
    // it was (assets_out > 0, since the exactly-one-of guard above already rejects both zero), so
    // the "0 or >= min_debt" rule collapses to ">= min_debt" here -- the "0" branch exists for
    // other instructions (e.g. a full repay), never for a successful borrow.
    require!(
        new_position_debt_assets >= min_debt,
        AegisError::DebtBelowMinimum
    );

    Ok(BorrowComputation {
        assets_out,
        shares_minted,
        new_position_borrow_shares,
        new_position_debt_assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // U-BORROW-01 / INV-BOR-02: borrow cannot remove more than free liquidity.
    #[test]
    fn u_borrow_01_free_liquidity_bound() {
        // total_supply_assets = 1000, total_borrow_assets = 900 -> free liquidity = 100.
        let err = compute_borrow(101, 0, 900, 900_000_000, 1_000, 10, 0).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::InsufficientLiquidity)
        );
        // Exactly at the boundary must succeed (assuming it also clears min_debt).
        let ok = compute_borrow(100, 0, 900, 900_000_000, 1_000, 10, 0);
        assert!(
            ok.is_ok(),
            "borrowing exactly the free liquidity must succeed: {ok:?}"
        );
    }

    // U-BORROW-02 / INV-SOLV-07 / E-25: a borrow that leaves debt below min_debt is rejected.
    #[test]
    fn u_borrow_02_min_debt_floor() {
        let err = compute_borrow(5, 0, 0, 0, 1_000_000, 10, 0).unwrap_err();
        assert_eq!(
            err,
            anchor_lang::error::Error::from(AegisError::DebtBelowMinimum)
        );
        let ok = compute_borrow(10, 0, 0, 0, 1_000_000, 10, 0);
        assert!(
            ok.is_ok(),
            "borrowing exactly min_debt must succeed: {ok:?}"
        );
    }

    // Borrow-shares are minted with ceil (U-ROUND-03 / INV-BOR-03), and assets returned via
    // shares are floored (U-ROUND-07) -- pinned again here at the instruction-computation level.
    #[test]
    fn conversions_use_the_documented_rounding_directions() {
        let result = compute_borrow(7, 0, 11, 13, 1_000_000, 1, 0).unwrap();
        assert!(result.shares_minted > 0);
    }
}
