//! Aegis Protocol on-chain program.
//!
//! Phase 1 scope only: this program exists to prove the toolchain, not the protocol. It
//! contains exactly one no-op instruction (`ping`) so that `anchor build`, IDL generation,
//! and a LiteSVM deploy-and-invoke round trip can all be demonstrated end to end. No
//! account state, no protocol instructions — those begin at Phase 2
//! (see `docs/phases/phase-01-foundation.md` §2).

use anchor_lang::prelude::*;

declare_id!("2GtoBADM175vkjf5UYpbD198Ry1cJadXMGo8sCQvXndh");

#[program]
pub mod aegis {
    use super::*;

    /// Does nothing and always succeeds. Proves the program builds, deploys, and is
    /// invocable — the entire Phase 1 acceptance bar for on-chain code.
    pub fn ping(_ctx: Context<Ping>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Ping {}
