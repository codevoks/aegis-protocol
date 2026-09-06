//! Aegis Protocol on-chain program.
//!
//! Phase 5 adds the real oracle: `oracle::require_valid_price` implements checks O-1..O-11
//! (`docs/oracle-design.md`) against the real `pyth-solana-receiver-sdk` 2.0.0. The Phase 3/4
//! hard gates are removed — `borrow` now borrows for real, oracle-validated and LTV-checked
//! (`instruction-catalogue.md` §14), and `withdraw_collateral` now validates post-withdrawal
//! health whenever `position.borrow_shares > 0` (§11), while a debt-free withdrawal still reads
//! no oracle at all (E-08). Still no liquidation or bad debt — see
//! `docs/phases/phase-05-oracle.md`.
//!
//! `lib.rs` contains no logic beyond wiring instruction entry points to their handlers
//! (architecture.md §2): validate accounts → read state → call math → write state → move tokens
//! → emit event, all of which lives in `instructions/`, `state/`, `token/` and `aegis-math`.
//!
//! `ping` is unchanged from Phase 1: a no-op that proves the toolchain builds, deploys, and is
//! invocable (`tests/smoke.rs`, `docs/phases/phase-01-foundation.md`), kept so that Phase 1's own
//! evidence keeps passing.

use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod events;
pub mod guards;
pub mod instructions;
pub mod oracle;
pub mod state;
pub mod token;

// Anchor's `#[program]` macro expands to code that references each Accounts struct's
// macro-generated `__client_accounts_*` / `__cpi_client_accounts_*` sibling modules via bare
// `crate::` paths, so they must be visible at the crate root — hence the glob imports straight
// from each instruction's own module (not merely the named re-exports in `instructions::admin`,
// which exist for the public API surface). These are private, unqualified `use` statements, not
// `pub use`, so the `handler` name every instruction file defines never becomes an ambiguous
// public re-export; it is only ever called through a fully qualified path below.
use instructions::admin::create_market::*;
use instructions::admin::initialize_protocol::*;
use instructions::borrow::accrue::*;
use instructions::borrow::borrow::*;
use instructions::borrow::repay::*;
use instructions::collateral::deposit_collateral::*;
use instructions::collateral::withdraw_collateral::*;
use instructions::lend::supply::*;
use instructions::lend::withdraw::*;
use instructions::position::close_position::*;
use instructions::position::init_position::*;

declare_id!("2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh");

#[program]
pub mod aegis {
    use super::*;

    /// Does nothing and always succeeds. Phase 1's toolchain proof; unchanged in Phase 2.
    pub fn ping(_ctx: Context<Ping>) -> Result<()> {
        Ok(())
    }

    /// Creates the singleton `Protocol` account (`instruction-catalogue.md` §1).
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        args: InitProtocolArgs,
    ) -> Result<()> {
        instructions::admin::initialize_protocol::handler(ctx, args)
    }

    /// Creates one isolated lending market, its two custody vaults, and the mandatory protocol
    /// fee position (`instruction-catalogue.md` §6).
    pub fn create_market(ctx: Context<CreateMarket>, args: CreateMarketArgs) -> Result<()> {
        instructions::admin::create_market::handler(ctx, args)
    }

    /// Creates an empty `Position` for `(market, owner)` (`instruction-catalogue.md` §9).
    pub fn init_position(ctx: Context<InitPosition>) -> Result<()> {
        instructions::position::init_position::handler(ctx)
    }

    /// Deposits collateral into a position by measured delta; no oracle, no pause check, and
    /// `Market` is never written (`instruction-catalogue.md` §10).
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::collateral::deposit_collateral::handler(ctx, amount)
    }

    /// Withdraws collateral. A debt-free position reads no oracle at all (E-08); a position with
    /// outstanding `borrow_shares` validates the oracle and requires post-withdrawal
    /// `debt_value <= collateral_value * max_ltv / WAD` (`instruction-catalogue.md` §11).
    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::collateral::withdraw_collateral::handler(ctx, amount)
    }

    /// Closes an empty `Position` and returns its rent to `owner` (`instruction-catalogue.md` §20).
    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        instructions::position::close_position::handler(ctx)
    }

    /// Deposits the loan asset, crediting the lender's own position with supply shares
    /// (`instruction-catalogue.md` §12). Exactly one of `assets`/`shares` must be nonzero.
    pub fn supply(ctx: Context<Supply>, assets: u64, shares: u128) -> Result<()> {
        instructions::lend::supply::handler(ctx, assets, shares)
    }

    /// Redeems the lender's own supply shares, bounded by free liquidity
    /// (`instruction-catalogue.md` §13).
    pub fn withdraw(ctx: Context<Withdraw>, assets: u64, shares: u128) -> Result<()> {
        instructions::lend::withdraw::handler(ctx, assets, shares)
    }

    /// Borrows the loan asset against posted collateral, oracle-validated and LTV-checked
    /// (`instruction-catalogue.md` §14, `docs/oracle-design.md`). Fails closed on any oracle
    /// validation failure.
    pub fn borrow(ctx: Context<Borrow>, assets: u64, shares: u128) -> Result<()> {
        instructions::borrow::borrow::handler(ctx, assets, shares)
    }

    /// Repays debt on any position. No owner signature, no oracle, unpausable
    /// (`instruction-catalogue.md` §15, INV-ADM-04). Clamped to the position's actual debt.
    pub fn repay(ctx: Context<Repay>, assets: u64, shares: u128) -> Result<()> {
        instructions::borrow::repay::handler(ctx, assets, shares)
    }

    /// Standalone, permissionless interest accrual (`instruction-catalogue.md` §16). `dt == 0` is
    /// a successful no-op.
    pub fn accrue_interest(ctx: Context<AccrueInterest>) -> Result<()> {
        instructions::borrow::accrue::handler(ctx)
    }
}

#[derive(Accounts)]
pub struct Ping {}
