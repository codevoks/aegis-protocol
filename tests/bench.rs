//! Phase 11 — the Mollusk CU benchmark harness (`docs/performance-strategy.md` §5,
//! `docs/phases/phase-11-performance.md`). See `tests/bench/harness.rs` for the measurement
//! mechanics and `tests/bench/scenarios.rs` for the world-building fixtures.
//!
//! Every one of the 13 current production instructions (`programs/aegis/src/lib.rs`'s `#[program]`
//! module -- `ping` is a Phase 1 toolchain-proof, not production, and is excluded; the Phase 12
//! governance/pause instructions in `docs/instruction-catalogue.md` do not exist in this program
//! yet and are correctly out of scope) is benchmarked here, entirely through real instructions
//! executed against a real `LiteSVM` world and then re-measured through Mollusk against the
//! genuine compiled `aegis.so`.
//!
//! **PERF-I6 runs first** (`perf_i6_liquidate_worst_case`), per the phase spec's explicit
//! ordering: before any other measurement or any optimization, `liquidate`'s worst realistic case
//! must be shown to fit the 200,000 CU default budget, because if it does not, that is a
//! correctness/resource-safety problem (T-27) rather than an ordinary tuning target. Test
//! functions below are alphabetized by Rust but Cargo does not guarantee run order across
//! `#[test]` functions in one binary -- `cu_benchmark_suite` is therefore the single source of
//! truth for `benchmarks/cu.json` and reruns every scenario itself rather than trying to share
//! state with `perf_i6_liquidate_worst_case`.

#[path = "bench/contention.rs"]
mod contention;
#[path = "bench/harness.rs"]
mod harness;
#[path = "bench/scenarios.rs"]
mod scenarios;

use aegis_test_kit::market::{
    absorb_bad_debt_ix, accrue_interest_ix, borrow_ix, close_position_ix, deposit_collateral_ix,
    liquidate_ix, repay_ix, supply_ix, withdraw_collateral_fees_ix, withdraw_collateral_ix,
    withdraw_ix,
};
use aegis_test_kit::{fetch_market, fetch_position, spl_token_2022_interface, spl_token_interface};
use mollusk_svm::Mollusk;
use scenarios::{Fixture, TokenVariant};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

/// Large enough that `borrow`'s free-liquidity check never binds in any scenario here.
const LENDER_SUPPLY: u64 = 1_000_000_000_000_000; // 1e9 USDC @ 6dp
const BORROWER_COLLATERAL: u64 = 1_000_000_000_000; // 1,000 SOL @ 9dp

fn send(fx: &mut Fixture, signer: &Keypair, ix: Instruction) {
    let blockhash = fx.svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &blockhash);
    let tx =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[signer]).expect("sign");
    fx.svm
        .send_transaction(tx)
        .expect("setup transaction must succeed");
}

fn send_priced(fx: &mut Fixture, signer: &Keypair, ix: Instruction, cu_limit: u32) {
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
    let blockhash = fx.svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[budget_ix, ix], Some(&signer.pubkey()), &blockhash);
    let tx =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[signer]).expect("sign");
    fx.svm
        .send_transaction(tx)
        .expect("setup transaction must succeed");
}

fn variant_label(v: TokenVariant) -> &'static str {
    match v {
        TokenVariant::ClassicSpl => "spl_token",
        TokenVariant::Token2022BothSides => "token_2022_both_sides",
    }
}

/// One row of `benchmarks/cu.json`.
#[derive(Debug, Clone)]
struct CuRecord {
    instruction: &'static str,
    scenario: &'static str,
    token_program: &'static str,
    cu: u64,
    accounts: usize,
}

fn record(
    mollusk: &mut Mollusk,
    fx: &Fixture,
    ix: &Instruction,
    instruction: &'static str,
    scenario: &'static str,
    variant: TokenVariant,
    out: &mut Vec<CuRecord>,
) -> u64 {
    eprintln!(
        "[bench] measuring {instruction}/{scenario}/{}",
        variant_label(variant)
    );
    let accounts = harness::snapshot_for(&fx.svm, ix);
    let ts = scenarios::now(&fx.svm);
    let cu = harness::measure_at(mollusk, ts, ix, &accounts);
    out.push(CuRecord {
        instruction,
        scenario,
        token_program: variant_label(variant),
        cu,
        accounts: ix.accounts.len(),
    });
    cu
}

fn record_na(
    mollusk: &mut Mollusk,
    fx: &Fixture,
    ix: &Instruction,
    instruction: &'static str,
    scenario: &'static str,
    out: &mut Vec<CuRecord>,
) -> u64 {
    eprintln!("[bench] measuring {instruction}/{scenario}/n_a");
    let accounts = harness::snapshot_for(&fx.svm, ix);
    let ts = scenarios::now(&fx.svm);
    let cu = harness::measure_at(mollusk, ts, ix, &accounts);
    out.push(CuRecord {
        instruction,
        scenario,
        token_program: "n/a",
        cu,
        accounts: ix.accounts.len(),
    });
    cu
}

/// Builds a market, a lender who supplies deep liquidity, and a borrower who deposits collateral
/// and borrows right up against `max_ltv` -- entirely through real instructions.
struct BorrowedWorld {
    fx: Fixture,
    borrower: Keypair,
    borrower_position: Pubkey,
    borrower_loan_ata: Pubkey,
    lender: Keypair,
    lender_position: Pubkey,
    lender_ata: Pubkey,
}

fn build_maximally_borrowed_position(variant: TokenVariant) -> BorrowedWorld {
    let mut fx = Fixture::new(variant, 0);

    let (lender, lender_ata) =
        fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
    let lender_position = fx.init_position_for(lender.pubkey());
    let ix = supply_ix(
        &lender.pubkey(),
        fx.market,
        lender_position,
        fx.fee_position,
        fx.loan_vault,
        lender_ata,
        fx.loan_mint,
        fx.loan_token_program,
        LENDER_SUPPLY,
        0,
    );
    send(&mut fx, &lender, ix);

    let (borrower, borrower_collateral_ata) = fx.wallet_with_ata(
        fx.collateral_mint,
        fx.collateral_token_program,
        BORROWER_COLLATERAL,
    );
    let borrower_position = fx.init_position_for(borrower.pubkey());
    let ix = deposit_collateral_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.collateral_vault,
        borrower_collateral_ata,
        fx.collateral_mint,
        fx.collateral_token_program,
        BORROWER_COLLATERAL,
    );
    send(&mut fx, &borrower, ix);

    let (c1, l1) = fx.valid_prices();
    let position_after_deposit = fetch_position(&fx.svm, &borrower_position);

    // Borrow right up against max_ltv (0.75) using the *credited* (post-transfer-fee) collateral
    // amount actually on deposit -- a real, maximal, oracle-validated borrow, not a round number.
    // collateral_value (USD, 1e8-scaled) = collateral_amount(9dp) * price(1e8-scaled) / 1e9
    let collateral_value_usd_1e8 = (position_after_deposit.collateral_amount as u128)
        * (scenarios::COLLATERAL_PRICE as u128)
        / 1_000_000_000u128;
    let max_ltv_wad = 750_000_000_000_000_000u128; // 0.75 WAD
    let borrowable_usd_1e8 = collateral_value_usd_1e8 * max_ltv_wad / 1_000_000_000_000_000_000u128;
    // loan price is exactly $1.00 (1e8-scaled) -> loan base units (6dp) = usd_1e8 / 1e8 * 1e6
    let borrowable_assets = (borrowable_usd_1e8 / 100u128) as u64;
    let borrow_assets = borrowable_assets * 95 / 100; // 5% margin under the exact boundary

    let borrower_loan_ata = fx.ata_for(borrower.pubkey(), fx.loan_mint, fx.loan_token_program, 0);
    let ix = borrow_ix(
        &borrower.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        borrower_loan_ata,
        fx.loan_mint,
        fx.loan_token_program,
        c1,
        l1,
        borrow_assets,
        0,
    );
    send_priced(&mut fx, &borrower, ix, 800_000);

    BorrowedWorld {
        fx,
        borrower,
        borrower_position,
        borrower_loan_ata,
        lender,
        lender_position,
        lender_ata,
    }
}

/// `B-CU-LIQUIDATE-WORST-CASE` (INV-RES-01).
#[test]
fn perf_i6_liquidate_worst_case() {
    // Worst realistic path per phase-11-performance.md #4: Token-2022 on BOTH sides (collateral
    // carries a live 2% transfer fee; the loan side is the only Token-2022 configuration
    // `create_market` accepts for a loan asset -- see `TokenVariant::Token2022BothSides`'s doc
    // comment), oracle validation on both feeds, full interest accrual, the liquidation-math clamp
    // branch (the more expensive of the two, per `aegis_test_kit::market`'s own doc comment on
    // `LIQUIDATE_COMPUTE_UNIT_LIMIT`), a real token transfer in both directions, and fee handling
    // (`collateral_fee_accrued`) -- built entirely through real `supply`/`deposit_collateral`/
    // `borrow` instructions, never fixture-injected debt.
    let BorrowedWorld {
        mut fx,
        borrower_position,
        ..
    } = build_maximally_borrowed_position(TokenVariant::Token2022BothSides);

    // Crash the collateral price hard enough that the position is deep underwater: HF well below
    // `full_liq_hf` (0.95), so a liquidator may repay up to 100% of the debt, AND the seize
    // (principal + bonus) implied by a full repay at the crashed price exceeds the position's
    // actual collateral balance -- forcing the collateral-clamp recomputation branch.
    let crashed_price = scenarios::COLLATERAL_PRICE / 20; // -95%
    let (c2, l2) = fx.distressed_prices(crashed_price);

    let market_before = fetch_market(&fx.svm, &fx.market);
    let position_before = fetch_position(&fx.svm, &borrower_position);
    eprintln!(
        "[perf-i6] pre-liquidation: borrow_shares={} collateral_amount={} total_borrow_assets={} \
         total_borrow_shares={}",
        position_before.borrow_shares,
        position_before.collateral_amount,
        market_before.total_borrow_assets,
        market_before.total_borrow_shares
    );

    let (liquidator, liquidator_loan_ata) =
        fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
    let (_, liquidator_collateral_ata) =
        fx.wallet_with_ata(fx.collateral_mint, fx.collateral_token_program, 0);

    // Request full repayment of the outstanding debt (permitted: HF is far below full_liq_hf).
    // This fixture has exactly one borrower, so `total_borrow_assets` IS the position's exact
    // outstanding debt -- the precise `max_repay` ceiling `liquidate` enforces under full
    // liquidation eligibility; anything above it is rejected outright (not clamped).
    let full_repay = fetch_market(&fx.svm, &fx.market).total_borrow_assets;

    let ix = liquidate_ix(
        &liquidator.pubkey(),
        fx.market,
        borrower_position,
        fx.fee_position,
        fx.loan_vault,
        fx.collateral_vault,
        liquidator_loan_ata,
        liquidator_collateral_ata,
        fx.loan_mint,
        fx.collateral_mint,
        fx.loan_token_program,
        fx.collateral_token_program,
        c2,
        l2,
        full_repay,
        0,
    );

    let mut mollusk = harness::new_mollusk();
    let accounts = harness::snapshot_for(&fx.svm, &ix);
    let ts = scenarios::now(&fx.svm);
    let cu = harness::measure_at(&mut mollusk, ts, &ix, &accounts);

    // Confirm this actually exercised the clamp path (collateral fully exhausted while a bonus
    // was still owed), by re-running the SAME instruction for real through LiteSVM and inspecting
    // the resulting position -- Mollusk's own execution above is stateless/throwaway, so the
    // clamp-path evidence has to come from an independent execution against the identical state.
    send_priced(&mut fx, &liquidator, ix, 800_000);
    let position_after = fetch_position(&fx.svm, &borrower_position);
    eprintln!(
        "[perf-i6] post-liquidation (independent LiteSVM re-execution): collateral_amount={} \
         borrow_shares={} (0 collateral with borrow_shares > 0 confirms the collateral-clamp path \
         fired AND left bad debt -- the bonus-inclusive seizure would have exceeded available \
         collateral)",
        position_after.collateral_amount, position_after.borrow_shares
    );
    assert_eq!(
        position_after.collateral_amount, 0,
        "PERF-I6 fixture must exercise the collateral-clamp branch, the more expensive of \
         liquidate's two liquidation-math paths -- got a non-zero remaining collateral_amount, \
         meaning the unclamped path fired instead; the price crash needs to be more severe"
    );

    eprintln!("[perf-i6] liquidate (Token-2022 both sides, worst case, no callback) CU = {cu}");
    assert!(
        cu < 200_000,
        "PERF-I6: worst-case liquidate consumed {cu} CU, which is >= the 200,000 CU default \
         budget -- this is a T-27 correctness/resource-safety problem, not an ordinary \
         optimization target (docs/phases/phase-11-performance.md #5)"
    );
}

/// Zero-account, no-op instruction: the floor of Anchor's own per-instruction dispatch overhead
/// (discriminator match + empty handler). Not itself a production benchmark row (`ping` is a
/// Phase 1 toolchain proof, not a production instruction) but useful context for every other
/// number in this file: any large CU figure elsewhere is real work, not framework overhead.
#[test]
fn perf_baseline_ping_zero_accounts() {
    use aegis::accounts::Ping as PingAccounts;
    use aegis::instruction::Ping as PingInstruction;
    use anchor_lang::{InstructionData, ToAccountMetas};
    let ix = Instruction {
        program_id: aegis::ID,
        accounts: PingAccounts {}.to_account_metas(None),
        data: PingInstruction {}.data(),
    };
    let mut mollusk = harness::new_mollusk();
    let cu = harness::measure_at(&mut mollusk, 0, &ix, &[]);
    eprintln!("[perf-baseline] ping (zero accounts, no-op) CU = {cu}");
}

/// Builds `create_market`'s own instruction (not yet sent) against a freshly initialized
/// protocol, for the two token variants -- the one instruction whose own vault/account creation
/// cost genuinely differs from a plain deposit/withdraw once Token-2022 extension sizing is
/// involved.
fn create_market_case(variant: TokenVariant) -> (litesvm::LiteSVM, Instruction) {
    use aegis_test_kit::market::{collateral_vault_pda, loan_vault_pda, market_pda, position_pda};
    use aegis_test_kit::{
        create_spl_mint, create_token_2022_mint, deploy, initialize_protocol,
        reference_market_args, Token2022Extension,
    };
    let program_id = aegis::id();
    let (mut svm, admin) = deploy(program_id, scenarios::program_bytes());
    let guardian = scenarios::fixed_pubkey(200);
    let fee_recipient = scenarios::fixed_pubkey(201);
    initialize_protocol(&mut svm, &admin, guardian, fee_recipient).expect("initialize_protocol");

    let (collateral_mint, loan_mint, collateral_token_program, loan_token_program) = match variant {
        TokenVariant::ClassicSpl => {
            let c = create_spl_mint(&mut svm, &admin, 202, 9, admin.pubkey(), None);
            let l = create_spl_mint(&mut svm, &admin, 203, 6, admin.pubkey(), None);
            (c, l, spl_token_interface::ID, spl_token_interface::ID)
        }
        TokenVariant::Token2022BothSides => {
            let c = create_token_2022_mint(
                &mut svm,
                &admin,
                202,
                9,
                admin.pubkey(),
                None,
                &[Token2022Extension::TransferFeeConfig {
                    basis_points: 200,
                    maximum_fee: u64::MAX,
                }],
            );
            let l = create_token_2022_mint(&mut svm, &admin, 203, 6, admin.pubkey(), None, &[]);
            (
                c,
                l,
                spl_token_2022_interface::ID,
                spl_token_2022_interface::ID,
            )
        }
    };

    let args = reference_market_args(
        0,
        scenarios::COLLATERAL_FEED_ID,
        scenarios::LOAN_FEED_ID,
        false,
    );
    let (protocol, _) = aegis_test_kit::market::protocol_pda();
    let (market, _) = market_pda(&collateral_mint, &loan_mint, args.config_id);
    let (collateral_vault, _) = collateral_vault_pda(&market);
    let (loan_vault, _) = loan_vault_pda(&market);
    let (fee_position, _) = position_pda(&market, &fee_recipient);
    use anchor_lang::solana_program::system_program;
    use anchor_lang::{InstructionData, ToAccountMetas};
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
    let _ = admin;
    (svm, ix)
}

/// The complete Phase 11 CU baseline: every one of the 13 current production instructions,
/// benchmarked for every applicable token-program variant and representative state
/// (`docs/phases/phase-11-performance.md` #7-9). Writes `benchmarks/cu.json` when
/// `AEGIS_BENCH_WRITE=1` is set in the environment (`make bench`); otherwise just measures and
/// prints the table, so `cargo test --workspace` never has the side effect of touching the
/// repository. Never panics on an over-threshold instruction -- every measurement is recorded
/// regardless, so one expensive instruction cannot hide the rest of the baseline; `PERF-I6`'s own
/// test enforces the 200k gate for `liquidate` specifically, and `scripts/check-cu-regression.sh`
/// enforces it (and the 10% regression bound) for every instruction against the committed file.
///
/// `B-CU-ALL` (INV-RES-01: "every user-facing instruction completes within the default 200k CU
/// budget, with a committed measurement").
#[test]
fn cu_benchmark_suite() {
    let mut mollusk = harness::new_mollusk();
    let mut out: Vec<CuRecord> = Vec::new();

    // --- initialize_protocol ---
    {
        let (svm, admin) = aegis_test_kit::deploy(aegis::id(), scenarios::program_bytes());
        let guardian = scenarios::fixed_pubkey(210);
        let fee_recipient = scenarios::fixed_pubkey(211);
        let ix = aegis_test_kit::market::initialize_protocol_ix(
            &admin.pubkey(),
            guardian,
            fee_recipient,
        );
        eprintln!("[bench] measuring initialize_protocol/fresh/n_a");
        let accounts = harness::snapshot_for(&svm, &ix);
        let cu = harness::measure_at(&mut mollusk, scenarios::now(&svm), &ix, &accounts);
        out.push(CuRecord {
            instruction: "initialize_protocol",
            scenario: "fresh",
            token_program: "n/a",
            cu,
            accounts: ix.accounts.len(),
        });
    }

    // --- create_market ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let (svm, ix) = create_market_case(variant);
        eprintln!(
            "[bench] measuring create_market/fresh/{}",
            variant_label(variant)
        );
        let accounts = harness::snapshot_for(&svm, &ix);
        let cu = harness::measure_at(&mut mollusk, scenarios::now(&svm), &ix, &accounts);
        out.push(CuRecord {
            instruction: "create_market",
            scenario: "fresh",
            token_program: variant_label(variant),
            cu,
            accounts: ix.accounts.len(),
        });
    }

    // --- init_position ---
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 1);
        let owner = scenarios::fixed_pubkey(220);
        let (ix, _) =
            aegis_test_kit::market::init_position_ix(&fx.admin.pubkey(), fx.market, owner);
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "init_position",
            "fresh_market",
            &mut out,
        );
    }

    // --- deposit_collateral: existing position, adding more collateral ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let mut fx = Fixture::new(variant, 2);
        let (depositor, ata) = fx.wallet_with_ata(
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL * 2,
        );
        let position = fx.init_position_for(depositor.pubkey());
        let ix = deposit_collateral_ix(
            &depositor.pubkey(),
            fx.market,
            position,
            fx.collateral_vault,
            ata,
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL,
        );
        send(&mut fx, &depositor, ix.clone());
        // Second deposit into the now-nonempty position -- the realistic steady-state case.
        let ix2 = deposit_collateral_ix(
            &depositor.pubkey(),
            fx.market,
            position,
            fx.collateral_vault,
            ata,
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL / 4,
        );
        record(
            &mut mollusk,
            &fx,
            &ix2,
            "deposit_collateral",
            "existing_position",
            variant,
            &mut out,
        );
    }

    // --- withdraw_collateral: no-debt (no oracle) vs with-debt (oracle+LTV) ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let mut fx = Fixture::new(variant, 3);
        let (owner, ata) = fx.wallet_with_ata(
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL,
        );
        let position = fx.init_position_for(owner.pubkey());
        let ix = deposit_collateral_ix(
            &owner.pubkey(),
            fx.market,
            position,
            fx.collateral_vault,
            ata,
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL,
        );
        send(&mut fx, &owner, ix);
        let (c, l) = fx.valid_prices();
        let withdraw_ix_no_debt = withdraw_collateral_ix(
            &owner.pubkey(),
            fx.market,
            position,
            fx.collateral_vault,
            ata,
            fx.collateral_mint,
            fx.collateral_token_program,
            c,
            l,
            1_000_000,
        );
        record(
            &mut mollusk,
            &fx,
            &withdraw_ix_no_debt,
            "withdraw_collateral",
            "no_debt",
            variant,
            &mut out,
        );

        if variant == TokenVariant::ClassicSpl {
            // With-debt path: only need one variant to characterize the oracle+LTV delta (the
            // token-program choice is orthogonal to whether the oracle path runs at all).
            let (lender, lender_ata) =
                fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
            let lender_position = fx.init_position_for(lender.pubkey());
            let supply_ix_ = supply_ix(
                &lender.pubkey(),
                fx.market,
                lender_position,
                fx.fee_position,
                fx.loan_vault,
                lender_ata,
                fx.loan_mint,
                fx.loan_token_program,
                LENDER_SUPPLY,
                0,
            );
            send(&mut fx, &lender, supply_ix_);
            let borrower_loan_ata =
                fx.ata_for(owner.pubkey(), fx.loan_mint, fx.loan_token_program, 0);
            let small_borrow = 1_000_000_000u64; // well under max_ltv, leaves room to also withdraw a bit
            let borrow_ix_ = borrow_ix(
                &owner.pubkey(),
                fx.market,
                position,
                fx.fee_position,
                fx.loan_vault,
                borrower_loan_ata,
                fx.loan_mint,
                fx.loan_token_program,
                c,
                l,
                small_borrow,
                0,
            );
            send_priced(&mut fx, &owner, borrow_ix_, 800_000);
            let (c2, l2) = fx.valid_prices();
            let withdraw_ix_with_debt = withdraw_collateral_ix(
                &owner.pubkey(),
                fx.market,
                position,
                fx.collateral_vault,
                ata,
                fx.collateral_mint,
                fx.collateral_token_program,
                c2,
                l2,
                1_000_000,
            );
            record(
                &mut mollusk,
                &fx,
                &withdraw_ix_with_debt,
                "withdraw_collateral",
                "with_debt_oracle_ltv",
                variant,
                &mut out,
            );
        }
    }

    // --- close_position: freshly initialized, all-zero balances ---
    {
        let mut fx = Fixture::new(TokenVariant::ClassicSpl, 4);
        let owner = Keypair::new_from_array([230u8; 32]);
        fx.svm
            .airdrop(&owner.pubkey(), 10_000_000_000)
            .expect("airdrop");
        let position = fx.init_position_for(owner.pubkey());
        let ix = close_position_ix(&owner.pubkey(), fx.market, position);
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "close_position",
            "empty_position",
            &mut out,
        );
    }

    // --- supply / withdraw: SPL vs Token-2022 loan side, with prior accrual ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let BorrowedWorld {
            mut fx,
            lender,
            lender_position,
            lender_ata,
            ..
        } = build_maximally_borrowed_position(variant);
        fx.warp_seconds(3600 * 24); // one day of accrued interest before the next supply/withdraw
        let (lender2, lender2_ata) =
            fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
        let lender2_position = fx.init_position_for(lender2.pubkey());
        let ix = supply_ix(
            &lender2.pubkey(),
            fx.market,
            lender2_position,
            fx.fee_position,
            fx.loan_vault,
            lender2_ata,
            fx.loan_mint,
            fx.loan_token_program,
            1_000_000_000,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &ix,
            "supply",
            "existing_market_with_accrued_interest",
            variant,
            &mut out,
        );

        // The original lender (the position's owner) withdraws a small amount of their supply.
        let ix = withdraw_ix(
            &lender.pubkey(),
            fx.market,
            lender_position,
            fx.fee_position,
            fx.loan_vault,
            lender_ata,
            fx.loan_mint,
            fx.loan_token_program,
            1_000_000,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &ix,
            "withdraw",
            "existing_market_with_accrued_interest",
            variant,
            &mut out,
        );
    }

    // --- borrow ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let mut fx = Fixture::new(variant, 5);
        let (lender, lender_ata) =
            fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
        let lender_position = fx.init_position_for(lender.pubkey());
        let ix = supply_ix(
            &lender.pubkey(),
            fx.market,
            lender_position,
            fx.fee_position,
            fx.loan_vault,
            lender_ata,
            fx.loan_mint,
            fx.loan_token_program,
            LENDER_SUPPLY,
            0,
        );
        send(&mut fx, &lender, ix);
        let (borrower, borrower_collateral_ata) = fx.wallet_with_ata(
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL,
        );
        let position = fx.init_position_for(borrower.pubkey());
        let ix = deposit_collateral_ix(
            &borrower.pubkey(),
            fx.market,
            position,
            fx.collateral_vault,
            borrower_collateral_ata,
            fx.collateral_mint,
            fx.collateral_token_program,
            BORROWER_COLLATERAL,
        );
        send(&mut fx, &borrower, ix);
        let (c, l) = fx.valid_prices();
        let borrower_loan_ata =
            fx.ata_for(borrower.pubkey(), fx.loan_mint, fx.loan_token_program, 0);
        let ix = borrow_ix(
            &borrower.pubkey(),
            fx.market,
            position,
            fx.fee_position,
            fx.loan_vault,
            borrower_loan_ata,
            fx.loan_mint,
            fx.loan_token_program,
            c,
            l,
            100_000_000_000,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &ix,
            "borrow",
            "healthy_borrow_against_fresh_collateral",
            variant,
            &mut out,
        );
    }

    // --- repay: dt=0 vs dt>0, SPL vs Token-2022 ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        let BorrowedWorld {
            mut fx,
            borrower,
            borrower_loan_ata,
            ..
        } = build_maximally_borrowed_position(variant);
        let debt = fetch_market(&fx.svm, &fx.market).total_borrow_assets;
        let (borrower_position2, _) =
            aegis_test_kit::market::position_pda(&fx.market, &borrower.pubkey());
        let repay_ix_dt0 = repay_ix(
            &borrower.pubkey(),
            fx.market,
            borrower_position2,
            fx.fee_position,
            fx.loan_vault,
            borrower_loan_ata,
            fx.loan_mint,
            fx.loan_token_program,
            debt / 10,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &repay_ix_dt0,
            "repay",
            "dt0_partial",
            variant,
            &mut out,
        );

        fx.warp_seconds(3600 * 24 * 30);
        let repay_ix_dt30 = repay_ix(
            &borrower.pubkey(),
            fx.market,
            borrower_position2,
            fx.fee_position,
            fx.loan_vault,
            borrower_loan_ata,
            fx.loan_mint,
            fx.loan_token_program,
            debt / 10,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &repay_ix_dt30,
            "repay",
            "dt_30d_partial",
            variant,
            &mut out,
        );
    }

    // --- accrue_interest: dt=0 vs dt>0 (token-agnostic: touches no token accounts) ---
    {
        let BorrowedWorld { mut fx, .. } =
            build_maximally_borrowed_position(TokenVariant::ClassicSpl);
        let ix_dt0 = accrue_interest_ix(fx.market, fx.fee_position);
        record_na(
            &mut mollusk,
            &fx,
            &ix_dt0,
            "accrue_interest",
            "dt0_noop",
            &mut out,
        );
        fx.warp_seconds(3600 * 24 * 30);
        let ix_dt30 = accrue_interest_ix(fx.market, fx.fee_position);
        record_na(
            &mut mollusk,
            &fx,
            &ix_dt30,
            "accrue_interest",
            "dt_30d",
            &mut out,
        );
    }

    // --- liquidate: unclamped (close-factor partial) and clamped (full, worst case), both variants ---
    for variant in [TokenVariant::ClassicSpl, TokenVariant::Token2022BothSides] {
        // Unclamped: mild crash, close_factor-limited partial repay.
        let BorrowedWorld {
            mut fx,
            borrower_position,
            ..
        } = build_maximally_borrowed_position(variant);
        let crashed = scenarios::COLLATERAL_PRICE * 85 / 100; // -15%
        let (c, l) = fx.distressed_prices(crashed);
        let debt = fetch_market(&fx.svm, &fx.market).total_borrow_assets;
        let (liquidator, liquidator_loan_ata) =
            fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
        let (_, liquidator_collateral_ata) =
            fx.wallet_with_ata(fx.collateral_mint, fx.collateral_token_program, 0);
        let ix = liquidate_ix(
            &liquidator.pubkey(),
            fx.market,
            borrower_position,
            fx.fee_position,
            fx.loan_vault,
            fx.collateral_vault,
            liquidator_loan_ata,
            liquidator_collateral_ata,
            fx.loan_mint,
            fx.collateral_mint,
            fx.loan_token_program,
            fx.collateral_token_program,
            c,
            l,
            debt / 2,
            0,
        );
        record(
            &mut mollusk,
            &fx,
            &ix,
            "liquidate",
            "unclamped_close_factor_partial",
            variant,
            &mut out,
        );

        // Clamped: severe crash, full repay request, collateral-clamp branch (PERF-I6's own worst
        // case for the Token-2022-both-sides variant).
        let BorrowedWorld {
            mut fx,
            borrower_position,
            ..
        } = build_maximally_borrowed_position(variant);
        let crashed = scenarios::COLLATERAL_PRICE / 20; // -95%
        let (c, l) = fx.distressed_prices(crashed);
        let debt = fetch_market(&fx.svm, &fx.market).total_borrow_assets;
        let (liquidator, liquidator_loan_ata) =
            fx.wallet_with_ata(fx.loan_mint, fx.loan_token_program, LENDER_SUPPLY);
        let (_, liquidator_collateral_ata) =
            fx.wallet_with_ata(fx.collateral_mint, fx.collateral_token_program, 0);
        let ix = liquidate_ix(
            &liquidator.pubkey(),
            fx.market,
            borrower_position,
            fx.fee_position,
            fx.loan_vault,
            fx.collateral_vault,
            liquidator_loan_ata,
            liquidator_collateral_ata,
            fx.loan_mint,
            fx.collateral_mint,
            fx.loan_token_program,
            fx.collateral_token_program,
            c,
            l,
            debt,
            0,
        );
        let scenario = if variant == TokenVariant::Token2022BothSides {
            "clamped_full_worst_case"
        } else {
            "clamped_full"
        };
        record(
            &mut mollusk,
            &fx,
            &ix,
            "liquidate",
            scenario,
            variant,
            &mut out,
        );

        if variant == TokenVariant::ClassicSpl {
            // absorb_bad_debt reuses this exact post-liquidation bad-debt state (collateral == 0,
            // borrow_shares > 0) -- the natural byproduct of a clamped full liquidation.
            send_priced(&mut fx, &liquidator, ix.clone(), 800_000);
            let after = fetch_position(&fx.svm, &borrower_position);
            assert_eq!(after.collateral_amount, 0, "bad-debt fixture precondition");
            assert!(after.borrow_shares > 0, "bad-debt fixture precondition");
            let bad_debt_ix = absorb_bad_debt_ix(fx.market, borrower_position, fx.fee_position);
            record_na(
                &mut mollusk,
                &fx,
                &bad_debt_ix,
                "absorb_bad_debt",
                "post_full_liquidation_dust",
                &mut out,
            );
        } else {
            // withdraw_collateral_fees reuses the protocol_cut this liquidation generated.
            send_priced(&mut fx, &liquidator, ix.clone(), 800_000);
        }
        let market_state = fetch_market(&fx.svm, &fx.market);
        if market_state.collateral_fee_accrued > 0 {
            let (_, admin_collateral_ata) =
                fx.wallet_with_ata(fx.collateral_mint, fx.collateral_token_program, 0);
            let wcf_ix = withdraw_collateral_fees_ix(
                &fx.admin.pubkey(),
                fx.market,
                fx.collateral_vault,
                admin_collateral_ata,
                fx.collateral_mint,
                fx.collateral_token_program,
                market_state.collateral_fee_accrued,
            );
            record(
                &mut mollusk,
                &fx,
                &wcf_ix,
                "withdraw_collateral_fees",
                "full_accrued_balance",
                variant,
                &mut out,
            );
        }
    }

    // --- Phase 12: governance/admin instructions. All admin/guardian-gated, no oracle, no token
    // CPI -- benchmarked once each (token_program "n/a") rather than per-TokenVariant, since none
    // of these instructions touch a token account at all. ---
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 90);
        let new_admin = scenarios::fixed_pubkey(230);
        let ix = aegis_test_kit::market::set_pending_admin_ix(&fx.admin.pubkey(), new_admin);
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "set_pending_admin",
            "fresh_protocol",
            &mut out,
        );
    }
    {
        let mut fx = Fixture::new(TokenVariant::ClassicSpl, 91);
        let admin_kp = fx.admin.insecure_clone();
        let pending_admin = Keypair::new_from_array([231u8; 32]);
        let set_ix = aegis_test_kit::market::set_pending_admin_ix(
            &fx.admin.pubkey(),
            pending_admin.pubkey(),
        );
        send(&mut fx, &admin_kp, set_ix);
        let ix = aegis_test_kit::market::accept_admin_ix(&pending_admin.pubkey());
        eprintln!("[bench] measuring accept_admin/fresh_protocol/n_a");
        let accounts = harness::snapshot_for(&fx.svm, &ix);
        let ts = scenarios::now(&fx.svm);
        let cu = harness::measure_at(&mut mollusk, ts, &ix, &accounts);
        out.push(CuRecord {
            instruction: "accept_admin",
            scenario: "fresh_protocol",
            token_program: "n/a",
            cu,
            accounts: ix.accounts.len(),
        });
    }
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 92);
        let new_guardian = scenarios::fixed_pubkey(232);
        let ix = aegis_test_kit::market::set_guardian_ix(&fx.admin.pubkey(), new_guardian);
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "set_guardian",
            "fresh_protocol",
            &mut out,
        );
    }
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 93);
        let ix = aegis_test_kit::market::set_protocol_pause_ix(
            &fx.admin.pubkey(),
            aegis::constants::PAUSE_SUPPLY,
        );
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "set_protocol_pause",
            "admin_sets_one_bit",
            &mut out,
        );
    }
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 94);
        let ix = aegis_test_kit::market::set_market_pause_ix(
            &fx.admin.pubkey(),
            fx.market,
            aegis::constants::PAUSE_SUPPLY,
        );
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "set_market_pause",
            "admin_sets_one_bit",
            &mut out,
        );
    }
    {
        let fx = Fixture::new(TokenVariant::ClassicSpl, 95);
        let mut args = scenarios::reference_set_market_params_args(fx.fee_recipient);
        args.max_ltv -= 1; // tightening: applies immediately, no PendingMarketParams created
        let ix = aegis_test_kit::market::set_market_params_ix(
            &fx.admin.pubkey(),
            fx.market,
            fx.fee_position,
            None,
            args,
        );
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "set_market_params",
            "tighten_immediate",
            &mut out,
        );
    }
    {
        let mut fx = Fixture::new(TokenVariant::ClassicSpl, 96);
        let admin_kp = fx.admin.insecure_clone();
        let mut loosen_args = scenarios::reference_set_market_params_args(fx.fee_recipient);
        loosen_args.max_ltv += 1; // loosening: stages a PendingMarketParams
        let stage_ix = aegis_test_kit::market::set_market_params_ix(
            &fx.admin.pubkey(),
            fx.market,
            fx.fee_position,
            None,
            loosen_args,
        );
        send_priced(&mut fx, &admin_kp, stage_ix, 400_000);
        let mut clock = fx.svm.get_sysvar::<solana_clock::Clock>();
        clock.unix_timestamp += aegis::constants::PARAM_TIMELOCK_SECS;
        fx.svm.set_sysvar(&clock);
        let ix = aegis_test_kit::market::commit_pending_params_ix(
            &fx.admin.pubkey(),
            fx.admin.pubkey(),
            fx.market,
            fx.fee_position,
        );
        record_na(
            &mut mollusk,
            &fx,
            &ix,
            "commit_pending_params",
            "at_effective_at",
            &mut out,
        );
    }
    {
        // migrate_protocol_v2 measured against a real, injected ProtocolV1 account -- the same
        // legitimate fixture technique tests/phase12_migration.rs uses, since no real transaction
        // in this already-Phase-12 program can produce a ProtocolV1 account any other way.
        let (mut svm, admin) = aegis_test_kit::deploy(aegis::id(), scenarios::program_bytes());
        let (protocol_pubkey, bump) = aegis_test_kit::protocol_pda();
        let v1 = aegis::state::ProtocolV1 {
            admin: admin.pubkey(),
            pending_admin: Pubkey::default(),
            guardian: scenarios::fixed_pubkey(240),
            fee_recipient: scenarios::fixed_pubkey(241),
            paused: 0,
            bump,
            _reserved: [0u8; 64],
        };
        let mut data = Vec::new();
        anchor_lang::AccountSerialize::try_serialize(&v1, &mut data).unwrap();
        svm.set_account(
            protocol_pubkey,
            solana_account::Account {
                lamports: svm.minimum_balance_for_rent_exemption(data.len()),
                data,
                owner: aegis::id(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
        let ix = aegis_test_kit::market::migrate_protocol_v2_ix(&admin.pubkey());
        eprintln!("[bench] measuring migrate_protocol_v2/v1_account/n_a");
        let accounts = harness::snapshot_for(&svm, &ix);
        let ts = scenarios::now(&svm);
        let cu = harness::measure_at(&mut mollusk, ts, &ix, &accounts);
        out.push(CuRecord {
            instruction: "migrate_protocol_v2",
            scenario: "v1_account",
            token_program: "n/a",
            cu,
            accounts: ix.accounts.len(),
        });
    }

    let over_budget_count = print_and_maybe_write(&out);
    assert_eq!(
        over_budget_count, 0,
        "B-CU-ALL / INV-RES-01: {over_budget_count} scenario(s) at/over the 200,000 CU budget -- \
         see the table above for exactly which"
    );
}

/// Returns the number of scenarios at or over the 200,000 CU budget.
fn print_and_maybe_write(records: &[CuRecord]) -> usize {
    eprintln!("\n=== Phase 11 CU benchmark suite ===");
    eprintln!(
        "{:<28} {:<32} {:<24} {:>10} {:>10}",
        "instruction", "scenario", "token_program", "cu", "accounts"
    );
    for r in records {
        eprintln!(
            "{:<28} {:<32} {:<24} {:>10} {:>10}",
            r.instruction, r.scenario, r.token_program, r.cu, r.accounts
        );
    }
    let over_budget: Vec<&CuRecord> = records.iter().filter(|r| r.cu >= 200_000).collect();
    if !over_budget.is_empty() {
        eprintln!(
            "\n!!! {} scenario(s) at/over the 200,000 CU budget:",
            over_budget.len()
        );
        for r in &over_budget {
            eprintln!(
                "  {} / {} / {}: {} CU",
                r.instruction, r.scenario, r.token_program, r.cu
            );
        }
    }

    if std::env::var("AEGIS_BENCH_WRITE").as_deref() == Ok("1") {
        let commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let measurements: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "instruction": r.instruction,
                    "scenario": r.scenario,
                    "token_program": r.token_program,
                    "cu": r.cu,
                    "accounts": r.accounts,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "schema_version": 1,
            "meta": {
                "commit": commit,
                "rustc": rustc_version(),
                "solana_cli": "3.1.10",
                "anchor_cli": "1.2.0",
                "mollusk_svm": "0.15.1",
            },
            "measurements": measurements,
        });
        // `AEGIS_BENCH_OUTPUT` lets the CI regression script (`scripts/check-cu-regression.sh`)
        // capture a fresh measurement into a scratch file without overwriting the committed
        // baseline it is comparing against.
        let path = std::env::var("AEGIS_BENCH_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/cu.json")
            });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n")
            .expect("write CU benchmark output");
        eprintln!("\nwrote {}", path.display());
    }

    over_budget.len()
}

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
