//! Protocol/market/position lifecycle helpers built against the real `aegis` program — no mocks,
//! no stubs, real instructions sent through LiteSVM (`docs/zero-cost-demo.md` §6).

// `litesvm::types::TransactionResult`'s `Err` variant is a third-party type this crate does not
// control; test fixtures pass it straight through so callers can assert on either branch.
#![allow(clippy::result_large_err)]

use aegis::instructions::admin::{CreateMarketArgs, InitProtocolArgs};
use aegis::state::{Market, Position, Protocol};
use anchor_lang::solana_program::system_program;
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::types::TransactionResult;
use litesvm::LiteSVM;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::Instruction;
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use solana_transaction_error::TransactionError;

/// `borrow`, the debt path of `withdraw_collateral`, and `repay` when a nontrivial interval has
/// elapsed since the last accrual can all exceed the network's default 200,000 CU budget: each
/// performs several 256-bit `mul_div_*` divisions (accrual, share conversion, and -- for
/// `borrow`/`withdraw_collateral` -- oracle-band and LTV valuation on top) together with a full
/// Anchor account list and a token CPI. A real client must request a higher compute unit limit via
/// the standard `ComputeBudget` program, exactly as this test harness does below; this is a
/// resource-allocation concern, not a security check -- INV-RES-01's 200k-budget measurement is
/// explicitly Phase 11 (Performance) scope, not a Phase 5 acceptance criterion.
const HIGHER_COMPUTE_UNIT_LIMIT: u32 = 400_000;
/// `liquidate` is the most compute-heavy instruction in the protocol: it validates TWO oracle
/// feeds (not one), accrues interest, computes conservative valuation and the health factor, and
/// then runs the full liquidation math (`aegis_math::liquidation`) including the collateral-clamp
/// branch, which performs several additional 256-bit `mul_div_*` divisions on top of the
/// non-clamped path. `400_000` (sufficient for `borrow`/`repay`/`withdraw_collateral`) is not
/// always sufficient here, measured directly against the clamp path during Phase 6 authoring; a
/// real liquidator client must budget for the worst case. Resource-allocation, not a security
/// check -- as `HIGHER_COMPUTE_UNIT_LIMIT`'s own doc comment states, INV-RES-01 is Phase 11 scope.
const LIQUIDATE_COMPUTE_UNIT_LIMIT: u32 = 800_000;
/// Phase 8: the callback branch does everything the plain path does, PLUS an extra CPI running the
/// callback program's own logic (a swap, in the honest case). Sized generously since this is a
/// test-only compute request, not a production CU claim (INV-RES-01 is explicitly Phase 11 scope).
const LIQUIDATE_WITH_CALLBACK_COMPUTE_UNIT_LIMIT: u32 = 1_000_000;

// --- PDA derivation, mirroring account-model.md exactly ---

pub fn protocol_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[aegis::constants::PROTOCOL_SEED], &aegis::ID)
}

pub fn market_pda(collateral_mint: &Pubkey, loan_mint: &Pubkey, config_id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            aegis::constants::MARKET_SEED,
            collateral_mint.as_ref(),
            loan_mint.as_ref(),
            &config_id.to_le_bytes(),
        ],
        &aegis::ID,
    )
}

pub fn position_pda(market: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            aegis::constants::POSITION_SEED,
            market.as_ref(),
            owner.as_ref(),
        ],
        &aegis::ID,
    )
}

pub fn collateral_vault_pda(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[aegis::constants::COLLATERAL_VAULT_SEED, market.as_ref()],
        &aegis::ID,
    )
}

pub fn loan_vault_pda(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[aegis::constants::LOAN_VAULT_SEED, market.as_ref()],
        &aegis::ID,
    )
}

/// Phase 12: `PendingMarketParams` PDA (`PDA([b"pending_params", market])`).
pub fn pending_market_params_pda(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[aegis::constants::PENDING_PARAMS_SEED, market.as_ref()],
        &aegis::ID,
    )
}

// --- reference parameter set (economic-model.md §5.1) ---

/// SOL/USDC reference risk, IRM and oracle configuration from `economic-model.md` §5.1, with the
/// caller supplying only what genuinely varies between fixtures.
#[allow(clippy::too_many_arguments)]
pub fn reference_market_args(
    config_id: u16,
    collateral_feed_id: [u8; 32],
    loan_feed_id: [u8; 32],
    ack_freeze_authority: bool,
) -> CreateMarketArgs {
    CreateMarketArgs {
        config_id,
        oracle_kind: 0,
        collateral_feed_id,
        loan_feed_id,
        max_price_age_secs: 60,
        max_conf_bps: 100,
        max_ltv: 750_000_000_000_000_000,          // 0.75 WAD
        liq_threshold: 800_000_000_000_000_000,    // 0.80 WAD
        liq_bonus: 50_000_000_000_000_000,         // 0.05 WAD
        close_factor: 500_000_000_000_000_000,     // 0.50 WAD
        full_liq_hf: 950_000_000_000_000_000,      // 0.95 WAD
        liq_protocol_fee: 100_000_000_000_000_000, // 0.10 WAD
        fee: 100_000_000_000_000_000,              // 0.10 WAD
        min_debt: 10_000_000,                      // 10 USDC @ 6dp
        base_rate_ps: 0,
        slope1_ps: 0,
        slope2_ps: 0,
        u_kink: 800_000_000_000_000_000,        // 0.80 WAD
        max_rate_ps: 1_000_000_000_000_000_000, // 1.00 WAD
        ack_freeze_authority,
    }
}

// --- transaction submission ---

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ix: Instruction,
) -> TransactionResult {
    send_many(svm, payer, extra_signers, vec![ix])
}

fn send_many(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ixs: Vec<Instruction>,
) -> TransactionResult {
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&ixs, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers)
        .expect("failed to sign transaction");
    svm.send_transaction(tx)
}

/// As `send`, but prepends a `ComputeBudget::set_compute_unit_limit` instruction --
/// `borrow`/`withdraw_collateral`'s oracle-validated path needs it (see
/// `ORACLE_INSTRUCTION_COMPUTE_UNIT_LIMIT`'s doc comment).
fn send_priced(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ix: Instruction,
) -> TransactionResult {
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(HIGHER_COMPUTE_UNIT_LIMIT);
    send_many(svm, payer, extra_signers, vec![budget_ix, ix])
}

/// As `send_priced`, but with `LIQUIDATE_COMPUTE_UNIT_LIMIT` -- `liquidate`'s own, higher budget.
fn send_liquidate(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ix: Instruction,
) -> TransactionResult {
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(LIQUIDATE_COMPUTE_UNIT_LIMIT);
    send_many(svm, payer, extra_signers, vec![budget_ix, ix])
}

/// Asserts `result` failed with exactly the given `AegisError` — not merely that the transaction
/// failed (`testing-strategy.md` §4.2: "a test that merely asserts 'it failed' is not a security
/// test").
pub fn assert_aegis_error(result: &TransactionResult, expected: aegis::error::AegisError) {
    let expected_code = u32::from(expected);
    match result {
        Err(failed) => match &failed.err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
                assert_eq!(
                    *code, expected_code,
                    "expected AegisError code {expected_code}, got custom program error code {code}"
                );
            }
            other => panic!("expected a custom program error {expected_code}, got {other:?}"),
        },
        Ok(meta) => {
            panic!("expected AegisError code {expected_code}, transaction succeeded: {meta:?}")
        }
    }
}

// --- initialize_protocol ---

pub fn initialize_protocol_ix(
    admin: &Pubkey,
    guardian: Pubkey,
    fee_recipient: Pubkey,
) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::InitializeProtocol {
            payer: *admin,
            protocol,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: aegis::instruction::InitializeProtocol {
            args: InitProtocolArgs {
                guardian,
                fee_recipient,
            },
        }
        .data(),
    }
}

/// Initializes the protocol with `admin` as payer/admin. Returns the raw result so both
/// happy-path setup and adversarial tests (e.g. a second call, which must fail) can use it.
pub fn initialize_protocol(
    svm: &mut LiteSVM,
    admin: &Keypair,
    guardian: Pubkey,
    fee_recipient: Pubkey,
) -> TransactionResult {
    let ix = initialize_protocol_ix(&admin.pubkey(), guardian, fee_recipient);
    send(svm, admin, &[], ix)
}

// --- create_market ---

/// Creates a market. `protocol_fee_recipient` must be the live `Protocol.fee_recipient` (fetch it
/// with [`fetch_protocol`] first) so the mandatory fee `Position` is derived correctly.
#[allow(clippy::too_many_arguments)]
pub fn create_market(
    svm: &mut LiteSVM,
    admin: &Keypair,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
    collateral_token_program: Pubkey,
    loan_token_program: Pubkey,
    protocol_fee_recipient: Pubkey,
    args: CreateMarketArgs,
) -> (TransactionResult, Pubkey, Pubkey, Pubkey, Pubkey) {
    let (protocol, _) = protocol_pda();
    let (market, _) = market_pda(&collateral_mint, &loan_mint, args.config_id);
    let (collateral_vault, _) = collateral_vault_pda(&market);
    let (loan_vault, _) = loan_vault_pda(&market);
    let (fee_position, _) = position_pda(&market, &protocol_fee_recipient);

    let ix = Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::CreateMarket {
            admin: admin.pubkey(),
            protocol,
            collateral_mint,
            loan_mint,
            collateral_token_program,
            loan_token_program,
            market,
            collateral_vault,
            loan_vault,
            fee_position,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: aegis::instruction::CreateMarket { args }.data(),
    };
    let result = send(svm, admin, &[], ix);
    (result, market, collateral_vault, loan_vault, fee_position)
}

// --- init_position ---

pub fn init_position_ix(payer: &Pubkey, market: Pubkey, owner: Pubkey) -> (Instruction, Pubkey) {
    let (position, _) = position_pda(&market, &owner);
    let ix = Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::InitPosition {
            payer: *payer,
            market,
            owner,
            position,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: aegis::instruction::InitPosition {}.data(),
    };
    (ix, position)
}

pub fn init_position(
    svm: &mut LiteSVM,
    payer: &Keypair,
    market: Pubkey,
    owner: Pubkey,
) -> (TransactionResult, Pubkey) {
    let (ix, position) = init_position_ix(&payer.pubkey(), market, owner);
    (send(svm, payer, &[], ix), position)
}

// --- deposit_collateral ---

#[allow(clippy::too_many_arguments)]
pub fn deposit_collateral_ix(
    depositor: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    collateral_vault: Pubkey,
    depositor_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::DepositCollateral {
            depositor: *depositor,
            market,
            position,
            collateral_vault,
            depositor_collateral_ata,
            collateral_mint,
            collateral_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::DepositCollateral { amount }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_collateral(
    svm: &mut LiteSVM,
    depositor: &Keypair,
    market: Pubkey,
    position: Pubkey,
    collateral_vault: Pubkey,
    depositor_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    amount: u64,
) -> TransactionResult {
    let ix = deposit_collateral_ix(
        &depositor.pubkey(),
        market,
        position,
        collateral_vault,
        depositor_collateral_ata,
        collateral_mint,
        collateral_token_program,
        amount,
    );
    send(svm, depositor, &[], ix)
}

// --- withdraw_collateral ---

#[allow(clippy::too_many_arguments)]
pub fn withdraw_collateral_ix(
    owner: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    collateral_vault: Pubkey,
    owner_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::WithdrawCollateral {
            owner: *owner,
            protocol: protocol_pda().0,
            market,
            position,
            collateral_vault,
            owner_collateral_ata,
            collateral_mint,
            collateral_token_program,
            collateral_price_update,
            loan_price_update,
        }
        .to_account_metas(None),
        data: aegis::instruction::WithdrawCollateral { amount }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_collateral(
    svm: &mut LiteSVM,
    owner: &Keypair,
    market: Pubkey,
    position: Pubkey,
    collateral_vault: Pubkey,
    owner_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    amount: u64,
) -> TransactionResult {
    let ix = withdraw_collateral_ix(
        &owner.pubkey(),
        market,
        position,
        collateral_vault,
        owner_collateral_ata,
        collateral_mint,
        collateral_token_program,
        collateral_price_update,
        loan_price_update,
        amount,
    );
    send_priced(svm, owner, &[], ix)
}

// --- close_position ---

pub fn close_position_ix(owner: &Pubkey, market: Pubkey, position: Pubkey) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::ClosePosition {
            owner: *owner,
            market,
            position,
        }
        .to_account_metas(None),
        data: aegis::instruction::ClosePosition {}.data(),
    }
}

pub fn close_position(
    svm: &mut LiteSVM,
    owner: &Keypair,
    market: Pubkey,
    position: Pubkey,
) -> TransactionResult {
    let ix = close_position_ix(&owner.pubkey(), market, position);
    send(svm, owner, &[], ix)
}

// --- supply ---

#[allow(clippy::too_many_arguments)]
pub fn supply_ix(
    owner: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Supply {
            owner: *owner,
            protocol: protocol_pda().0,
            market,
            position,
            fee_position,
            loan_vault,
            owner_loan_ata,
            loan_mint,
            loan_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::Supply { assets, shares }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn supply(
    svm: &mut LiteSVM,
    owner: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> TransactionResult {
    let ix = supply_ix(
        &owner.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        owner_loan_ata,
        loan_mint,
        loan_token_program,
        assets,
        shares,
    );
    send(svm, owner, &[], ix)
}

// --- withdraw ---

#[allow(clippy::too_many_arguments)]
pub fn withdraw_ix(
    owner: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Withdraw {
            owner: *owner,
            protocol: protocol_pda().0,
            market,
            position,
            fee_position,
            loan_vault,
            owner_loan_ata,
            loan_mint,
            loan_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::Withdraw { assets, shares }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    svm: &mut LiteSVM,
    owner: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> TransactionResult {
    let ix = withdraw_ix(
        &owner.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        owner_loan_ata,
        loan_mint,
        loan_token_program,
        assets,
        shares,
    );
    send(svm, owner, &[], ix)
}

// --- borrow (real, oracle-validated -- Phase 5) ---

#[allow(clippy::too_many_arguments)]
pub fn borrow_ix(
    owner: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    assets: u64,
    shares: u128,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Borrow {
            owner: *owner,
            protocol: protocol_pda().0,
            market,
            position,
            fee_position,
            loan_vault,
            owner_loan_ata,
            loan_mint,
            loan_token_program,
            collateral_price_update,
            loan_price_update,
        }
        .to_account_metas(None),
        data: aegis::instruction::Borrow { assets, shares }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn borrow(
    svm: &mut LiteSVM,
    owner: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    owner_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    assets: u64,
    shares: u128,
) -> TransactionResult {
    let ix = borrow_ix(
        &owner.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        owner_loan_ata,
        loan_mint,
        loan_token_program,
        collateral_price_update,
        loan_price_update,
        assets,
        shares,
    );
    send_priced(svm, owner, &[], ix)
}

// --- repay ---

#[allow(clippy::too_many_arguments)]
pub fn repay_ix(
    payer: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    payer_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Repay {
            payer: *payer,
            market,
            position,
            fee_position,
            loan_vault,
            payer_loan_ata,
            loan_mint,
            loan_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::Repay { assets, shares }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn repay(
    svm: &mut LiteSVM,
    payer: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    payer_loan_ata: Pubkey,
    loan_mint: Pubkey,
    loan_token_program: Pubkey,
    assets: u64,
    shares: u128,
) -> TransactionResult {
    let ix = repay_ix(
        &payer.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        payer_loan_ata,
        loan_mint,
        loan_token_program,
        assets,
        shares,
    );
    // `repay` accrues interest before transferring (economic-model.md §4.5); a nonzero `dt` since
    // the last accrual, combined with the token CPI and full account list, can exceed the default
    // 200,000 CU the same way borrow's does -- see `ORACLE_INSTRUCTION_COMPUTE_UNIT_LIMIT`'s doc
    // comment (this is a pre-existing repay/accrual cost, unrelated to the oracle; no prior-phase
    // test exercised repay together with a large accrual gap, only `accrue_interest` standalone).
    send_priced(svm, payer, &[], ix)
}

// --- accrue_interest ---

pub fn accrue_interest_ix(market: Pubkey, fee_position: Pubkey) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::AccrueInterest {
            market,
            fee_position,
        }
        .to_account_metas(None),
        data: aegis::instruction::AccrueInterest {}.data(),
    }
}

/// Permissionless: any funded keypair can pay for and submit this transaction.
pub fn accrue_interest(
    svm: &mut LiteSVM,
    payer: &Keypair,
    market: Pubkey,
    fee_position: Pubkey,
) -> TransactionResult {
    let ix = accrue_interest_ix(market, fee_position);
    send(svm, payer, &[], ix)
}

// --- liquidate ---

#[allow(clippy::too_many_arguments)]
pub fn liquidate_ix(
    liquidator: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    collateral_vault: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    loan_mint: Pubkey,
    collateral_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::Liquidate {
            liquidator: *liquidator,
            protocol: protocol_pda().0,
            market,
            position,
            fee_position,
            loan_vault,
            collateral_vault,
            liquidator_loan_ata,
            liquidator_collateral_ata,
            loan_mint,
            collateral_mint,
            loan_token_program,
            collateral_token_program,
            collateral_price_update,
            loan_price_update,
            callback_program: None,
            callback_collateral_account: None,
        }
        .to_account_metas(None),
        data: aegis::instruction::Liquidate {
            repay_assets,
            seize_collateral,
            callback_data: vec![],
        }
        .data(),
    }
}

/// Phase 8 (`docs/composability.md`, ADR-0013): as `liquidate_ix`, but with the optional callback
/// supplied. `extra_accounts` becomes `ctx.remaining_accounts` verbatim — the callback's own,
/// liquidator-chosen accounts (e.g. a swap route), opaque to Aegis.
#[allow(clippy::too_many_arguments)]
pub fn liquidate_with_callback_ix(
    liquidator: &Pubkey,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    collateral_vault: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    loan_mint: Pubkey,
    collateral_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
    callback_program: Pubkey,
    callback_collateral_account: Pubkey,
    extra_accounts: Vec<solana_instruction::AccountMeta>,
    callback_data: Vec<u8>,
) -> Instruction {
    let mut accounts = aegis::accounts::Liquidate {
        liquidator: *liquidator,
        protocol: protocol_pda().0,
        market,
        position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        loan_mint,
        collateral_mint,
        loan_token_program,
        collateral_token_program,
        collateral_price_update,
        loan_price_update,
        callback_program: Some(callback_program),
        callback_collateral_account: Some(callback_collateral_account),
    }
    .to_account_metas(None);
    accounts.extend(extra_accounts);

    Instruction {
        program_id: aegis::ID,
        accounts,
        data: aegis::instruction::Liquidate {
            repay_assets,
            seize_collateral,
            callback_data,
        }
        .data(),
    }
}

/// Permissionless: any funded keypair can pay for and submit this transaction.
#[allow(clippy::too_many_arguments)]
pub fn liquidate_with_callback(
    svm: &mut LiteSVM,
    liquidator: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    collateral_vault: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    loan_mint: Pubkey,
    collateral_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
    callback_program: Pubkey,
    callback_collateral_account: Pubkey,
    extra_accounts: Vec<solana_instruction::AccountMeta>,
    callback_data: Vec<u8>,
) -> TransactionResult {
    let ix = liquidate_with_callback_ix(
        &liquidator.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        loan_mint,
        collateral_mint,
        loan_token_program,
        collateral_token_program,
        collateral_price_update,
        loan_price_update,
        repay_assets,
        seize_collateral,
        callback_program,
        callback_collateral_account,
        extra_accounts,
        callback_data,
    );
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(
        LIQUIDATE_WITH_CALLBACK_COMPUTE_UNIT_LIMIT,
    );
    send_many(svm, liquidator, &[], vec![budget_ix, ix])
}

/// As `liquidate_with_callback_ix`, but with a caller-chosen compute unit limit instead of the
/// generous default -- `A-CPI-03` needs a bounded outer limit to prove a compute-exhausting
/// callback fails cleanly rather than merely "eventually", and needs it smaller than what a
/// hostile callback's own burn loop will consume.
#[allow(clippy::too_many_arguments)]
pub fn liquidate_with_callback_and_compute_limit(
    svm: &mut LiteSVM,
    liquidator: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    collateral_vault: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    loan_mint: Pubkey,
    collateral_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
    callback_program: Pubkey,
    callback_collateral_account: Pubkey,
    extra_accounts: Vec<solana_instruction::AccountMeta>,
    callback_data: Vec<u8>,
    compute_unit_limit: u32,
) -> TransactionResult {
    let ix = liquidate_with_callback_ix(
        &liquidator.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        loan_mint,
        collateral_mint,
        loan_token_program,
        collateral_token_program,
        collateral_price_update,
        loan_price_update,
        repay_assets,
        seize_collateral,
        callback_program,
        callback_collateral_account,
        extra_accounts,
        callback_data,
    );
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit);
    send_many(svm, liquidator, &[], vec![budget_ix, ix])
}

#[allow(clippy::too_many_arguments)]
pub fn liquidate(
    svm: &mut LiteSVM,
    liquidator: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    loan_vault: Pubkey,
    collateral_vault: Pubkey,
    liquidator_loan_ata: Pubkey,
    liquidator_collateral_ata: Pubkey,
    loan_mint: Pubkey,
    collateral_mint: Pubkey,
    loan_token_program: Pubkey,
    collateral_token_program: Pubkey,
    collateral_price_update: Pubkey,
    loan_price_update: Pubkey,
    repay_assets: u64,
    seize_collateral: u64,
) -> TransactionResult {
    let ix = liquidate_ix(
        &liquidator.pubkey(),
        market,
        position,
        fee_position,
        loan_vault,
        collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        loan_mint,
        collateral_mint,
        loan_token_program,
        collateral_token_program,
        collateral_price_update,
        loan_price_update,
        repay_assets,
        seize_collateral,
    );
    // Oracle-validated for TWO feeds plus the full liquidation math (including the
    // collateral-clamp branch) -- `liquidate`'s own, higher compute budget.
    send_liquidate(svm, liquidator, &[], ix)
}

// --- absorb_bad_debt ---

pub fn absorb_bad_debt_ix(market: Pubkey, position: Pubkey, fee_position: Pubkey) -> Instruction {
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::AbsorbBadDebt {
            market,
            position,
            fee_position,
        }
        .to_account_metas(None),
        data: aegis::instruction::AbsorbBadDebt {}.data(),
    }
}

/// Permissionless: any funded keypair can pay for and submit this transaction.
pub fn absorb_bad_debt(
    svm: &mut LiteSVM,
    payer: &Keypair,
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
) -> TransactionResult {
    let ix = absorb_bad_debt_ix(market, position, fee_position);
    send(svm, payer, &[], ix)
}

// --- withdraw_collateral_fees ---

#[allow(clippy::too_many_arguments)]
pub fn withdraw_collateral_fees_ix(
    admin: &Pubkey,
    market: Pubkey,
    collateral_vault: Pubkey,
    admin_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    amount: u64,
) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::WithdrawCollateralFees {
            admin: *admin,
            protocol,
            market,
            collateral_vault,
            admin_collateral_ata,
            collateral_mint,
            collateral_token_program,
        }
        .to_account_metas(None),
        data: aegis::instruction::WithdrawCollateralFees { amount }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_collateral_fees(
    svm: &mut LiteSVM,
    admin: &Keypair,
    market: Pubkey,
    collateral_vault: Pubkey,
    admin_collateral_ata: Pubkey,
    collateral_mint: Pubkey,
    collateral_token_program: Pubkey,
    amount: u64,
) -> TransactionResult {
    let ix = withdraw_collateral_fees_ix(
        &admin.pubkey(),
        market,
        collateral_vault,
        admin_collateral_ata,
        collateral_mint,
        collateral_token_program,
        amount,
    );
    send(svm, admin, &[], ix)
}

// --- account fetch/decode ---

pub fn fetch_protocol(svm: &LiteSVM, protocol: &Pubkey) -> Protocol {
    let account = svm
        .get_account(protocol)
        .expect("protocol account must exist");
    Protocol::try_deserialize(&mut account.data.as_slice()).expect("valid Protocol account")
}

pub fn fetch_market(svm: &LiteSVM, market: &Pubkey) -> Market {
    let account = svm.get_account(market).expect("market account must exist");
    Market::try_deserialize(&mut account.data.as_slice()).expect("valid Market account")
}

pub fn fetch_position(svm: &LiteSVM, position: &Pubkey) -> Position {
    let account = svm
        .get_account(position)
        .expect("position account must exist");
    Position::try_deserialize(&mut account.data.as_slice()).expect("valid Position account")
}

pub fn fetch_pending_market_params(
    svm: &LiteSVM,
    pending_market_params: &Pubkey,
) -> aegis::state::PendingMarketParams {
    let account = svm
        .get_account(pending_market_params)
        .expect("pending_market_params account must exist");
    aegis::state::PendingMarketParams::try_deserialize(&mut account.data.as_slice())
        .expect("valid PendingMarketParams account")
}

/// `None` iff no `PendingMarketParams` account currently exists for `market` (uninitialized,
/// system-owned) — distinct from an account that exists but errors to deserialize.
pub fn try_fetch_pending_market_params(
    svm: &LiteSVM,
    market: &Pubkey,
) -> Option<aegis::state::PendingMarketParams> {
    let (pending, _) = pending_market_params_pda(market);
    let account = svm.get_account(&pending)?;
    if account.owner == anchor_lang::solana_program::system_program::ID && account.lamports == 0 {
        return None;
    }
    Some(
        aegis::state::PendingMarketParams::try_deserialize(&mut account.data.as_slice())
            .expect("valid PendingMarketParams account"),
    )
}

// =====================================================================================
// Phase 12: governance, upgrades, and migrations
// =====================================================================================

// --- set_pending_admin / accept_admin ---

pub fn set_pending_admin_ix(admin: &Pubkey, new_admin: Pubkey) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::SetPendingAdmin {
            admin: *admin,
            protocol,
        }
        .to_account_metas(None),
        data: aegis::instruction::SetPendingAdmin { new_admin }.data(),
    }
}

pub fn set_pending_admin(
    svm: &mut LiteSVM,
    admin: &Keypair,
    new_admin: Pubkey,
) -> TransactionResult {
    let ix = set_pending_admin_ix(&admin.pubkey(), new_admin);
    send(svm, admin, &[], ix)
}

pub fn accept_admin_ix(pending_admin: &Pubkey) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::AcceptAdmin {
            pending_admin: *pending_admin,
            protocol,
        }
        .to_account_metas(None),
        data: aegis::instruction::AcceptAdmin {}.data(),
    }
}

pub fn accept_admin(svm: &mut LiteSVM, pending_admin: &Keypair) -> TransactionResult {
    let ix = accept_admin_ix(&pending_admin.pubkey());
    send(svm, pending_admin, &[], ix)
}

// --- set_guardian ---

pub fn set_guardian_ix(admin: &Pubkey, new_guardian: Pubkey) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::SetGuardian {
            admin: *admin,
            protocol,
        }
        .to_account_metas(None),
        data: aegis::instruction::SetGuardian { new_guardian }.data(),
    }
}

pub fn set_guardian(svm: &mut LiteSVM, admin: &Keypair, new_guardian: Pubkey) -> TransactionResult {
    let ix = set_guardian_ix(&admin.pubkey(), new_guardian);
    send(svm, admin, &[], ix)
}

// --- set_protocol_pause / set_market_pause ---

pub fn set_protocol_pause_ix(authority: &Pubkey, flags: u8) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::SetProtocolPause {
            authority: *authority,
            protocol,
        }
        .to_account_metas(None),
        data: aegis::instruction::SetProtocolPause { flags }.data(),
    }
}

pub fn set_protocol_pause(svm: &mut LiteSVM, authority: &Keypair, flags: u8) -> TransactionResult {
    let ix = set_protocol_pause_ix(&authority.pubkey(), flags);
    send(svm, authority, &[], ix)
}

pub fn set_market_pause_ix(authority: &Pubkey, market: Pubkey, flags: u8) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::SetMarketPause {
            authority: *authority,
            protocol,
            market,
        }
        .to_account_metas(None),
        data: aegis::instruction::SetMarketPause { flags }.data(),
    }
}

pub fn set_market_pause(
    svm: &mut LiteSVM,
    authority: &Keypair,
    market: Pubkey,
    flags: u8,
) -> TransactionResult {
    let ix = set_market_pause_ix(&authority.pubkey(), market, flags);
    send(svm, authority, &[], ix)
}

// --- set_market_params / commit_pending_params ---

/// `new_fee_position` is only required (and only actually validated on-chain) when
/// `args.fee_recipient` differs from the market's current `fee_recipient` -- pass `None` when it
/// is unchanged.
#[allow(clippy::too_many_arguments)]
pub fn set_market_params_ix(
    admin: &Pubkey,
    market: Pubkey,
    fee_position: Pubkey,
    new_fee_position: Option<Pubkey>,
    args: aegis::instructions::admin::SetMarketParamsArgs,
) -> Instruction {
    let (protocol, _) = protocol_pda();
    let (pending_market_params, _) = pending_market_params_pda(&market);
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::SetMarketParams {
            admin: *admin,
            protocol,
            market,
            fee_position,
            new_fee_position,
            pending_market_params,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: aegis::instruction::SetMarketParams { args }.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn set_market_params(
    svm: &mut LiteSVM,
    admin: &Keypair,
    market: Pubkey,
    fee_position: Pubkey,
    new_fee_position: Option<Pubkey>,
    args: aegis::instructions::admin::SetMarketParamsArgs,
) -> TransactionResult {
    let ix = set_market_params_ix(
        &admin.pubkey(),
        market,
        fee_position,
        new_fee_position,
        args,
    );
    send_priced(svm, admin, &[], ix)
}

/// `fee_position` must be `PDA(market, market.fee_recipient)` for the market's **current**
/// `fee_recipient` at call time (fetch it with [`fetch_market`] first) -- accepted as a parameter
/// rather than re-derived here since this function has no `svm` access of its own.
pub fn commit_pending_params_ix(
    payer: &Pubkey,
    admin: Pubkey,
    market: Pubkey,
    fee_position: Pubkey,
) -> Instruction {
    let (protocol, _) = protocol_pda();
    let (pending_market_params, _) = pending_market_params_pda(&market);
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::CommitPendingParams {
            payer: *payer,
            protocol,
            admin,
            market,
            fee_position,
            pending_market_params,
        }
        .to_account_metas(None),
        data: aegis::instruction::CommitPendingParams {}.data(),
    }
}

pub fn commit_pending_params(
    svm: &mut LiteSVM,
    payer: &Keypair,
    admin: Pubkey,
    market: Pubkey,
    fee_position: Pubkey,
) -> TransactionResult {
    let ix = commit_pending_params_ix(&payer.pubkey(), admin, market, fee_position);
    send(svm, payer, &[], ix)
}

// --- migrate_protocol_v2 ---

pub fn migrate_protocol_v2_ix(admin: &Pubkey) -> Instruction {
    let (protocol, _) = protocol_pda();
    Instruction {
        program_id: aegis::ID,
        accounts: aegis::accounts::MigrateProtocolV2 {
            admin: *admin,
            protocol,
        }
        .to_account_metas(None),
        data: aegis::instruction::MigrateProtocolV2 {}.data(),
    }
}

pub fn migrate_protocol_v2(svm: &mut LiteSVM, admin: &Keypair) -> TransactionResult {
    let ix = migrate_protocol_v2_ix(&admin.pubkey());
    send(svm, admin, &[], ix)
}
