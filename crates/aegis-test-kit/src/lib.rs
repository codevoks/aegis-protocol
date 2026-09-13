//! Local, offline LiteSVM test fixtures for Aegis Protocol.
//!
//! Phase 2 adds mint factories (SPL Token and Token-2022, with configurable extensions) and
//! protocol/market/position lifecycle helpers on top of Phase 1's bootstrap
//! (see `docs/zero-cost-demo.md` §6). Everything here is deterministic and runs with no network:
//! LiteSVM ships the real SPL Token / Token-2022 program bytecode it needs, in-process.

pub mod invariants;
pub mod market;
pub mod mints;
pub mod pyth_fixture;
pub mod state_injection;
pub mod svm;
pub mod token_accounts;
pub mod user_tokens;

pub use market::{
    absorb_bad_debt, absorb_bad_debt_ix, accept_admin, accept_admin_ix, accrue_interest,
    accrue_interest_ix, assert_aegis_error, borrow, borrow_ix, close_position, close_position_ix,
    collateral_vault_pda, commit_pending_params, commit_pending_params_ix, create_market,
    deposit_collateral, deposit_collateral_ix, fetch_market, fetch_pending_market_params,
    fetch_position, fetch_protocol, init_position, initialize_protocol, liquidate, liquidate_ix,
    liquidate_with_callback, liquidate_with_callback_and_compute_limit, liquidate_with_callback_ix,
    loan_vault_pda, market_pda, migrate_protocol_v2, migrate_protocol_v2_ix,
    pending_market_params_pda, position_pda, protocol_pda, reference_market_args, repay, repay_ix,
    set_guardian, set_guardian_ix, set_market_params, set_market_params_ix, set_market_pause,
    set_market_pause_ix, set_pending_admin, set_pending_admin_ix, set_protocol_pause,
    set_protocol_pause_ix, supply, supply_ix, try_fetch_pending_market_params, withdraw,
    withdraw_collateral, withdraw_collateral_fees, withdraw_collateral_fees_ix,
    withdraw_collateral_ix, withdraw_ix,
};
pub use mints::{
    advance_epoch, create_spl_mint, create_token_2022_mint,
    create_token_2022_mint_with_unrecognized_extension, fetch_transfer_fee_config,
    set_transfer_fee_rate, Token2022Extension, TransferFeeConfig,
};
pub use pyth_fixture::{inject_price_update, set_price, PriceFixture};
pub use state_injection::{seed_borrow_state, set_liquidation_guard, set_token_account_amount};
pub use svm::{deploy, deterministic_payer};
pub use token_accounts::{
    fetch_mint_decimals, fetch_mint_extension_types, fetch_token_account_base,
};
pub use user_tokens::{create_immutable_owner_account, create_token_account, mint_to};

// Re-exported so tests can construct raw Pyth account state (owner overrides, VerificationLevel
// variants) without declaring their own separately-versioned dependency on this crate.
pub use pyth_solana_receiver_sdk;

// Re-exported so integration tests can reference program IDs and low-level types without
// declaring their own, separately-versioned dependency on these crates.
pub use spl_token_2022_interface;
pub use spl_token_interface;
