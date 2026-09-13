//! `set_market_params(args)` — admin parameter updates with full bounds re-validation, the
//! tighten/loosen timelock asymmetry, and `accrue_mut` before any parameter takes effect
//! (`instruction-catalogue.md` §7, `governance.md` §4, INV-ADM-05..09).
//!
//! **Identity fields (`collateral_mint`, `loan_mint`, both token programs, both vaults, both
//! decimals, `config_id`) are not present in [`SetMarketParamsArgs`] at all** — INV-ADM-06 is
//! satisfied by the args type having no field capable of expressing the change, not merely by a
//! runtime check (`A-ADM-06`).
//!
//! Ordering (architecture.md §2, INV-ADM-07): validate accounts → `accrue_mut` under the
//! **current** (about-to-be-superseded) parameters → validate the **proposed** parameters against
//! the canonical bounds → classify tighten/loosen → apply immediately or stage.

use crate::constants::{MARKET_SEED, PENDING_PARAMS_SEED, POSITION_SEED, PROTOCOL_SEED};
use crate::error::AegisError;
use crate::events::{MarketParamsUpdated, ParamsStaged};
use crate::guards::require_non_default_pubkey;
use crate::state::{Market, MutableMarketParams, PendingMarketParams, Position, Protocol};
use anchor_lang::prelude::*;
use anchor_lang::system_program;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SetMarketParamsArgs {
    pub oracle_kind: u8,
    pub collateral_feed_id: [u8; 32],
    pub loan_feed_id: [u8; 32],
    pub max_price_age_secs: u32,
    pub max_conf_bps: u16,

    pub max_ltv: u128,
    pub liq_threshold: u128,
    pub liq_bonus: u128,
    pub close_factor: u128,
    pub full_liq_hf: u128,
    pub liq_protocol_fee: u128,
    pub fee: u128,
    pub min_debt: u64,

    pub base_rate_ps: u128,
    pub slope1_ps: u128,
    pub slope2_ps: u128,
    pub u_kink: u128,
    pub max_rate_ps: u128,

    /// Not a risk parameter (absent from `governance.md` §4's tighten/loosen table) — always
    /// applied immediately, orthogonal to whatever else this call tightens or stages.
    pub fee_recipient: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: SetMarketParamsArgs)]
pub struct SetMarketParams<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
        has_one = admin @ AegisError::NotProtocolAdmin,
    )]
    pub protocol: Account<'info, Protocol>,

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

    /// The fee position for the market's **current** `fee_recipient` — accrual (under the old
    /// parameters, INV-ADM-07) always mints to whoever was authoritative before this call.
    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,

    /// Required, and must already exist (`init_position` is permissionless), exactly when
    /// `args.fee_recipient != market.fee_recipient` — proves the new fee position is reachable
    /// before this instruction ever points `market.fee_recipient` at it. Ignored entirely when
    /// `fee_recipient` is not changing.
    #[account(
        seeds = [POSITION_SEED, market.key().as_ref(), args.fee_recipient.as_ref()],
        bump = new_fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub new_fee_position: Option<Account<'info, Position>>,

    /// CHECK: manually managed, not a typed Anchor account. On the tightening path this account
    /// is never read or written. On the loosening path, the handler verifies it is not already
    /// initialized (else `PendingParamsAlreadyStaged`) and creates it itself via a manual
    /// `system_program::create_account` CPI signed with its own PDA seeds — the sanctioned
    /// Anchor-1.0 pattern for "create only if it doesn't already exist" now that the banned
    /// conditional-init constraint (AGENTS.md §7.10) is off the table.
    #[account(mut, seeds = [PENDING_PARAMS_SEED, market.key().as_ref()], bump)]
    pub pending_market_params: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[allow(clippy::too_many_arguments)]
pub fn handler(ctx: Context<SetMarketParams>, args: SetMarketParamsArgs) -> Result<()> {
    require_non_default_pubkey(args.fee_recipient, AegisError::DefaultPubkeyNotAllowed)?;

    // INV-ADM-07: settle accrued interest under the CURRENT parameters before anything else reads
    // or writes a parameter this instruction might change.
    let now = Clock::get()?.unix_timestamp;
    ctx.accounts
        .market
        .accrue_mut(&mut ctx.accounts.fee_position, now)?;

    // Full canonical bounds re-validation against the PROPOSED values -- the exact same functions
    // `create_market` uses, never a parallel/weaker copy (INV-ADM-05).
    Market::validate_risk_params(
        args.max_ltv,
        args.liq_threshold,
        args.liq_bonus,
        args.close_factor,
        args.full_liq_hf,
        args.liq_protocol_fee,
        args.fee,
        args.min_debt,
    )?;
    Market::validate_irm_params(
        args.base_rate_ps,
        args.slope1_ps,
        args.slope2_ps,
        args.u_kink,
        args.max_rate_ps,
    )?;
    Market::validate_oracle_config(args.max_price_age_secs, args.max_conf_bps)?;

    let old_params = MutableMarketParams::from_market(&ctx.accounts.market);
    let new_params = MutableMarketParams {
        oracle_kind: args.oracle_kind,
        collateral_feed_id: args.collateral_feed_id,
        loan_feed_id: args.loan_feed_id,
        max_price_age_secs: args.max_price_age_secs,
        max_conf_bps: args.max_conf_bps,
        max_ltv: args.max_ltv,
        liq_threshold: args.liq_threshold,
        liq_bonus: args.liq_bonus,
        close_factor: args.close_factor,
        full_liq_hf: args.full_liq_hf,
        liq_protocol_fee: args.liq_protocol_fee,
        fee: args.fee,
        min_debt: args.min_debt,
        base_rate_ps: args.base_rate_ps,
        slope1_ps: args.slope1_ps,
        slope2_ps: args.slope2_ps,
        u_kink: args.u_kink,
        max_rate_ps: args.max_rate_ps,
    };

    let old_fee_recipient = ctx.accounts.market.fee_recipient;
    let fee_recipient_changing = args.fee_recipient != old_fee_recipient;
    if fee_recipient_changing {
        let new_fee_position = ctx
            .accounts
            .new_fee_position
            .as_ref()
            .ok_or(AegisError::FeeRecipientPositionMissing)?;
        require_keys_eq!(
            new_fee_position.owner,
            args.fee_recipient,
            AegisError::FeeRecipientPositionMissing
        );
    }

    if MutableMarketParams::is_loosening(&old_params, &new_params) {
        // ---- Loosening: stage behind the timelock, apply nothing risk-related yet ----
        require!(
            ctx.accounts.pending_market_params.owner == &system_program::ID
                && ctx.accounts.pending_market_params.lamports() == 0,
            AegisError::PendingParamsAlreadyStaged
        );

        let effective_at = now
            .checked_add(crate::constants::PARAM_TIMELOCK_SECS)
            .ok_or(AegisError::ArithmeticOverflow)?;

        let market_key = ctx.accounts.market.key();
        let bump = ctx.bumps.pending_market_params;
        let signer_seeds: &[&[u8]] = &[PENDING_PARAMS_SEED, market_key.as_ref(), &[bump]];

        // The audited path: Anchor's own `system_program::create_account` (a `CpiContext`-based
        // wrapper), the exact same pattern `token/vault.rs`'s `create_vault` already uses for a
        // PDA-signed account creation -- never a raw `invoke_signed` call in this crate's own
        // source (`scripts/check-cpi-allowlist.sh` enforces this repo-wide).
        system_program::create_account(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                system_program::CreateAccount {
                    from: ctx.accounts.admin.to_account_info(),
                    to: ctx.accounts.pending_market_params.to_account_info(),
                },
            )
            .with_signer(&[signer_seeds]),
            Rent::get()?.minimum_balance(PendingMarketParams::LEN),
            PendingMarketParams::LEN as u64,
            &crate::ID,
        )?;

        let pending = PendingMarketParams {
            market: market_key,
            effective_at,
            oracle_kind: new_params.oracle_kind,
            collateral_feed_id: new_params.collateral_feed_id,
            loan_feed_id: new_params.loan_feed_id,
            max_price_age_secs: new_params.max_price_age_secs,
            max_conf_bps: new_params.max_conf_bps,
            max_ltv: new_params.max_ltv,
            liq_threshold: new_params.liq_threshold,
            liq_bonus: new_params.liq_bonus,
            close_factor: new_params.close_factor,
            full_liq_hf: new_params.full_liq_hf,
            liq_protocol_fee: new_params.liq_protocol_fee,
            fee: new_params.fee,
            min_debt: new_params.min_debt,
            base_rate_ps: new_params.base_rate_ps,
            slope1_ps: new_params.slope1_ps,
            slope2_ps: new_params.slope2_ps,
            u_kink: new_params.u_kink,
            max_rate_ps: new_params.max_rate_ps,
            bump,
            _reserved: [0u8; 32],
        };
        let mut data: Vec<u8> = Vec::with_capacity(PendingMarketParams::LEN);
        AccountSerialize::try_serialize(&pending, &mut data)?;
        ctx.accounts
            .pending_market_params
            .try_borrow_mut_data()?
            .copy_from_slice(&data);

        // fee_recipient is orthogonal to the risk-parameter timelock: apply it now even though
        // the risk bundle is staged.
        if fee_recipient_changing {
            ctx.accounts.market.fee_recipient = args.fee_recipient;
        }

        emit!(ParamsStaged {
            market: market_key,
            pending_market_params: ctx.accounts.pending_market_params.key(),
            admin: ctx.accounts.admin.key(),
            effective_at,
            oracle_kind: new_params.oracle_kind,
            collateral_feed_id: new_params.collateral_feed_id,
            loan_feed_id: new_params.loan_feed_id,
            max_price_age_secs: new_params.max_price_age_secs,
            max_conf_bps: new_params.max_conf_bps,
            max_ltv: new_params.max_ltv,
            liq_threshold: new_params.liq_threshold,
            liq_bonus: new_params.liq_bonus,
            close_factor: new_params.close_factor,
            full_liq_hf: new_params.full_liq_hf,
            liq_protocol_fee: new_params.liq_protocol_fee,
            fee: new_params.fee,
            min_debt: new_params.min_debt,
            base_rate_ps: new_params.base_rate_ps,
            slope1_ps: new_params.slope1_ps,
            slope2_ps: new_params.slope2_ps,
            u_kink: new_params.u_kink,
            max_rate_ps: new_params.max_rate_ps,
        });
    } else {
        // ---- Tightening (or a no-op risk bundle with only fee_recipient changing): immediate ----
        new_params.apply_to(&mut ctx.accounts.market);
        if fee_recipient_changing {
            ctx.accounts.market.fee_recipient = args.fee_recipient;
        }

        emit!(MarketParamsUpdated {
            market: ctx.accounts.market.key(),
            admin: ctx.accounts.admin.key(),
            old_fee_recipient,
            new_fee_recipient: ctx.accounts.market.fee_recipient,
            old_oracle_kind: old_params.oracle_kind,
            new_oracle_kind: new_params.oracle_kind,
            old_collateral_feed_id: old_params.collateral_feed_id,
            new_collateral_feed_id: new_params.collateral_feed_id,
            old_loan_feed_id: old_params.loan_feed_id,
            new_loan_feed_id: new_params.loan_feed_id,
            old_max_price_age_secs: old_params.max_price_age_secs,
            new_max_price_age_secs: new_params.max_price_age_secs,
            old_max_conf_bps: old_params.max_conf_bps,
            new_max_conf_bps: new_params.max_conf_bps,
            old_max_ltv: old_params.max_ltv,
            new_max_ltv: new_params.max_ltv,
            old_liq_threshold: old_params.liq_threshold,
            new_liq_threshold: new_params.liq_threshold,
            old_liq_bonus: old_params.liq_bonus,
            new_liq_bonus: new_params.liq_bonus,
            old_close_factor: old_params.close_factor,
            new_close_factor: new_params.close_factor,
            old_full_liq_hf: old_params.full_liq_hf,
            new_full_liq_hf: new_params.full_liq_hf,
            old_liq_protocol_fee: old_params.liq_protocol_fee,
            new_liq_protocol_fee: new_params.liq_protocol_fee,
            old_fee: old_params.fee,
            new_fee: new_params.fee,
            old_min_debt: old_params.min_debt,
            new_min_debt: new_params.min_debt,
            old_base_rate_ps: old_params.base_rate_ps,
            new_base_rate_ps: new_params.base_rate_ps,
            old_slope1_ps: old_params.slope1_ps,
            new_slope1_ps: new_params.slope1_ps,
            old_slope2_ps: old_params.slope2_ps,
            new_slope2_ps: new_params.slope2_ps,
            old_u_kink: old_params.u_kink,
            new_u_kink: new_params.u_kink,
            old_max_rate_ps: old_params.max_rate_ps,
            new_max_rate_ps: new_params.max_rate_ps,
        });
    }

    Ok(())
}
