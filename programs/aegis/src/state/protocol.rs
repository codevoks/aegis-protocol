//! `Protocol` — singleton configuration and the root of administrative authority
//! (`account-model.md` §3). Read-only in every user instruction; only admin `set_*` instructions
//! (Phase 12) ever write it after `initialize_protocol` creates it.

use anchor_lang::prelude::*;

#[account]
pub struct Protocol {
    /// Full authority. Set once at `initialize_protocol` to the deployer.
    pub admin: Pubkey,
    /// Two-step admin transfer target (Phase 12). `Pubkey::default()` means "none pending".
    pub pending_admin: Pubkey,
    /// Pause-only authority — may set pause bits but never clear them (INV-AUTH-04).
    pub guardian: Pubkey,
    /// Default fee recipient snapshotted into every new `Market` at creation.
    pub fee_recipient: Pubkey,
    /// Global pause bitflags (`constants::PAUSE_*`).
    pub paused: u8,
    /// Canonical bump for `PDA([b"protocol"])`.
    pub bump: u8,
    /// Forward-compatibility space for additive fields without a realloc (INV-RES-05). Always
    /// written as all-zero; never read.
    pub _reserved: [u8; 64],
}

impl Protocol {
    /// `8` (Anchor discriminator) + `32*4` (four `Pubkey` fields) + `1` (`paused`) + `1` (`bump`)
    /// + `64` (`_reserved`) = `202`, matching `account-model.md` §3 exactly.
    pub const LEN: usize = 8 + (32 * 4) + 1 + 1 + 64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_matches_account_model_spec() {
        assert_eq!(Protocol::LEN, 202);
    }
}
