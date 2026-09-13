//! Phase 11 — the Mollusk CU-measurement harness (`docs/performance-strategy.md` §5,
//! `docs/phases/phase-11-performance.md`).
//!
//! Design: every scenario builds its pre-state through REAL instructions in an in-process
//! `LiteSVM` (reusing the exact, already-tested `aegis_test_kit`/`aegis_test_kit::market` builders
//! Phases 2-10 use), so account bytes reaching the benchmark are byte-identical to what a real
//! transaction history produces -- never a hand-guessed struct literal. Immediately before the
//! instruction under measurement, every account the instruction references is snapshotted out of
//! that `LiteSVM` world (`snapshot_for`) and handed to a `Mollusk` instance, which executes the
//! REAL compiled `aegis.so` through the actual BPF loader (the same artifact `cargo test` already
//! validates) and reports its own `compute_units_consumed` -- LiteSVM does not expose that number,
//! which is the whole reason this harness exists alongside it rather than instead of it.
//!
//! One `Mollusk` is built per benchmark run (`new_mollusk`) and reused sequentially across every
//! scenario in that run (this harness has no concurrency; `bench.rs` drives one sequential list of
//! measurements) -- only `sysvars.clock.unix_timestamp` changes between calls, reset explicitly
//! every time by [`measure_at`] so scenarios never leak clock state into each other.

use aegis_test_kit::{spl_token_2022_interface, spl_token_interface};
use anchor_lang::solana_program::system_program;
use mollusk_svm::result::ProgramResult;
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Builds a fresh Mollusk instance with the real compiled `aegis.so` (the identical bytes
/// `tests/smoke.rs` and every other integration test load) plus the real SPL Token and
/// Token-2022 program ELFs, so CPIs execute against the genuine token programs.
pub fn new_mollusk() -> Mollusk {
    let mut mollusk = Mollusk::default();
    let program_bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"));
    mollusk.add_program_with_loader_and_elf(
        &aegis::ID,
        &mollusk_svm::program::loader_keys::LOADER_V3,
        program_bytes,
    );
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk_svm_programs_token::token2022::add_program(&mut mollusk);
    mollusk
}

/// The stub `Account` row Mollusk needs for a program pubkey referenced as a plain `AccountMeta`
/// (as opposed to the top-level program being invoked, which Mollusk fills in automatically) --
/// e.g. `system_program`/`collateral_token_program` passed into an Aegis instruction so its own
/// CPI can find them in the transaction context.
fn program_stub(program_id: &Pubkey) -> Option<Account> {
    if *program_id == system_program::ID {
        return Some(mollusk_svm::program::keyed_account_for_system_program().1);
    }
    if *program_id == spl_token_interface::ID {
        return Some(mollusk_svm_programs_token::token::account());
    }
    if *program_id == spl_token_2022_interface::ID {
        return Some(mollusk_svm_programs_token::token2022::account());
    }
    None
}

/// Snapshots every account `ix` references directly out of `svm` -- the exact bytes a real
/// transaction executed against this world would see. Programs (`system_program`, the token
/// programs) are substituted with Mollusk's own bundled program accounts rather than read from
/// `svm`, since what matters for CPI is Mollusk's program cache, not LiteSVM's representation. An
/// account that does not yet exist in `svm` (e.g. the target of an Anchor `init` constraint --
/// `init_position`'s new `Position`, `create_market`'s new `Market`/vaults/fee position,
/// `initialize_protocol`'s `Protocol`) is represented the same way a genuinely nonexistent account
/// looks on-chain: zero lamports, empty data, owned by the System Program
/// (`Account::default()`'s owner is the all-zero pubkey, which IS the System Program's address) --
/// never a panic, since benchmarking an instruction's OWN account-creation cost is a legitimate
/// scenario, not a fixture bug.
pub fn snapshot_for(svm: &litesvm::LiteSVM, ix: &Instruction) -> Vec<(Pubkey, Account)> {
    ix.accounts
        .iter()
        .map(|meta| {
            let pubkey = meta.pubkey;
            let account = program_stub(&pubkey)
                .or_else(|| svm.get_account(&pubkey))
                .unwrap_or_default();
            (pubkey, account)
        })
        .collect()
}

/// Runs `ix` against `accounts` through the real compiled program via Mollusk, at the given clock
/// (unix seconds -- matching the world's own clock keeps accrual/oracle-staleness math consistent
/// with whatever the `LiteSVM` setup already assumed), and returns the compute units consumed.
/// Panics with the program's raw result if the instruction did not succeed: a benchmark that
/// silently measures a *failed* instruction's CU would be worthless and misleading.
pub fn measure_at(
    mollusk: &mut Mollusk,
    unix_timestamp: i64,
    ix: &Instruction,
    accounts: &[(Pubkey, Account)],
) -> u64 {
    mollusk.sysvars.clock.unix_timestamp = unix_timestamp;
    let result = mollusk.process_instruction(ix, accounts);
    match result.program_result {
        ProgramResult::Success => result.compute_units_consumed,
        other => panic!(
            "bench: instruction did not succeed (program_result={other:?}, raw_result={:?}); \
             a benchmark must only ever measure a successful execution",
            result.raw_result
        ),
    }
}
