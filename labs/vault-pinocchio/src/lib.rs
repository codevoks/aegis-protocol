//! Phase 11 lab (`docs/adr/0003-native-pinocchio-as-labs.md`, `docs/phases/phase-11-performance.md`
//! #23-24, #26): the SAME custody primitive `labs/vault-anchor` implements -- initialize a vault
//! PDA, deposit into it, withdraw from it via a PDA-signed CPI -- written against Pinocchio
//! instead of Anchor: `no_std`, no framework macros, manual account parsing, raw byte-tag
//! instruction encoding.
//!
//! **Scope, deliberately bounded** (ADR-0003), identical to `labs/vault-anchor`: classic SPL
//! Token only, one mint, one owner per vault. Not a re-implementation of Aegis's
//! Market/Position/oracle/liquidation logic -- exactly this custody shape and nothing else.
//!
//! ## Instruction encoding
//! Byte 0 is the tag (`0` = [`TAG_INITIALIZE`], `1` = [`TAG_DEPOSIT`], `2` = [`TAG_WITHDRAW`]).
//! Deposit/Withdraw additionally take a little-endian `u64` amount in bytes `1..9`. See
//! `instructions/{initialize,deposit,withdraw}.rs` for the exact account lists.
//!
//! ## Security checks (equivalent to `labs/vault-anchor`'s Anchor constraints,
//! `phase-11-performance.md` #27) -- item 26 explicitly forbids benchmarking an unsafe
//! stripped-down Pinocchio program against a fully validated Anchor one, so every one of these is
//! hand-written here, not skipped:
//! - **Signer**: `is_signer()` checked on `payer` (init), `depositor` (deposit), and `owner`
//!   (withdraw).
//! - **Owning-program**: `vault_authority` must be `owned_by` this program, and exactly
//!   [`state::VaultAuthority::LEN`] bytes, before any of its fields are trusted (deposit,
//!   withdraw). `vault_token_account` and every user-supplied token account must be `owned_by`
//!   the real SPL Token program.
//! - **Token program pin**: `token_program`'s address must literally equal
//!   `pinocchio_token::ID` (the real classic SPL Token program) -- checked directly, not via
//!   `TokenProgram::verify`, which also accepts Token-2022 and would silently widen this lab's
//!   deliberately classic-SPL-only scope.
//! - **PDA / vault**: `vault_authority` is re-derived on every instruction -- `Initialize` via
//!   `find_program_address` (the bump is not yet known), `Deposit`/`Withdraw` via
//!   `create_program_address` from the *stored* bump, never re-searched, matching
//!   `phase-11-performance.md`'s hot-path guidance. `vault_token_account`'s address, as recorded
//!   in `VaultAuthority` state, is compared against the account actually passed in.
//! - **Authority**: the vault token account's SPL-level authority is `vault_authority`, and only
//!   `vault_authority`'s own PDA signature (via a signed CPI, `[VAULT_AUTHORITY_SEED, mint,
//!   bump]`) can move tokens out of it.
//! - **Mint pin**: every token account's mint (the first 32 bytes of its raw SPL layout, see
//!   `spl.rs`) is compared against `VaultAuthority.mint`.
//! - **Amount**: `amount > 0` is required (`VaultError::ZeroAmount`); `TransferChecked` is used
//!   for every transfer (validates mint + decimals), never plain `Transfer`.
//!
//! Arithmetic note: this primitive never adds, subtracts, multiplies, or divides a caller-
//! supplied amount -- it forwards `amount` verbatim to `TransferChecked` and lets the token
//! program account for the balance change. There is therefore no local arithmetic that could
//! overflow to guard with checked operations.
//!
//! ## Why this crate compiles unmodified for both `cargo build-sbf` and a host test binary
//! Unlike Anchor's `no-entrypoint` feature, Pinocchio's own `entrypoint!`, `default_allocator!`
//! and `default_panic_handler!` macros already gate every target-specific effect behind
//! `cfg(any(target_os = "solana", target_arch = "bpf"))` internally (verified by reading
//! `pinocchio` 0.11.2's own macro source): off that target, the global allocator and panic
//! handler macros degrade to a no-op `extern crate std as __std;`, and the CPI syscalls
//! (`invoke_signed`, `create_program_address`'s syscall path, etc.) degrade to `black_box`
//! no-ops or fall back to a pure-Rust implementation gated on the `curve25519` feature (already
//! active in this workspace's resolved dependency graph, via `solana-message`/
//! `solana-transaction`'s own host-side PDA needs -- the same mechanism `labs/vault-anchor`'s own
//! tests already rely on). So this crate needs no extra `#[cfg]` gymnastics or Cargo feature to
//! be linked as an ordinary host `lib` by `tests/basic.rs`: the real on-chain logic simply never
//! runs there, since the test drives the actual built `.so` through `LiteSVM` instead of calling
//! `process_instruction` as a native Rust function.
#![no_std]

use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

pub mod error;
pub mod instructions;
pub mod spl;
pub mod state;

pub use error::VaultError;
pub use state::VaultAuthority;

/// This program's own on-chain address -- the keypair `cargo build-sbf` generated at
/// `target/deploy/vault_pinocchio-keypair.json` (not committed; `target/` is gitignored), the
/// same reconciliation `labs/vault-anchor` already needed between its `declare_id!` and its own
/// build-generated keypair. See `docs/phases/phase-11-performance.md` #23/#26.
pub const ID: Address = Address::from_str_const("CKurGQ2wRSZi4tjLzeh1xseCNrBCVTvYBLB1rBMYjVKW");

pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const VAULT_TOKEN_SEED: &[u8] = b"vault_token";

pub const TAG_INITIALIZE: u8 = 0;
pub const TAG_DEPOSIT: u8 = 1;
pub const TAG_WITHDRAW: u8 = 2;

// `entrypoint!` alone expands to `program_entrypoint!` + `default_allocator!` +
// `default_panic_handler!`; the last of those only registers a post-panic logging *hook* (see
// pinocchio 0.11.2's own doc comment on `entrypoint!`) and relies on `std` to supply the actual
// `#[panic_handler]` lang item. This crate has no `std` anywhere in its dependency tree for the
// real `sbf-solana-solana` build, so nothing would provide that lang item and the SBF build
// fails to link ("`#[panic_handler]` function required, but not found") -- confirmed by trying
// `entrypoint!` first. `nostd_panic_handler!` is Pinocchio's own answer for exactly this case.
pinocchio::program_entrypoint!(process_instruction);
pinocchio::default_allocator!();
pinocchio::nostd_panic_handler!();

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(VaultError::IncorrectProgramId.into());
    }

    let (&tag, rest) = instruction_data
        .split_first()
        .ok_or(VaultError::InvalidInstructionData)?;

    match tag {
        TAG_INITIALIZE => instructions::initialize::process(program_id, accounts),
        TAG_DEPOSIT => instructions::deposit::process(program_id, accounts, parse_amount(rest)?),
        TAG_WITHDRAW => instructions::withdraw::process(program_id, accounts, parse_amount(rest)?),
        _ => Err(VaultError::InvalidInstructionData.into()),
    }
}

fn parse_amount(data: &[u8]) -> Result<u64, ProgramError> {
    let bytes: [u8; 8] = data
        .get(0..8)
        .ok_or(VaultError::InvalidInstructionData)?
        .try_into()
        .map_err(|_| VaultError::InvalidInstructionData)?;
    Ok(u64::from_le_bytes(bytes))
}
