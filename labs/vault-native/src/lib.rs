//! Phase 11 lab (`docs/adr/0003-native-pinocchio-as-labs.md`, `docs/phases/phase-11-performance.md`
//! #23/25): the SAME custody primitive `labs/vault-anchor` implements -- initialize a vault PDA,
//! deposit into it, withdraw from it via `invoke_signed` -- written against plain `solana-program`
//! with **no Anchor**: manual account parsing, manual validation, and manual serialization, so
//! `labs/cu-bench` can measure what Anchor's declarative constraints cost in compute units.
//!
//! **Scope, deliberately bounded** (ADR-0003), identical to `labs/vault-anchor`: classic SPL Token
//! only, one mint, one owner per vault. This is not a re-implementation of Aegis's
//! Market/Position/oracle/liquidation logic -- it is exactly the custody shape (a state PDA
//! authorizing a token-vault PDA) and nothing else.
//!
//! ## Manual equivalent of Anchor's automatic checks
//!
//! Anchor's `#[account]` derives an 8-byte discriminator and its `Account<'info, T>` wrapper checks
//! it plus account ownership on every deserialization. There is no such mechanism here, so
//! [`VaultAuthority::load`] performs the manual equivalent by hand: **owner == this program**, and
//! **data length == the exact expected size**, before a single byte of the buffer is trusted. This
//! is a real security control, not a stylistic difference -- skipping it would let a
//! wrong-sized or wrong-owner account be parsed as a `VaultAuthority` and could type-confuse this
//! program into acting on attacker-controlled bytes.
//!
//! ## Security checks (equivalent across all three labs, `phase-11-performance.md` #27)
//! - **Signer**: every signer requirement is checked explicitly via `AccountInfo::is_signer`
//!   (`require_signer`), never assumed from account order.
//! - **Owning program**: every account's owner is checked explicitly before its bytes are trusted --
//!   `vault_authority` against `program_id`, `vault_token_account`/`mint`/token accounts against the
//!   passed-in `token_program`'s key, and `token_program`/`system_program` against the real program
//!   IDs (`require_owned_by`, `require_key_eq`).
//! - **PDA**: `vault_authority` is derived with `find_program_address` only in `Initialize` (the one
//!   place that is acceptable, to discover the canonical bump); `Deposit` and `Withdraw` re-derive it
//!   with `create_program_address` from the **stored** bump and compare it to the passed-in account
//!   key, per `docs/account-model.md` §7's "use stored bumps, never re-search."
//! - **Mint pinned**: `vault_authority.mint` is compared against the `mint` account passed in, and
//!   against the raw mint field (byte offset `0..32`) read directly out of both token accounts'
//!   account data, before either token account is trusted at all.
//! - **Vault address pinned**: `vault_authority.vault_token_account`, as recorded in the PDA's own
//!   state, must equal the `vault_token_account` actually passed in the instruction -- the same
//!   "checked twice" defense-in-depth `docs/account-model.md` §1 describes for the main Aegis
//!   program (PDA derivation *and* a stored-pubkey comparison).
//! - **Authority**: only `vault_authority`'s own PDA signature (`invoke_signed` with the stored
//!   bump) can move tokens out of `vault_token_account`; `Withdraw` additionally requires the
//!   transaction signer to equal `vault_authority.owner` exactly.
//! - **Amount semantics**: `amount > 0` is required on `Deposit` and `Withdraw`; the transferred
//!   amount is exactly what the caller requested, via `TransferChecked` (validates mint and
//!   decimals), never plain `Transfer`.
//! - **Arithmetic**: this primitive has no arithmetic of its own (`amount` passes straight through
//!   to the CPI unchanged), but every place a length or offset is used it is a fixed compile-time
//!   constant, never a runtime computation that could wrap -- consistent with AGENTS.md's
//!   checked-arithmetic rule even though there is nothing here for `checked_add`/`checked_sub` to
//!   guard.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
// solana-program 4.x no longer bundles `system_instruction`/`system_program` (verified against
// its own crate docs) -- the System Program's ID and instruction builders now live here.
use solana_system_interface::{instruction as system_instruction, program as system_program};

solana_program::declare_id!("5fFhXv6YaFfJM45tL1RQq6ie7FprxqtdD3nzQwZsKLF3");

pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const VAULT_TOKEN_SEED: &[u8] = b"vault_token";

/// `spl_token_interface::state::Account::LEN` (mint 32 + owner 32 + amount 8 + delegate COption<Pubkey> 36
/// + state 1 + is_native COption<u64> 12 + delegated_amount 8 + close_authority COption<Pubkey> 36).
pub const SPL_TOKEN_ACCOUNT_LEN: usize = 165;
/// Byte offset of the `mint` field within an SPL Token `Account`'s raw data.
const SPL_TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
/// Byte offset of the `decimals` field within an SPL Token `Mint`'s raw data
/// (mint_authority COption<Pubkey> 36 + supply 8 + decimals 1).
const SPL_MINT_DECIMALS_OFFSET: usize = 44;
/// `spl_token_interface::state::Mint::LEN`.
const SPL_MINT_LEN: usize = 82;

/// Instruction tags, byte 0 of instruction data. Not an Anchor 8-byte sighash discriminator --
/// this is a from-scratch native program.
const TAG_INITIALIZE: u8 = 0;
const TAG_DEPOSIT: u8 = 1;
const TAG_WITHDRAW: u8 = 2;

/// A handful of named, specific errors (AGENTS.md §3: "specific errors, never generic ones").
/// Not exhaustive -- scoped to this lab's primitive.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultNativeError {
    /// Instruction data was empty or too short for its tag.
    InvalidInstructionData = 0,
    /// The first byte of instruction data did not match any known instruction.
    UnknownInstructionTag = 1,
    /// A required signer did not sign.
    NotSigner = 2,
    /// An account's owning program did not match what was expected.
    WrongOwner = 3,
    /// The account passed as `token_program` is not the real SPL Token program.
    WrongTokenProgram = 4,
    /// The account passed as `system_program` is not the real System program.
    WrongSystemProgram = 5,
    /// A PDA did not match its expected derived address.
    InvalidPda = 6,
    /// The passed-in vault token account does not match the address recorded in `VaultAuthority`.
    VaultAccountMismatch = 7,
    /// A token account's (or the vault's recorded) mint did not match the pinned mint.
    MintMismatch = 8,
    /// `amount` was zero.
    ZeroAmount = 9,
    /// `vault_authority` has not been initialized (wrong data length).
    NotInitialized = 10,
    /// `vault_authority` (or `vault_token_account`) already exists.
    AlreadyInitialized = 11,
    /// The signer did not match `vault_authority.owner`.
    OwnerMismatch = 12,
    /// A token account's raw data was too short to be a real SPL Token account.
    InvalidTokenAccountData = 13,
}

impl From<VaultNativeError> for ProgramError {
    fn from(e: VaultNativeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, rest) = instruction_data
        .split_first()
        .ok_or(VaultNativeError::InvalidInstructionData)?;
    match *tag {
        TAG_INITIALIZE => process_initialize(program_id, accounts),
        TAG_DEPOSIT => process_deposit(program_id, accounts, parse_amount(rest)?),
        TAG_WITHDRAW => process_withdraw(program_id, accounts, parse_amount(rest)?),
        _ => Err(VaultNativeError::UnknownInstructionTag.into()),
    }
}

fn parse_amount(data: &[u8]) -> Result<u64, ProgramError> {
    let bytes: [u8; 8] = data
        .get(0..8)
        .ok_or(VaultNativeError::InvalidInstructionData)?
        .try_into()
        .map_err(|_| VaultNativeError::InvalidInstructionData)?;
    Ok(u64::from_le_bytes(bytes))
}

/// Manual byte-level state for the vault authority PDA: `owner (32) | mint (32) |
/// vault_token_account (32) | bump (1)` = 97 bytes, field order matching `labs/vault-anchor`'s
/// `VaultAuthority` exactly (shape, not byte-for-byte discriminator scheme -- this program has none).
pub struct VaultAuthority {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub bump: u8,
}

impl VaultAuthority {
    pub const LEN: usize = 32 + 32 + 32 + 1;

    fn write_into(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        if dst.len() != Self::LEN {
            return Err(VaultNativeError::InvalidInstructionData.into());
        }
        dst[0..32].copy_from_slice(self.owner.as_ref());
        dst[32..64].copy_from_slice(self.mint.as_ref());
        dst[64..96].copy_from_slice(self.vault_token_account.as_ref());
        dst[96] = self.bump;
        Ok(())
    }

    /// Parses the raw byte layout only -- no ownership check. Public so off-chain callers (this
    /// crate's own integration tests included) can read a fetched account's data without linking
    /// against `AccountInfo`. On-chain code must go through [`VaultAuthority::load`] instead, which
    /// adds the owner check this function deliberately does not do.
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(VaultNativeError::NotInitialized.into());
        }
        let owner = Pubkey::new_from_array(data[0..32].try_into().unwrap());
        let mint = Pubkey::new_from_array(data[32..64].try_into().unwrap());
        let vault_token_account = Pubkey::new_from_array(data[64..96].try_into().unwrap());
        let bump = data[96];
        Ok(Self {
            owner,
            mint,
            vault_token_account,
            bump,
        })
    }

    /// The manual equivalent of Anchor's automatic discriminator + owner check: an account is only
    /// trusted as a `VaultAuthority` if it is owned by this program AND its data is exactly the
    /// expected size. Skipping either check would let an attacker point this program at
    /// wrong-owner or wrong-shape bytes.
    fn load(account: &AccountInfo, program_id: &Pubkey) -> Result<Self, ProgramError> {
        require_owned_by(account, program_id)?;
        let data = account.try_borrow_data()?;
        Self::unpack(&data)
    }
}

fn require_signer(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer {
        return Err(VaultNativeError::NotSigner.into());
    }
    Ok(())
}

fn require_owned_by(account: &AccountInfo, expected: &Pubkey) -> Result<(), ProgramError> {
    if account.owner != expected {
        return Err(VaultNativeError::WrongOwner.into());
    }
    Ok(())
}

fn require_key_eq(actual: &Pubkey, expected: &Pubkey, err: VaultNativeError) -> ProgramResult {
    if actual != expected {
        return Err(err.into());
    }
    Ok(())
}

/// Reads the raw `mint` field (offset 0..32) directly out of an SPL Token `Account`'s data,
/// after checking it is actually owned by the real token program and long enough to hold that
/// field -- the manual equivalent of Anchor's `Account<'info, TokenAccount>` deserialization.
fn read_token_account_mint(
    account: &AccountInfo,
    token_program: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    require_owned_by(account, token_program)?;
    let data = account.try_borrow_data()?;
    if data.len() < SPL_TOKEN_ACCOUNT_LEN {
        return Err(VaultNativeError::InvalidTokenAccountData.into());
    }
    let mint_bytes: [u8; 32] = data
        [SPL_TOKEN_ACCOUNT_MINT_OFFSET..SPL_TOKEN_ACCOUNT_MINT_OFFSET + 32]
        .try_into()
        .unwrap();
    Ok(Pubkey::new_from_array(mint_bytes))
}

/// Reads the raw `decimals` field directly out of an SPL Token `Mint`'s data, after checking
/// ownership and length, for use in `TransferChecked` (which must be given the real decimals, not
/// a caller-supplied value).
fn read_mint_decimals(mint: &AccountInfo, token_program: &Pubkey) -> Result<u8, ProgramError> {
    require_owned_by(mint, token_program)?;
    let data = mint.try_borrow_data()?;
    if data.len() < SPL_MINT_LEN {
        return Err(VaultNativeError::InvalidTokenAccountData.into());
    }
    Ok(data[SPL_MINT_DECIMALS_OFFSET])
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let payer = next_account_info(iter)?;
    let owner = next_account_info(iter)?;
    let mint = next_account_info(iter)?;
    let vault_authority = next_account_info(iter)?;
    let vault_token_account = next_account_info(iter)?;
    let token_program = next_account_info(iter)?;
    let system_program_account = next_account_info(iter)?;

    // Signer: only the payer needs to sign to create the vault (matching `labs/vault-anchor`'s
    // `InitializeVault`, which also does not require the owner's signature).
    require_signer(payer)?;

    // Owning program checks up front, before any address is derived from these accounts' keys.
    require_key_eq(
        token_program.key,
        &spl_token_interface::ID,
        VaultNativeError::WrongTokenProgram,
    )?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        VaultNativeError::WrongSystemProgram,
    )?;

    // `find_program_address` is acceptable here -- and only here -- to discover the canonical
    // bump the first time (`docs/account-model.md` §7).
    let (expected_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, mint.key.as_ref()], program_id);
    require_key_eq(
        vault_authority.key,
        &expected_vault_authority,
        VaultNativeError::InvalidPda,
    )?;
    let (expected_vault_token_account, vault_token_bump) =
        Pubkey::find_program_address(&[VAULT_TOKEN_SEED, mint.key.as_ref()], program_id);
    require_key_eq(
        vault_token_account.key,
        &expected_vault_token_account,
        VaultNativeError::InvalidPda,
    )?;

    // Neither PDA may already be in use -- the manual equivalent of Anchor's `init` constraint
    // refusing to reinitialize an existing account.
    if vault_authority.owner != &system_program::ID || !vault_authority.data_is_empty() {
        return Err(VaultNativeError::AlreadyInitialized.into());
    }
    if vault_token_account.owner != &system_program::ID || !vault_token_account.data_is_empty() {
        return Err(VaultNativeError::AlreadyInitialized.into());
    }

    let rent = Rent::get()?;

    let vault_authority_seeds: &[&[u8]] = &[
        VAULT_AUTHORITY_SEED,
        mint.key.as_ref(),
        &[vault_authority_bump],
    ];
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            vault_authority.key,
            rent.minimum_balance(VaultAuthority::LEN),
            VaultAuthority::LEN as u64,
            program_id,
        ),
        &[
            payer.clone(),
            vault_authority.clone(),
            system_program_account.clone(),
        ],
        &[vault_authority_seeds],
    )?;

    let vault_token_seeds: &[&[u8]] = &[VAULT_TOKEN_SEED, mint.key.as_ref(), &[vault_token_bump]];
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            vault_token_account.key,
            rent.minimum_balance(SPL_TOKEN_ACCOUNT_LEN),
            SPL_TOKEN_ACCOUNT_LEN as u64,
            token_program.key,
        ),
        &[
            payer.clone(),
            vault_token_account.clone(),
            system_program_account.clone(),
        ],
        &[vault_token_seeds],
    )?;

    // SPL Token's own `InitializeAccount3` sets the vault authority PDA -- not the vault token
    // account itself -- as the SPL-level token authority.
    invoke(
        &spl_token_interface::instruction::initialize_account3(
            token_program.key,
            vault_token_account.key,
            mint.key,
            vault_authority.key,
        )?,
        &[
            vault_token_account.clone(),
            mint.clone(),
            token_program.clone(),
        ],
    )?;

    let state = VaultAuthority {
        owner: *owner.key,
        mint: *mint.key,
        vault_token_account: *vault_token_account.key,
        bump: vault_authority_bump,
    };
    let mut data = vault_authority.try_borrow_mut_data()?;
    state.write_into(&mut data)?;

    Ok(())
}

fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let iter = &mut accounts.iter();
    let depositor = next_account_info(iter)?;
    let vault_authority = next_account_info(iter)?;
    let vault_token_account = next_account_info(iter)?;
    let depositor_token_account = next_account_info(iter)?;
    let mint = next_account_info(iter)?;
    let token_program = next_account_info(iter)?;

    require_signer(depositor)?;
    require_key_eq(
        token_program.key,
        &spl_token_interface::ID,
        VaultNativeError::WrongTokenProgram,
    )?;
    if amount == 0 {
        return Err(VaultNativeError::ZeroAmount.into());
    }

    let state = VaultAuthority::load(vault_authority, program_id)?;

    // Hot path: re-derive with the stored bump via `create_program_address`, never re-search
    // (`docs/account-model.md` §7).
    let expected_vault_authority = Pubkey::create_program_address(
        &[VAULT_AUTHORITY_SEED, state.mint.as_ref(), &[state.bump]],
        program_id,
    )
    .map_err(|_| VaultNativeError::InvalidPda)?;
    require_key_eq(
        vault_authority.key,
        &expected_vault_authority,
        VaultNativeError::InvalidPda,
    )?;

    require_key_eq(mint.key, &state.mint, VaultNativeError::MintMismatch)?;
    require_key_eq(
        vault_token_account.key,
        &state.vault_token_account,
        VaultNativeError::VaultAccountMismatch,
    )?;

    // Mint pinned against both token accounts' actual on-chain mint field, not just their
    // addresses.
    let vault_mint = read_token_account_mint(vault_token_account, token_program.key)?;
    require_key_eq(&vault_mint, &state.mint, VaultNativeError::MintMismatch)?;
    let depositor_mint = read_token_account_mint(depositor_token_account, token_program.key)?;
    require_key_eq(&depositor_mint, &state.mint, VaultNativeError::MintMismatch)?;

    let decimals = read_mint_decimals(mint, token_program.key)?;

    // Ordinary signer transfer -- `depositor` authorizes it directly, no `invoke_signed` needed.
    invoke(
        &spl_token_interface::instruction::transfer_checked(
            token_program.key,
            depositor_token_account.key,
            mint.key,
            vault_token_account.key,
            depositor.key,
            &[],
            amount,
            decimals,
        )?,
        &[
            depositor_token_account.clone(),
            mint.clone(),
            vault_token_account.clone(),
            depositor.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let iter = &mut accounts.iter();
    let owner = next_account_info(iter)?;
    let vault_authority = next_account_info(iter)?;
    let vault_token_account = next_account_info(iter)?;
    let recipient_token_account = next_account_info(iter)?;
    let mint = next_account_info(iter)?;
    let token_program = next_account_info(iter)?;

    require_signer(owner)?;
    require_key_eq(
        token_program.key,
        &spl_token_interface::ID,
        VaultNativeError::WrongTokenProgram,
    )?;
    if amount == 0 {
        return Err(VaultNativeError::ZeroAmount.into());
    }

    let state = VaultAuthority::load(vault_authority, program_id)?;

    // Signer must match the vault's recorded owner exactly.
    require_key_eq(owner.key, &state.owner, VaultNativeError::OwnerMismatch)?;

    // Re-derive with the stored bump via `create_program_address`, never re-search.
    let expected_vault_authority = Pubkey::create_program_address(
        &[VAULT_AUTHORITY_SEED, state.mint.as_ref(), &[state.bump]],
        program_id,
    )
    .map_err(|_| VaultNativeError::InvalidPda)?;
    require_key_eq(
        vault_authority.key,
        &expected_vault_authority,
        VaultNativeError::InvalidPda,
    )?;

    require_key_eq(mint.key, &state.mint, VaultNativeError::MintMismatch)?;
    require_key_eq(
        vault_token_account.key,
        &state.vault_token_account,
        VaultNativeError::VaultAccountMismatch,
    )?;

    let vault_mint = read_token_account_mint(vault_token_account, token_program.key)?;
    require_key_eq(&vault_mint, &state.mint, VaultNativeError::MintMismatch)?;
    let recipient_mint = read_token_account_mint(recipient_token_account, token_program.key)?;
    require_key_eq(&recipient_mint, &state.mint, VaultNativeError::MintMismatch)?;

    let decimals = read_mint_decimals(mint, token_program.key)?;

    let signer_seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, state.mint.as_ref(), &[state.bump]];
    invoke_signed(
        &spl_token_interface::instruction::transfer_checked(
            token_program.key,
            vault_token_account.key,
            mint.key,
            recipient_token_account.key,
            vault_authority.key,
            &[],
            amount,
            decimals,
        )?,
        &[
            vault_token_account.clone(),
            mint.clone(),
            recipient_token_account.clone(),
            vault_authority.clone(),
            token_program.clone(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}
