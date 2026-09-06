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
