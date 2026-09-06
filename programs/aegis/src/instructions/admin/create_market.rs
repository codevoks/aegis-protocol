//! `create_market` — creates one isolated lending market: its `Market` account, both custody
//! vaults, and the mandatory protocol fee `Position` (`instruction-catalogue.md` §6).
//!
//! This instruction implements the whole of Phase 2's security surface in one place: admin
//! authorization, mint/token-program pinning, the Token-2022 extension policy, freeze-authority
//! acknowledgement, and every parameter bound including the derived liquidation-safety
//! constraint. No token transfer happens here — only custody structure is proven.

use crate::constants::{
    COLLATERAL_VAULT_SEED, FLAG_ACK_FREEZE_AUTHORITY, FLAG_COLLATERAL_HAS_TRANSFER_FEE,
    LOAN_VAULT_SEED, MARKET_SEED, POSITION_SEED, PROTOCOL_SEED,
};
use crate::error::AegisError;
use crate::events::MarketCreated;
use crate::state::{Market, Position, Protocol};
use crate::token::policy::{evaluate_mint, MintRole};
use crate::token::vault::create_vault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMarketArgs {
    pub config_id: u16,

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

    pub ack_freeze_authority: bool,
}

#[derive(Accounts)]
#[instruction(args: CreateMarketArgs)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROTOCOL_SEED],
        bump = protocol.bump,
        has_one = admin @ AegisError::NotProtocolAdmin,
    )]
    pub protocol: Account<'info, Protocol>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,
    pub loan_mint: InterfaceAccount<'info, Mint>,

    pub collateral_token_program: Interface<'info, TokenInterface>,
    pub loan_token_program: Interface<'info, TokenInterface>,

    // Boxed: `Market` is 640 bytes and pushed `try_accounts` past the SBF stack-frame limit
    // (4096 bytes) when held inline alongside every other account in this instruction.
    #[account(
        init,
        payer = admin,
        space = Market::LEN,
        seeds = [
            MARKET_SEED,
            collateral_mint.key().as_ref(),
            loan_mint.key().as_ref(),
            &args.config_id.to_le_bytes(),
        ],
        bump,
    )]
    pub market: Box<Account<'info, Market>>,

    /// CHECK: created and sized by `token::vault::create_vault` in the handler, not by Anchor's
    /// `init` sugar — see that module's doc comment for why. The PDA-and-bump constraint here
    /// still guarantees this is the one, canonical collateral-vault address for this market.
    #[account(mut, seeds = [COLLATERAL_VAULT_SEED, market.key().as_ref()], bump)]
    pub collateral_vault: UncheckedAccount<'info>,

    /// CHECK: as `collateral_vault`, for the loan asset.
    #[account(mut, seeds = [LOAN_VAULT_SEED, market.key().as_ref()], bump)]
    pub loan_vault: UncheckedAccount<'info>,

    /// The protocol's own fee position, mandatory so `absorb_bad_debt` (Phase 6) can always
    /// require it — this removes an entire "what if it doesn't exist" branch from the most
    /// delicate instruction in the protocol (account-model.md §9).
    #[account(
        init,
        payer = admin,
        space = Position::LEN,
        seeds = [POSITION_SEED, market.key().as_ref(), protocol.fee_recipient.as_ref()],
        bump,
    )]
    pub fee_position: Account<'info, Position>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateMarket>, args: CreateMarketArgs) -> Result<()> {
    // A same-asset market is degenerate: LTV is meaningless and liquidation is a no-op
    // (instruction-catalogue.md §6 precondition 2).
    require_keys_neq!(
        ctx.accounts.collateral_mint.key(),
        ctx.accounts.loan_mint.key(),
        AegisError::SameCollateralAndLoanMint
    );

    // T-11: `InterfaceAccount<Mint>` only proves the mint's owner is *one of* SPL Token /
    // Token-2022; pin it to the *specific* program passed for this asset.
    let collateral_mint_ai = ctx.accounts.collateral_mint.to_account_info();
    let loan_mint_ai = ctx.accounts.loan_mint.to_account_info();
    require_keys_eq!(
        *collateral_mint_ai.owner,
        ctx.accounts.collateral_token_program.key(),
        AegisError::TokenProgramMintMismatch
    );
    require_keys_eq!(
        *loan_mint_ai.owner,
        ctx.accounts.loan_token_program.key(),
        AegisError::TokenProgramMintMismatch
    );

    // Positive extension allowlist, per role (token-compatibility.md §2, §4, §6).
    let collateral_outcome = evaluate_mint(&collateral_mint_ai, MintRole::Collateral)?;
    let loan_outcome = evaluate_mint(&loan_mint_ai, MintRole::Loan)?;

    let any_freeze_authority =
        collateral_outcome.has_freeze_authority || loan_outcome.has_freeze_authority;
    if any_freeze_authority {
        require!(
            args.ack_freeze_authority,
            AegisError::FreezeAuthorityNotAcknowledged
        );
    }

    // Full parameter-bound validation, including the derived liquidation-safety constraint
    // (economic-model.md §5, INV-LIQ-06).
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

    // Custody: create both vaults, Market PDA as authority, Token-2022 length never hardcoded.
    let market_key = ctx.accounts.market.key();
    let market_account_info = ctx.accounts.market.to_account_info();
    let collateral_vault_bump = ctx.bumps.collateral_vault;
    let loan_vault_bump = ctx.bumps.loan_vault;

    create_vault(
        &ctx.accounts.collateral_vault.to_account_info(),
        &collateral_mint_ai,
        &market_account_info,
        &ctx.accounts.admin.to_account_info(),
        &ctx.accounts.collateral_token_program.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &collateral_outcome.extension_types,
        &[
            COLLATERAL_VAULT_SEED,
            market_key.as_ref(),
            &[collateral_vault_bump],
        ],
    )?;

    create_vault(
        &ctx.accounts.loan_vault.to_account_info(),
        &loan_mint_ai,
        &market_account_info,
        &ctx.accounts.admin.to_account_info(),
        &ctx.accounts.loan_token_program.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &loan_outcome.extension_types,
        &[LOAN_VAULT_SEED, market_key.as_ref(), &[loan_vault_bump]],
    )?;

    let mut flags = 0u8;
    if any_freeze_authority {
        flags |= FLAG_ACK_FREEZE_AUTHORITY;
    }
    if collateral_outcome.has_transfer_fee {
        flags |= FLAG_COLLATERAL_HAS_TRANSFER_FEE;
    }

    let fee_recipient = ctx.accounts.protocol.fee_recipient;
    let now = Clock::get()?.unix_timestamp;
    let collateral_decimals = ctx.accounts.collateral_mint.decimals;
    let loan_decimals = ctx.accounts.loan_mint.decimals;

    let market = &mut ctx.accounts.market;
    market.collateral_mint = ctx.accounts.collateral_mint.key();
    market.loan_mint = ctx.accounts.loan_mint.key();
    market.collateral_token_program = ctx.accounts.collateral_token_program.key();
    market.loan_token_program = ctx.accounts.loan_token_program.key();
    market.collateral_vault = ctx.accounts.collateral_vault.key();
    market.loan_vault = ctx.accounts.loan_vault.key();
    market.fee_recipient = fee_recipient;
    market.config_id = args.config_id;
    market.collateral_decimals = collateral_decimals;
    market.loan_decimals = loan_decimals;

    market.oracle_kind = args.oracle_kind;
    market.collateral_feed_id = args.collateral_feed_id;
    market.loan_feed_id = args.loan_feed_id;
    market.max_price_age_secs = args.max_price_age_secs;
    market.max_conf_bps = args.max_conf_bps;

    market.max_ltv = args.max_ltv;
    market.liq_threshold = args.liq_threshold;
    market.liq_bonus = args.liq_bonus;
    market.close_factor = args.close_factor;
    market.full_liq_hf = args.full_liq_hf;
    market.liq_protocol_fee = args.liq_protocol_fee;
    market.fee = args.fee;
    market.min_debt = args.min_debt;

    market.base_rate_ps = args.base_rate_ps;
    market.slope1_ps = args.slope1_ps;
    market.slope2_ps = args.slope2_ps;
    market.u_kink = args.u_kink;
    market.max_rate_ps = args.max_rate_ps;

    market.total_supply_assets = 0;
    market.total_supply_shares = 0;
    market.total_borrow_assets = 0;
    market.total_borrow_shares = 0;
    market.collateral_fee_accrued = 0;
    market.last_accrual_ts = now;

    market.paused = 0;
    market.flags = flags;
    market.bump = ctx.bumps.market;
    market.collateral_vault_bump = collateral_vault_bump;
    market.loan_vault_bump = loan_vault_bump;
    market._reserved = [0u8; 64];

    let fee_position = &mut ctx.accounts.fee_position;
    fee_position.market = market_key;
    fee_position.owner = fee_recipient;
    fee_position.supply_shares = 0;
    fee_position.borrow_shares = 0;
    fee_position.collateral_amount = 0;
    fee_position.bump = ctx.bumps.fee_position;
    fee_position._reserved = [0u8; 32];

    emit!(MarketCreated {
        market: market_key,
        collateral_mint: market.collateral_mint,
        loan_mint: market.loan_mint,
        collateral_token_program: market.collateral_token_program,
        loan_token_program: market.loan_token_program,
        collateral_vault: market.collateral_vault,
        loan_vault: market.loan_vault,
        fee_recipient,
        fee_position: fee_position.key(),
        config_id: market.config_id,
        collateral_decimals,
        loan_decimals,
        oracle_kind: market.oracle_kind,
        collateral_feed_id: market.collateral_feed_id,
        loan_feed_id: market.loan_feed_id,
        max_price_age_secs: market.max_price_age_secs,
        max_conf_bps: market.max_conf_bps,
        max_ltv: market.max_ltv,
        liq_threshold: market.liq_threshold,
        liq_bonus: market.liq_bonus,
        close_factor: market.close_factor,
        full_liq_hf: market.full_liq_hf,
        liq_protocol_fee: market.liq_protocol_fee,
        fee: market.fee,
        min_debt: market.min_debt,
        base_rate_ps: market.base_rate_ps,
        slope1_ps: market.slope1_ps,
        slope2_ps: market.slope2_ps,
        u_kink: market.u_kink,
        max_rate_ps: market.max_rate_ps,
        flags,
        collateral_extensions: collateral_outcome
            .extension_types
            .iter()
            .map(|e| u16::from(*e))
            .collect(),
        loan_extensions: loan_outcome
            .extension_types
            .iter()
            .map(|e| u16::from(*e))
            .collect(),
    });

    Ok(())
}
