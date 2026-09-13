//! Specific, named error codes for the vault primitive (AGENTS.md §3: "Specific errors, never
//! generic ones"). Anchor gets this almost for free from `#[error_code]`; here it is one small
//! hand-written enum converted into Pinocchio's `ProgramError::Custom(u32)`.

use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultError {
    /// `amount` must be greater than zero (deposit/withdraw).
    ZeroAmount = 0,
    /// A token account's pinned mint (first 32 bytes of its raw SPL layout) does not match
    /// `VaultAuthority.mint`.
    MintMismatch = 1,
    /// The withdraw signer does not match `VaultAuthority.owner`.
    OwnerMismatch = 2,
    /// The `token_program` account is not the real classic SPL Token program (this lab is
    /// classic-SPL-only, matching `labs/vault-anchor`'s scope; Token-2022 is deliberately not
    /// accepted even though `pinocchio_token::TokenProgram::verify` would allow it).
    InvalidTokenProgram = 3,
    /// The `system_program` account is not the real System program.
    InvalidSystemProgram = 4,
    /// The passed `vault_authority` account does not match the canonical PDA for
    /// `[VAULT_AUTHORITY_SEED, mint]` (init: `find_program_address`; deposit/withdraw:
    /// `create_program_address` from the stored bump).
    InvalidVaultAuthority = 5,
    /// The passed `vault_token_account` does not match the canonical PDA for
    /// `[VAULT_TOKEN_SEED, mint]`, or does not match `VaultAuthority.vault_token_account`.
    InvalidVaultTokenAccount = 6,
    /// An account required to be owned by a specific program (this program, or the token
    /// program) is not.
    InvalidAccountOwner = 7,
    /// An account's raw data is the wrong length for what it is being read as.
    InvalidAccountData = 8,
    /// Instruction data is missing the tag byte, or the amount's 8 little-endian bytes.
    InvalidInstructionData = 9,
    /// A required signer did not sign.
    MissingRequiredSignature = 10,
    /// The instruction did not receive as many accounts as it requires.
    NotEnoughAccountKeys = 11,
    /// The `program_id` the runtime invoked does not match this program's own `ID`.
    IncorrectProgramId = 12,
}

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
