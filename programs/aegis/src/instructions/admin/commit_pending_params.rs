//! `commit_pending_params()` — applies a previously-staged risk-increasing parameter change once
//! its timelock has elapsed (`instruction-catalogue.md` §7, `governance.md` §4, INV-ADM-09).
//! Permissionless (any funded signer may trigger it, like `accrue_interest`/`absorb_bad_debt`):
//! by the time the timelock has elapsed there is no remaining admin discretion to exercise, only
//! applying exactly what was already staged and publicly observable via `ParamsStaged`.
//!
//! Re-runs the full canonical bounds validation against the staged values (never assumes a
//! proposal valid at staging time is still valid at commit time) and calls `accrue_mut` under the
//! still-active OLD parameters before applying the new ones, exactly mirroring
//! `set_market_params`'s own ordering.

use crate::constants::{MARKET_SEED, PENDING_PARAMS_SEED, POSITION_SEED, PROTOCOL_SEED};
use crate::error::AegisError;
use crate::events::StagedParamsCommitted;
use crate::state::{Market, MutableMarketParams, PendingMarketParams, Position, Protocol};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CommitPendingParams<'info> {
    /// Permissionless caller; pays only the rent refund's destination is fixed below, not this
    /// signer.
    pub payer: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
    )]
    pub protocol: Account<'info, Protocol>,

    /// The rent for `pending_market_params` was paid by the admin who staged it
    /// (`set_market_params`); it is refunded to the CURRENT admin here, never to `payer`, so a
    /// permissionless caller cannot redirect rent to themselves.
    ///
    /// CHECK: address-pinned to `protocol.admin`; never read beyond receiving the `close` refund.
    #[account(mut, address = protocol.admin @ AegisError::NotProtocolAdmin)]
    pub admin: UncheckedAccount<'info>,

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
        seeds = [POSITION_SEED, market.key().as_ref(), market.fee_recipient.as_ref()],
        bump = fee_position.bump,
        has_one = market @ AegisError::PositionMarketMismatch,
    )]
    pub fee_position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [PENDING_PARAMS_SEED, market.key().as_ref()],
        bump = pending_market_params.bump,
        has_one = market @ AegisError::PendingParamsMarketMismatch,
        close = admin,
    )]
    pub pending_market_params: Account<'info, PendingMarketParams>,
}

pub fn handler(ctx: Context<CommitPendingParams>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= ctx.accounts.pending_market_params.effective_at,
        AegisError::PendingParamsNotYetEffective
    );

    let pending = &ctx.accounts.pending_market_params;

    // Re-validate: a proposal valid when staged is not assumed valid forever.
    Market::validate_risk_params(
        pending.max_ltv,
        pending.liq_threshold,
        pending.liq_bonus,
        pending.close_factor,
        pending.full_liq_hf,
        pending.liq_protocol_fee,
        pending.fee,
        pending.min_debt,
    )?;
    Market::validate_irm_params(
        pending.base_rate_ps,
        pending.slope1_ps,
        pending.slope2_ps,
        pending.u_kink,
        pending.max_rate_ps,
    )?;
    Market::validate_oracle_config(pending.max_price_age_secs, pending.max_conf_bps)?;

    let new_params = MutableMarketParams {
        oracle_kind: pending.oracle_kind,
        collateral_feed_id: pending.collateral_feed_id,
        loan_feed_id: pending.loan_feed_id,
        max_price_age_secs: pending.max_price_age_secs,
        max_conf_bps: pending.max_conf_bps,
        max_ltv: pending.max_ltv,
        liq_threshold: pending.liq_threshold,
        liq_bonus: pending.liq_bonus,
        close_factor: pending.close_factor,
        full_liq_hf: pending.full_liq_hf,
        liq_protocol_fee: pending.liq_protocol_fee,
        fee: pending.fee,
        min_debt: pending.min_debt,
        base_rate_ps: pending.base_rate_ps,
        slope1_ps: pending.slope1_ps,
        slope2_ps: pending.slope2_ps,
        u_kink: pending.u_kink,
        max_rate_ps: pending.max_rate_ps,
    };
    let effective_at = pending.effective_at;

    // INV-ADM-07 (regression, delayed path): settle accrued interest under the still-active OLD
    // parameters before the new ones take effect.
    ctx.accounts
        .market
        .accrue_mut(&mut ctx.accounts.fee_position, now)?;

    new_params.apply_to(&mut ctx.accounts.market);

    emit!(StagedParamsCommitted {
        market: ctx.accounts.market.key(),
        pending_market_params: ctx.accounts.pending_market_params.key(),
        effective_at,
    });

    // `close = admin` (declarative, above) zeroes the discriminator and refunds rent to `admin`
    // once the handler returns -- clearing the way for a future `set_market_params` loosening
    // proposal on this market.
    Ok(())
}
