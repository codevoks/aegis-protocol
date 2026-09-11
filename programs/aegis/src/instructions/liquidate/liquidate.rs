//! `liquidate` — the most dangerous instruction in the protocol (`instruction-catalogue.md` §17,
//! `docs/phases/phase-06-liquidation.md`, `economic-model.md` §7), extended in Phase 8
//! (`docs/composability.md`, `docs/phases/phase-08-composability.md`, ADR-0013) with an
//! **optional** callback so a liquidator can seize collateral, swap it via an untrusted external
//! program, and repay in one transaction with no pre-funding.
//!
//! **Ordering is security-critical (INV-ORA-07), exactly mirroring `borrow`'s precedent:**
//! oracle validation happens first, strictly before `accrue_mut` or any other state write, so a
//! failed oracle check leaves nothing modified. `HF < WAD` is checked **strictly** (E-12,
//! INV-LIQ-01/INV-SOLV-02) — `HF == WAD` is never liquidatable, and nothing in this file uses
//! `<=` against a health factor.
//!
//! No owner signature is required (`account-model.md` §5.1, INV-AUTH-03): liquidation is
//! permissionless by design, including self-liquidation by the position's own owner (`U-LIQ-07`,
//! `threat-model.md` T-22) — not blocked with a special case, since doing so would also block
//! legitimate third-party liquidator bots that happen to share a signer with the position owner
//! in a test fixture, and self-liquidation is provably unprofitable rather than dangerous.
//!
//! ## Phase 8: the callback is trusted for nothing
//!
//! Both branches below share everything through the point where `outcome`/`repay_shares` are
//! computed — that shared prefix is byte-for-byte what Phase 6 shipped. They then diverge:
//!
//! - **`callback_program == None`:** the exact, unmodified Phase 6 sequence (`I-LIQ-CB-02`) — pull
//!   the liquidator's repayment first, update accounting, then push seized collateral out.
//! - **`callback_program == Some(_)`:** update accounting, seize collateral into a
//!   liquidator-supplied, callback-controlled account, set a per-`Market` reentrancy guard, CPI
//!   into the callback with a minimal, explicitly-constructed account list that **never includes
//!   the `Market` or `liquidator` signer** (ADR-0013), clear the guard, **reload `loan_vault`**
//!   and measure the actual delta, and require that delta to meet or exceed the exact computed
//!   repayment (any surplus is ordinary unaccounted vault surplus, per INV-CUS-08).
//!   The callback's return value and any instruction-data claim it makes are never trusted for
//!   this — only the measured vault delta is.

use crate::constants::{COLLATERAL_VAULT_SEED, LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED};
use crate::error::AegisError;
use crate::events::Liquidated;
use crate::guards::require_exactly_one_u64;
use crate::oracle::{self, PriceBand};
use crate::state::{Market, Position};
use crate::token::transfer::{transfer_checked_in, transfer_checked_out};
use aegis_math::{
    collateral_value, compute_liquidation_by_repay, compute_liquidation_by_seize, debt_value,
    health_factor, is_liquidatable, to_assets_up, to_shares_down, LiquidationOutcome,
    LiquidationParams,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Anyone -- permissionless, including the position's own owner (self-liquidation, U-LIQ-07).
    #[account(mut)]
    pub liquidator: Signer<'info>,

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

    /// No `has_one = owner` -- the liquidator need not be, and is never required to be, the
    /// position owner.
    #[account(
        mut,
        has_one = market @ AegisError::PositionMarketMismatch,
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
        seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()],
        bump = market.collateral_vault_bump,
        address = market.collateral_vault @ AegisError::VaultMismatch,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = liquidator_loan_ata.mint == market.loan_mint @ AegisError::VaultMintMismatch,
    )]
    pub liquidator_loan_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Not constrained to any particular owner (`instruction-catalogue.md` §17: "a liquidator may
    /// direct proceeds anywhere" -- the amount is fully determined by protocol state, not by who
    /// controls this account).
    #[account(
        mut,
        constraint = liquidator_collateral_ata.mint == market.collateral_mint @ AegisError::VaultMintMismatch,
    )]
    pub liquidator_collateral_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = market.loan_mint @ AegisError::VaultMintMismatch)]
    pub loan_mint: InterfaceAccount<'info, Mint>,
    #[account(address = market.collateral_mint @ AegisError::VaultMintMismatch)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub loan_token_program: Interface<'info, TokenInterface>,
    pub collateral_token_program: Interface<'info, TokenInterface>,

    /// CHECK: validated field-by-field by `oracle::require_valid_price` (O-1..O-11).
    pub collateral_price_update: UncheckedAccount<'info>,
    /// CHECK: as `collateral_price_update`, for the loan asset's feed.
    pub loan_price_update: UncheckedAccount<'info>,

    // ---- Phase 8: optional liquidation callback (docs/composability.md, ADR-0013) ----
    /// The liquidator-selected, **UNTRUSTED** callback program. `None` reproduces Phase 6 behavior
    /// exactly (`I-LIQ-CB-02`); a liquidator who never supplies this account needs no callback
    /// program, no callback-specific remaining accounts, and no extra external dependency.
    ///
    /// CHECK: only `.executable` and `.key()` are ever read from this account. Every account the
    /// callback receives is constructed explicitly in the handler (`build_callback_instruction`) —
    /// this account is never itself passed into the callback's own CPI account list, and it is
    /// never a signer (Aegis dispatches with `invoke`, never `invoke_signed`, for this CPI).
    pub callback_program: Option<UncheckedAccount<'info>>,

    /// Required iff `callback_program` is `Some`: receives the seized collateral before the
    /// callback CPI. Ownership is deliberately unconstrained (typically the callback's own PDA) —
    /// Aegis pins only the mint, per the least-privilege callback contract (ADR-0013 §1.4): Aegis
    /// does not need to know or care who controls this account, only that it can safely receive
    /// `market.collateral_mint`.
    #[account(
        mut,
        constraint = callback_collateral_account.mint == market.collateral_mint @ AegisError::LiquidationCallbackAccountMismatch,
    )]
    pub callback_collateral_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
}

/// Builds the exact CPI instruction Aegis sends to the untrusted liquidation callback (ADR-0013
/// §1.4), including the safety check that no opaque `remaining_account` aliases a protected Aegis
/// key. A standalone, unit-testable function precisely so `INV-AUTH-07`/`A-AUTH-07` can assert on
/// the constructed `accounts` list directly (the account-meta list itself, not just the source
/// code that builds it).
///
/// Every `AccountMeta` this function produces has `is_signer: false` — unconditionally, regardless
/// of whether the underlying `AccountInfo` happens to be a real transaction signer — which is what
/// makes "no signer forwarded" a property of this one function's output rather than a habit that
/// has to be maintained at every call site.
#[allow(clippy::too_many_arguments)]
pub fn build_callback_instruction(
    callback_program: Pubkey,
    callback_collateral_account: &AccountInfo,
    loan_vault: &AccountInfo,
    collateral_mint: &AccountInfo,
    loan_mint: &AccountInfo,
    collateral_token_program: &AccountInfo,
    loan_token_program: &AccountInfo,
    remaining_accounts: &[AccountInfo],
    protected_keys: &[Pubkey],
    callback_data: Vec<u8>,
) -> Result<Instruction> {
    let mut metas = vec![
        AccountMeta::new(*callback_collateral_account.key, false),
        AccountMeta::new(*loan_vault.key, false),
        AccountMeta::new_readonly(*collateral_mint.key, false),
        AccountMeta::new_readonly(*loan_mint.key, false),
        AccountMeta::new_readonly(*collateral_token_program.key, false),
        AccountMeta::new_readonly(*loan_token_program.key, false),
    ];

    for acc in remaining_accounts {
        require!(
            !protected_keys.contains(acc.key),
            AegisError::CallbackAccountNotPermitted
        );
        metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: acc.is_writable,
        });
    }

    Ok(Instruction {
        program_id: callback_program,
        accounts: metas,
        data: callback_data,
    })
}

/// Shared by both branches: applies `outcome`'s accounting mutations to `market`/`position`
/// (`economic-model.md` §7.3). Never called before the oracle has been validated and the
/// liquidatability check has passed.
fn apply_liquidation_accounting(
    market: &mut Market,
    position: &mut Position,
    outcome: &LiquidationOutcome,
    repay_shares: u128,
) -> Result<()> {
    market.total_borrow_shares = market
        .total_borrow_shares
        .checked_sub(repay_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;
    market.total_borrow_assets = market
        .total_borrow_assets
        .checked_sub(outcome.repay_assets)
        .ok_or(AegisError::ArithmeticOverflow)?;
    // total_supply_assets is deliberately unchanged: lenders are repaid, never enriched, by a
    // liquidation (INV-LIQ-09, U-LIQ-06).
    market.collateral_fee_accrued = market
        .collateral_fee_accrued
        .checked_add(outcome.protocol_cut)
        .ok_or(AegisError::ArithmeticOverflow)?;

    position.borrow_shares = position
        .borrow_shares
        .checked_sub(repay_shares)
        .ok_or(AegisError::ArithmeticOverflow)?;
    position.collateral_amount = position
        .collateral_amount
        .checked_sub(outcome.total_seize)
        .ok_or(AegisError::ArithmeticOverflow)?;
    Ok(())
}

/// Shared by both branches: `hf_after`, recomputed from the now-mutated position/market totals
/// against the SAME already-validated price bands (evidence trail for `P-LIQ-1`).
fn compute_hf_after(
    market: &Market,
    position: &Position,
    collateral_band: PriceBand,
    loan_band: PriceBand,
) -> Result<u128> {
    let debt_after = to_assets_up(
        position.borrow_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;
    let cv_after = collateral_value(
        position.collateral_amount,
        collateral_band.lo,
        market.collateral_decimals,
    )
    .map_err(AegisError::from)?;
    let dv_after =
        debt_value(debt_after, loan_band.hi, market.loan_decimals).map_err(AegisError::from)?;
    Ok(health_factor(cv_after, market.liq_threshold, dv_after).map_err(AegisError::from)?)
}

pub fn handler<'info>(
    ctx: Context<'info, Liquidate<'info>>,
    repay_assets: u64,
    seize_collateral: u64,
    callback_data: Vec<u8>,
) -> Result<()> {
    // Phase 8 / ADR-0013: checked unconditionally, before anything else, in both branches. This
    // is always 0 outside an active callback CPI, so it never changes observable Phase 6 behavior
    // for a no-callback call (I-LIQ-CB-02) -- it is the Aegis-level defense that holds regardless
    // of RV-6's finding that the runtime already rejects indirect A -> callback -> A reentrancy
    // (docs/ecosystem-research.md §16.1).
    require!(
        ctx.accounts.market.liquidation_guard == 0,
        AegisError::LiquidationCallbackReentrancy
    );

    let has_callback = ctx.accounts.callback_program.is_some();
    require_eq!(
        has_callback,
        ctx.accounts.callback_collateral_account.is_some(),
        AegisError::LiquidationCallbackAccountMismatch
    );
    if let Some(callback_program) = &ctx.accounts.callback_program {
        require!(
            callback_program.to_account_info().executable,
            AegisError::LiquidationCallbackNotExecutable
        );
    }

    require_exactly_one_u64(repay_assets, seize_collateral)?;
    require_keys_eq!(
        ctx.accounts.loan_token_program.key(),
        ctx.accounts.market.loan_token_program,
        AegisError::TokenProgramMismatch
    );
    require_keys_eq!(
        ctx.accounts.collateral_token_program.key(),
        ctx.accounts.market.collateral_token_program,
        AegisError::TokenProgramMismatch
    );

    // INV-ORA-07: validate the oracle for BOTH assets before any state write -- the first
    // fallible operation that reads caller-supplied external data.
    let now = Clock::get()?.unix_timestamp;
    let (collateral_band, loan_band) = oracle::require_valid_price(
        &ctx.accounts.market,
        &ctx.accounts.collateral_price_update.to_account_info(),
        &ctx.accounts.loan_price_update.to_account_info(),
        now,
    )?;

    require!(
        ctx.accounts.position.collateral_amount > 0,
        AegisError::NothingToLiquidate
    );

    // Accrue under the already-validated price, exactly as `borrow` does -- this is the first
    // state write, and everything below uses the now-fully-accrued totals.
    ctx.accounts
        .market
        .accrue_mut(&mut ctx.accounts.fee_position, now)?;

    let market = &ctx.accounts.market;
    let position = &ctx.accounts.position;

    let debt_assets = to_assets_up(
        position.borrow_shares,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;

    let cv = collateral_value(
        position.collateral_amount,
        collateral_band.lo,
        market.collateral_decimals,
    )
    .map_err(AegisError::from)?;
    let dv =
        debt_value(debt_assets, loan_band.hi, market.loan_decimals).map_err(AegisError::from)?;
    let hf_before = health_factor(cv, market.liq_threshold, dv).map_err(AegisError::from)?;

    // INV-LIQ-01 / INV-SOLV-02 / E-12: STRICT. HF == WAD is not liquidatable.
    require!(is_liquidatable(hf_before), AegisError::NotLiquidatable);

    let params = LiquidationParams {
        collateral_decimals: market.collateral_decimals,
        loan_decimals: market.loan_decimals,
        price_c_lo: collateral_band.lo,
        price_l_hi: loan_band.hi,
        liq_bonus: market.liq_bonus,
        liq_protocol_fee: market.liq_protocol_fee,
        close_factor: market.close_factor,
        full_liq_hf: market.full_liq_hf,
        min_debt: market.min_debt,
    };

    let outcome = if repay_assets > 0 {
        compute_liquidation_by_repay(
            &params,
            debt_assets,
            position.collateral_amount,
            hf_before,
            repay_assets,
        )
    } else {
        compute_liquidation_by_seize(
            &params,
            debt_assets,
            position.collateral_amount,
            hf_before,
            seize_collateral,
        )
    }
    .map_err(AegisError::from)?;

    // economic-model.md §7.3: repay_shares from repay_assets, floored, then clamped to what the
    // position actually owes.
    let mut repay_shares = to_shares_down(
        outcome.repay_assets,
        market.total_borrow_assets,
        market.total_borrow_shares,
    )
    .map_err(AegisError::from)?;
    repay_shares = repay_shares.min(position.borrow_shares);

    if has_callback {
        // ================= Phase 8: callback branch =================
        let callback_program_key = ctx.accounts.callback_program.as_ref().unwrap().key();

        // 1. Apply accounting mutations up front -- safe because any later `?` failure reverts
        //    the whole transaction atomically (ADR-0013's "no partial state" guarantee comes from
        //    Solana's own atomicity, not from ordering these calls after the repayment check).
        apply_liquidation_accounting(
            &mut ctx.accounts.market,
            &mut ctx.accounts.position,
            &outcome,
            repay_shares,
        )?;

        // 2. Seize collateral into the callback-controlled account, market PDA signs (the same
        //    signer seeds and transfer_checked_out helper as the no-callback path -- a new
        //    destination on an existing, already-reviewed signer path).
        let market_key = ctx.accounts.market.key();
        let collateral_mint_key = ctx.accounts.market.collateral_mint;
        let loan_mint_key = ctx.accounts.market.loan_mint;
        let config_id_bytes = ctx.accounts.market.config_id.to_le_bytes();
        let market_bump = ctx.accounts.market.bump;
        let signer_seeds: &[&[u8]] = &[
            MARKET_SEED,
            collateral_mint_key.as_ref(),
            loan_mint_key.as_ref(),
            &config_id_bytes,
            &[market_bump],
        ];

        let callback_collateral_account = ctx
            .accounts
            .callback_collateral_account
            .as_ref()
            .unwrap()
            .to_account_info();

        transfer_checked_out(
            &ctx.accounts.collateral_vault.to_account_info(),
            &ctx.accounts.collateral_mint.to_account_info(),
            &callback_collateral_account,
            &ctx.accounts.market.to_account_info(),
            &ctx.accounts.collateral_token_program.to_account_info(),
            outcome.to_liquidator,
            ctx.accounts.collateral_mint.decimals,
            signer_seeds,
        )?;

        // 3. Set the reentrancy guard and flush it to the account's actual on-chain buffer NOW --
        //    Anchor's `Account<T>` only auto-syncs at the end of the whole instruction, but a
        //    hostile callback CPI-ing back into `liquidate` on this Market deserializes the
        //    CURRENT buffer, so the guard must be visible before the CPI, not just in our own
        //    in-memory copy (ADR-0013 §1.5, A-CPI-02).
        ctx.accounts.market.liquidation_guard = 1;
        ctx.accounts.market.exit(&crate::ID)?;

        // 4. Capture the pre-CPI loan_vault balance for the measured-delta repayment check.
        let loan_vault_before = ctx.accounts.loan_vault.amount;

        // 5. Build and dispatch the callback CPI (ADR-0013 §1.4): explicit, minimal account list,
        //    plain `invoke` (never `invoke_signed`) -- neither `market` nor `liquidator`'s
        //    AccountInfo is ever placed in this list, so no signer privilege reaches the callback.
        let protected_keys = [
            market_key,
            ctx.accounts.liquidator.key(),
            ctx.accounts.position.key(),
            ctx.accounts.fee_position.key(),
            ctx.accounts.collateral_vault.key(),
            ctx.accounts.liquidator_loan_ata.key(),
            ctx.accounts.liquidator_collateral_ata.key(),
        ];
        let loan_vault_info = ctx.accounts.loan_vault.to_account_info();
        let collateral_mint_info = ctx.accounts.collateral_mint.to_account_info();
        let loan_mint_info = ctx.accounts.loan_mint.to_account_info();
        let collateral_token_program_info = ctx.accounts.collateral_token_program.to_account_info();
        let loan_token_program_info = ctx.accounts.loan_token_program.to_account_info();

        let ix = build_callback_instruction(
            callback_program_key,
            &callback_collateral_account,
            &loan_vault_info,
            &collateral_mint_info,
            &loan_mint_info,
            &collateral_token_program_info,
            &loan_token_program_info,
            ctx.remaining_accounts,
            &protected_keys,
            callback_data,
        )?;

        let mut account_infos: Vec<AccountInfo> = vec![
            callback_collateral_account.clone(),
            loan_vault_info.clone(),
            collateral_mint_info,
            loan_mint_info,
            collateral_token_program_info,
            loan_token_program_info,
        ];
        account_infos.extend(ctx.remaining_accounts.iter().cloned());

        // Any error here (including the callback exhausting compute, A-CPI-03) propagates via
        // `?`, reverting the ENTIRE transaction atomically -- no special-case rollback logic
        // needed for A-CPI-01/03/04's "no partial state" requirement.
        invoke(&ix, &account_infos)?;

        // 6. Clear the guard (only reached on success; a failed CPI already reverted everything).
        ctx.accounts.market.liquidation_guard = 0;

        // 7. RE-READ: reload loan_vault and measure the delta. The callback's return value, its
        //    instruction data, and any quoted swap output are never consulted -- only this
        //    measured delta decides whether the repayment happened (A-CPI-04). The delta must
        //    meet or exceed the exact required repayment -- not equal it precisely -- because a
        //    real external swap essentially never lands on the exact wei amount required; any
        //    surplus becomes ordinary unaccounted vault surplus, exactly like an unsolicited
        //    direct transfer (INV-CUS-08), never credited beyond `outcome.repay_assets`. Anything
        //    less is rejected outright.
        ctx.accounts.loan_vault.reload()?;
        let loan_vault_after = ctx.accounts.loan_vault.amount;
        let actual_delta = loan_vault_after
            .checked_sub(loan_vault_before)
            .ok_or(AegisError::VaultAccountingError)?;
        require!(
            actual_delta >= outcome.repay_assets,
            AegisError::LiquidationCallbackRepaymentShortfall
        );

        // 8. Re-verify: hf_after, recomputed from the now-mutated (and now confirmed-repaid)
        //    position/market state.
        let hf_after = compute_hf_after(
            &ctx.accounts.market,
            &ctx.accounts.position,
            collateral_band,
            loan_band,
        )?;

        emit!(Liquidated {
            market: market_key,
            position: ctx.accounts.position.key(),
            liquidator: ctx.accounts.liquidator.key(),
            repay_assets: outcome.repay_assets,
            repay_shares,
            base_seize: outcome.base_seize,
            total_seize: outcome.total_seize,
            bonus_amount: outcome.bonus_amount,
            protocol_cut: outcome.protocol_cut,
            to_liquidator: outcome.to_liquidator,
            clamped: outcome.clamped,
            hf_before,
            hf_after,
            callback_program: Some(callback_program_key),
        });
    } else {
        // ================= Phase 6, unmodified (I-LIQ-CB-02) =================

        // --- receive the liquidator's repayment (measured delta; loan assets are fee-free by
        // policy, so credited == outcome.repay_assets is expected but verified, never assumed). ---
        let credited = transfer_checked_in(
            &ctx.accounts.liquidator_loan_ata.to_account_info(),
            &ctx.accounts.loan_mint.to_account_info(),
            &mut ctx.accounts.loan_vault,
            &ctx.accounts.liquidator.to_account_info(),
            &ctx.accounts.loan_token_program.to_account_info(),
            outcome.repay_assets,
            ctx.accounts.loan_mint.decimals,
        )?;
        require_eq!(
            credited,
            outcome.repay_assets,
            AegisError::VaultAccountingError
        );

        apply_liquidation_accounting(
            &mut ctx.accounts.market,
            &mut ctx.accounts.position,
            &outcome,
            repay_shares,
        )?;

        // hf_after, for the event and as on-chain evidence for P-LIQ-1: recomputed from the
        // now-mutated position/market totals against the SAME already-validated price bands.
        let hf_after = compute_hf_after(
            &ctx.accounts.market,
            &ctx.accounts.position,
            collateral_band,
            loan_band,
        )?;

        // --- transfer net seized collateral; protocol_cut stays physically in the vault ---
        let market = &ctx.accounts.market;
        let market_key = market.key();
        let config_id_bytes = market.config_id.to_le_bytes();
        let signer_seeds: &[&[u8]] = &[
            MARKET_SEED,
            market.collateral_mint.as_ref(),
            market.loan_mint.as_ref(),
            &config_id_bytes,
            &[market.bump],
        ];

        transfer_checked_out(
            &ctx.accounts.collateral_vault.to_account_info(),
            &ctx.accounts.collateral_mint.to_account_info(),
            &ctx.accounts.liquidator_collateral_ata.to_account_info(),
            &market.to_account_info(),
            &ctx.accounts.collateral_token_program.to_account_info(),
            outcome.to_liquidator,
            ctx.accounts.collateral_mint.decimals,
            signer_seeds,
        )?;

        emit!(Liquidated {
            market: market_key,
            position: ctx.accounts.position.key(),
            liquidator: ctx.accounts.liquidator.key(),
            repay_assets: outcome.repay_assets,
            repay_shares,
            base_seize: outcome.base_seize,
            total_seize: outcome.total_seize,
            bonus_amount: outcome.bonus_amount,
            protocol_cut: outcome.protocol_cut,
            to_liquidator: outcome.to_liquidator,
            clamped: outcome.clamped,
            hf_before,
            hf_after,
            callback_program: None,
        });
    }

    Ok(())
}
