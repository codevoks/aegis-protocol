//! Aegis Protocol on-chain program.
//!
//! Phase 3 scope adds real collateral custody flows on top of Phase 2's state and vaults:
//! `deposit_collateral` (no oracle, no pause, `Market` read-only, measured-delta accounting),
//! `withdraw_collateral` (zero-debt path only — any position with `borrow_shares > 0` is refused
//! with `OracleNotYetAvailable`, a hard sequencing gate rather than a bypass), and
//! `close_position`. Still no supply, borrow, repay, interest, oracle, or liquidation — see
//! `docs/phases/phase-03-collateral.md`.
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
use instructions::collateral::deposit_collateral::*;
use instructions::collateral::withdraw_collateral::*;
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

    /// Withdraws collateral for the zero-debt path only; a position with outstanding
    /// `borrow_shares` is refused with `OracleNotYetAvailable` (`instruction-catalogue.md` §11,
    /// `docs/phase-roadmap.md` "Sequencing the oracle dependency").
    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::collateral::withdraw_collateral::handler(ctx, amount)
    }

    /// Closes an empty `Position` and returns its rent to `owner` (`instruction-catalogue.md` §20).
    pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
        instructions::position::close_position::handler(ctx)
    }
}

#[derive(Accounts)]
pub struct Ping {}
