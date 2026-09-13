//! `Protocol` — singleton configuration and the root of administrative authority
//! (`account-model.md` §3). Read-only in every user instruction; only admin `set_*` instructions
//! (Phase 12) ever write it after `initialize_protocol` creates it.

use anchor_lang::prelude::*;

/// Phase 12 (`INV-UPG-01..03`, ADR-0014): the current live schema. `schema_version` is the
/// explicit version marker required by the migration design (distinct from, and in addition to,
/// the Anchor discriminator that already distinguishes this type from [`ProtocolV1`] at the byte
/// level) — `1` for every account created by `migrate_protocol_v2` or by `initialize_protocol`
/// from this point forward.
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
    /// Explicit schema-version marker (Phase 12, `constants::PROTOCOL_SCHEMA_VERSION`). One byte
    /// carved out of what was previously all of `_reserved` — additive, no realloc (governance.md
    /// §6 rule 1); `Protocol::LEN` is unchanged from the pre-Phase-12 202 bytes.
    pub schema_version: u8,
    /// Forward-compatibility space for additive fields without a realloc (INV-RES-05). Always
    /// written as all-zero; never read.
    pub _reserved: [u8; 63],
}

impl Protocol {
    /// `8` (Anchor discriminator) plus `32*4` (four `Pubkey` fields), `1` (`paused`), `1`
    /// (`bump`), `1` (`schema_version`) and `63` (`_reserved`) totals `202`, matching
    /// `account-model.md` §3 exactly — ADR-0014 shrank `_reserved` by one byte in the same breath
    /// it added `schema_version`, so this total is unchanged from before Phase 12.
    pub const LEN: usize = 8 + (32 * 4) + 1 + 1 + 1 + 63;
}

/// Phase 12 (`INV-UPG-01..03`, ADR-0014): the pre-Phase-12 `Protocol` layout, byte-for-byte, kept
/// solely so `migrate_protocol_v2` can deserialize an account created before `schema_version`
/// existed. **Never constructed by any live instruction** — no `init` anywhere targets this type;
/// it exists only as the `From` half of `Migration<'info, ProtocolV1, Protocol>`. Anchor's
/// `#[account]` macro derives a distinct discriminator for this type name, which is what makes an
/// already-migrated account (now serialized as `Protocol`) unreadable as `ProtocolV1` — the
/// mechanism `I-UPG-02` (migration idempotence) actually relies on.
#[account]
pub struct ProtocolV1 {
    pub admin: Pubkey,
    pub pending_admin: Pubkey,
    pub guardian: Pubkey,
    pub fee_recipient: Pubkey,
    pub paused: u8,
    pub bump: u8,
    pub _reserved: [u8; 64],
}

impl ProtocolV1 {
    pub const LEN: usize = 8 + (32 * 4) + 1 + 1 + 64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_matches_account_model_spec() {
        assert_eq!(Protocol::LEN, 202);
    }

    // ADR-0014: the migration must be additive (no realloc) -- the whole point of carving
    // schema_version out of _reserved rather than growing the account.
    #[test]
    fn v1_and_v2_have_the_same_on_chain_size() {
        assert_eq!(ProtocolV1::LEN, Protocol::LEN);
    }
}
