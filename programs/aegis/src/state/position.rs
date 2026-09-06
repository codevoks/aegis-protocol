//! `Position` — one user's entire relationship with one market (`account-model.md` §5).

use anchor_lang::prelude::*;

#[account]
pub struct Position {
    /// `has_one` target back to the owning `Market`.
    pub market: Pubkey,
    pub owner: Pubkey,
    pub supply_shares: u128,
    pub borrow_shares: u128,
    pub collateral_amount: u64,
    /// Canonical bump for `PDA([b"position", market, owner])`.
    pub bump: u8,
    /// Forward-compatibility space. Always written as all-zero; never read.
    pub _reserved: [u8; 32],
}

impl Position {
    /// `8` (discriminator) + `32*2` (`market`, `owner`) + `16*2` (`supply_shares`,
    /// `borrow_shares`) + `8` (`collateral_amount`) + `1` (`bump`) + `32` (`_reserved`) = `145`,
    /// matching `account-model.md` §5 exactly.
    pub const LEN: usize = 8 + (32 * 2) + (16 * 2) + 8 + 1 + 32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_matches_account_model_spec() {
        assert_eq!(Position::LEN, 145);
    }
}
