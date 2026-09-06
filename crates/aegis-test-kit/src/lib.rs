//! Local, offline LiteSVM test fixtures for Aegis Protocol.
//!
//! Phase 2 adds mint factories (SPL Token and Token-2022, with configurable extensions) and
//! protocol/market/position lifecycle helpers on top of Phase 1's bootstrap
//! (see `docs/zero-cost-demo.md` §6). Everything here is deterministic and runs with no network:
//! LiteSVM ships the real SPL Token / Token-2022 program bytecode it needs, in-process.

pub mod invariants;
pub mod market;
pub mod mints;
pub mod state_injection;
pub mod svm;
pub mod token_accounts;
pub mod user_tokens;

pub use market::{
    accrue_interest, accrue_interest_ix, assert_aegis_error, borrow, borrow_ix, close_position,
    close_position_ix, collateral_vault_pda, create_market, deposit_collateral,
    deposit_collateral_ix, fetch_market, fetch_position, fetch_protocol, init_position,
    initialize_protocol, loan_vault_pda, market_pda, position_pda, protocol_pda,
    reference_market_args, repay, repay_ix, supply, supply_ix, withdraw, withdraw_collateral,
    withdraw_collateral_ix, withdraw_ix,
};
pub use mints::{
    create_spl_mint, create_token_2022_mint, create_token_2022_mint_with_unrecognized_extension,
    Token2022Extension,
};
pub use state_injection::{seed_borrow_state, set_token_account_amount};
pub use svm::{deploy, deterministic_payer};
pub use token_accounts::{fetch_mint_extension_types, fetch_token_account_base};
pub use user_tokens::{create_token_account, mint_to};

// Re-exported so integration tests can reference program IDs and low-level types without
// declaring their own, separately-versioned dependency on these crates.
pub use spl_token_2022_interface;
pub use spl_token_interface;
