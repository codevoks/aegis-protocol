//! Small, shared preconditions reused across instruction handlers (architecture.md §2). Pause
//! guards and relation helpers grow here as later phases add pausable instructions; Phase 2 has
//! none (`initialize_protocol`, `create_market` and `init_position` are all unpausable), so this
//! module currently holds only the one check both admin-facing args and `init_position` need.

use crate::error::AegisError;
use anchor_lang::prelude::*;

/// Rejects the default `Pubkey` — used wherever an argument or account must name a real key
/// (`instruction-catalogue.md` #1's `guardian`/`fee_recipient` preconditions, #9's `owner`).
pub fn require_non_default_pubkey(key: Pubkey, err: AegisError) -> Result<()> {
    require_keys_neq!(key, Pubkey::default(), err);
    Ok(())
}
