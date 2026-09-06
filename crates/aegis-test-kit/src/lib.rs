//! Local, offline LiteSVM test fixtures for Aegis Protocol.
//!
//! Phase 2 adds mint factories (SPL Token and Token-2022, with configurable extensions) and
//! protocol/market/position lifecycle helpers on top of Phase 1's bootstrap
//! (see `docs/zero-cost-demo.md` §6). Everything here is deterministic and runs with no network:
//! LiteSVM ships the real SPL Token / Token-2022 program bytecode it needs, in-process.

pub mod market;
pub mod mints;
pub mod svm;
pub mod token_accounts;

pub use market::{
    assert_aegis_error, collateral_vault_pda, create_market, fetch_market, fetch_position,
    fetch_protocol, init_position, initialize_protocol, loan_vault_pda, market_pda, position_pda,
    protocol_pda, reference_market_args,
};
pub use mints::{
    create_spl_mint, create_token_2022_mint, create_token_2022_mint_with_unrecognized_extension,
    Token2022Extension,
};
pub use svm::{deploy, deterministic_payer};
pub use token_accounts::{fetch_mint_extension_types, fetch_token_account_base};

// Re-exported so integration tests can reference program IDs and low-level types without
// declaring their own, separately-versioned dependency on these crates.
pub use spl_token_2022_interface;
pub use spl_token_interface;
