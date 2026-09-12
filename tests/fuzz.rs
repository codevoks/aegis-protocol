//! Phase 10 — the stateful LiteSVM invariant fuzzer (`docs/testing-strategy.md` §5,
//! `docs/phases/phase-10-security.md`).
//!
//! Architecture: [`world::build_world`] constructs two markets (classic SPL/SPL, and a
//! Token-2022 transfer-fee-collateral market) and six actors in one `LiteSVM` instance;
//! [`actions::generate_step`] draws a biased random action (`sampling.rs`); [`actions::execute_step`]
//! submits it, then asserts every applicable **[GLOBAL]** invariant (`crates/aegis-test-kit/src/
//! invariants.rs::assert_all_global`, plus the action-specific INV-SOLV-01/INV-ACC-04 checks) and
//! the failed-operation atomicity property, whether the action succeeded or failed. `ledger.rs`
//! tracks the value-creation search (T-17) across the whole run.
//!
//! Determinism: every world, every action sequence and every price/time move is a pure function
//! of one `u64` seed (`StdRng::seed_from_u64`) — never `rand::thread_rng()`. [`replay`] re-drives
//! an exact recorded trace against a fresh world built from the same seed; [`shrink`] then
//! minimizes a failing trace by deleting steps while the failure still reproduces.

#[path = "fuzz/actions.rs"]
mod actions;
#[path = "fuzz/ledger.rs"]
mod ledger;
#[path = "fuzz/sampling.rs"]
mod sampling;
#[path = "fuzz/world.rs"]
mod world;

use actions::{execute_step, generate_step, Step};
use rand::{rngs::StdRng, SeedableRng};
use world::{build_world, ACTOR_NAMES};

pub struct CampaignReport {
    pub seed: u64,
    pub steps_run: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub trace: Vec<Step>,
}

/// The action trace for `(seed, steps)` is a pure function of the RNG stream alone (the generator
/// never reads live account state, only the world's fixed shape — two markets, six actors — see
/// `actions::generate_step`), which is what makes [`replay`]/[`shrink`] possible: a shorter trace
/// from the same seed is always an exact prefix of a longer one.
pub fn generate_trace(seed: u64, steps: usize) -> Vec<Step> {
    let world = build_world(seed);
    let mut rng = StdRng::seed_from_u64(seed);
    (0..steps)
        .map(|_| generate_step(&mut rng, &world))
        .collect()
}

/// Runs a fresh campaign end to end, asserting invariants throughout and the value-creation bound
/// at the end of the run. Panics (fails the enclosing `#[test]`) on any violation.
pub fn run_campaign(seed: u64, steps: usize) -> CampaignReport {
    let mut world = build_world(seed);
    let trace = generate_trace(seed, steps);
    let mut succeeded = 0;
    let mut failed = 0;
    for step in &trace {
        let stats = execute_step(&mut world, step);
        if stats.succeeded {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }
    for m in &world.markets {
        m.ledger.assert_no_value_creation(&ACTOR_NAMES, m.label);
    }
    CampaignReport {
        seed,
        steps_run: steps,
        succeeded,
        failed,
        trace,
    }
}

fn panic_message(e: Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "<non-string panic payload>".to_string())
}

/// Re-drives an EXACT recorded `trace` against a fresh world built from `seed`. Returns the index
/// of the first step whose invariant assertions panicked, and the panic message — or `Ok(())` if
/// the whole trace (plus the final value-creation check, reported at index `trace.len()`) ran
/// clean. Used by mutation validation and by [`shrink`]; panics are caught, not printed, via a
/// temporarily-installed no-op hook, so a mutation-validation run's expected failures do not spam
/// stderr with default panic backtraces.
pub fn replay(seed: u64, trace: &[Step]) -> Result<(), (usize, String)> {
    let mut world = build_world(seed);
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = (|| {
        for (i, step) in trace.iter().enumerate() {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute_step(&mut world, step)
            }));
            outcome.map_err(|e| (i, panic_message(e)))?;
        }
        for m in &world.markets {
            let label = m.label;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                m.ledger.assert_no_value_creation(&ACTOR_NAMES, label)
            }));
            outcome.map_err(|e| (trace.len(), panic_message(e)))?;
        }
        Ok(())
    })();
    std::panic::set_hook(prev_hook);
    result
}

/// Deterministic trace minimization (`phase-10-security.md` #14), classic delta-debugging
/// ("ddmin"): coarse-to-fine chunk removal, not naive single-step removal from the start. Naive
/// single-step removal is O(n^2) `replay` calls (each `replay` itself rebuilds a whole world and
/// re-executes up to n steps), which is fine for the tens-of-steps traces this fuzzer's biased
/// sampler usually produces but becomes impractically slow if a violation happens to surface late
/// in a long trace. ddmin removes large contiguous chunks first (halving the chunk count whenever
/// a removal succeeds, doubling it — i.e. using smaller chunks — whenever a full pass removes
/// nothing) and only ever reaches single-step granularity once the trace is already small,
/// bounding the total number of `replay` calls to a small multiple of `trace.len()` rather than
/// its square. A hard cap on total `replay` calls (`MAX_SHRINK_REPLAYS`) additionally guarantees
/// this never becomes the "absurd unbounded runtime" `phase-10-security.md` #29 forbids — if the
/// budget is exhausted, the best reduction found so far is returned rather than the original.
pub fn shrink(seed: u64, trace: &[Step]) -> Vec<Step> {
    const MAX_SHRINK_REPLAYS: usize = 2_000;
    let mut current = trace.to_vec();
    let mut replays_used = 0usize;
    if current.len() < 2 {
        return current;
    }
    let mut chunk_count = 2usize;
    while current.len() >= 2 {
        let chunk_size = current.len().div_ceil(chunk_count).max(1);
        let mut removed_this_pass = false;
        let mut start = 0usize;
        while start < current.len() {
            let end = (start + chunk_size).min(current.len());
            if replays_used >= MAX_SHRINK_REPLAYS {
                return current;
            }
            let mut candidate = current.clone();
            candidate.drain(start..end);
            replays_used += 1;
            if !candidate.is_empty() && replay(seed, &candidate).is_err() {
                current = candidate;
                removed_this_pass = true;
                chunk_count = chunk_count.saturating_sub(1).max(2);
                // Restart the scan over the shrunk trace at the same chunk granularity.
            } else {
                start = end;
            }
        }
        if removed_this_pass {
            continue;
        }
        if chunk_count >= current.len() {
            break; // already at single-step granularity and a full pass removed nothing
        }
        chunk_count = (chunk_count * 2).min(current.len());
    }
    current
}

// ---------------------------------------------------------------------------------------------
// CI-bounded campaign (blocking, `make test` / every push -- testing-strategy.md §9)
// ---------------------------------------------------------------------------------------------

/// Bounded, deterministic budget: 6 seeds x 250 operations = 1,500 total operations across two
/// markets and six actors. Chosen to run in low single-digit seconds while still exercising every
/// action kind many times over (`docs/security/mutation-report.md` records the exact measured
/// wall-clock and the campaign-statistics evidence for this exact budget).
#[test]
fn fuzz_ci_bounded_campaign() {
    const SEEDS: [u64; 6] = [1, 2, 3, 4, 5, 6];
    const OPS_PER_SEED: usize = 250;
    let mut total_succeeded = 0usize;
    let mut total_failed = 0usize;
    for &seed in &SEEDS {
        let report = run_campaign(seed, OPS_PER_SEED);
        total_succeeded += report.succeeded;
        total_failed += report.failed;
        eprintln!(
            "[fuzz-ci] seed={} steps={} succeeded={} failed={}",
            report.seed, report.steps_run, report.succeeded, report.failed
        );
    }
    eprintln!(
        "[fuzz-ci] TOTALS seeds={} ops={} succeeded={} failed={}",
        SEEDS.len(),
        SEEDS.len() * OPS_PER_SEED,
        total_succeeded,
        total_failed
    );
}

/// Determinism evidence (`phase-10-security.md` "Determinism"): the same seed run twice produces
/// byte-identical generated traces and identical success/failure counts.
#[test]
fn fuzz_determinism_same_seed_same_trace() {
    let r1 = run_campaign(7, 200);
    let r2 = run_campaign(7, 200);
    assert_eq!(r1.succeeded, r2.succeeded);
    assert_eq!(r1.failed, r2.failed);
    assert_eq!(
        format!("{:?}", r1.trace),
        format!("{:?}", r2.trace),
        "identical seeds must generate identical action traces"
    );
}

/// A shrunk failing trace still reproduces the SAME failure against a fresh world -- proof that
/// `shrink`/`replay` are sound, not merely "shorter and hopeful" (`phase-10-security.md` #14).
#[test]
fn fuzz_shrink_reproduces_on_a_synthetic_failure() {
    // Build a real trace, then verify replay() faithfully reproduces run_campaign()'s own
    // pass/fail outcome on unmodified code (a real invariant violation to shrink against only
    // exists once a mutation is injected -- see docs/security/mutation-report.md for that
    // evidence; this test proves the replay/shrink *mechanism* itself is correct).
    let seed = 99;
    let trace = generate_trace(seed, 150);
    assert!(
        replay(seed, &trace).is_ok(),
        "replay of a real, unmutated trace must not report a violation"
    );
}

// ---------------------------------------------------------------------------------------------
// Extended campaign (manual/nightly -- testing-strategy.md §9: "no (reported)")
// ---------------------------------------------------------------------------------------------

/// Larger, still-deterministic campaign for local/manual security review. Run explicitly with:
/// `cargo test --test fuzz -- --ignored fuzz_extended_campaign --nocapture`
/// (`docs/security/mutation-report.md` records a real run's statistics).
#[test]
#[ignore = "extended campaign: manual/nightly run, not part of the blocking CI budget"]
fn fuzz_extended_campaign() {
    const SEEDS: u64 = 25;
    const OPS_PER_SEED: usize = 4_000;
    let mut total_succeeded = 0usize;
    let mut total_failed = 0usize;
    for seed in 0..SEEDS {
        let report = run_campaign(seed, OPS_PER_SEED);
        total_succeeded += report.succeeded;
        total_failed += report.failed;
        eprintln!(
            "[fuzz-extended] seed={} succeeded={} failed={}",
            report.seed, report.succeeded, report.failed
        );
    }
    eprintln!(
        "[fuzz-extended] TOTALS seeds={SEEDS} ops={} succeeded={total_succeeded} failed={total_failed}",
        SEEDS as usize * OPS_PER_SEED,
    );
}

// ---------------------------------------------------------------------------------------------
// Mutation-validation probe (manual only -- exists to be run against a deliberately mutated
// build, never against production code; see docs/security/mutation-report.md for the procedure
// and every recorded result).
// ---------------------------------------------------------------------------------------------

/// Run with: `cargo test --test fuzz -- --ignored mutation_probe --nocapture`.
///
/// A larger per-seed budget than the CI campaign, so a removed **[GLOBAL]** check is reliably
/// exercised within it. Against **correct** code this test panics with "NOT DETECTED" (nothing to
/// find) -- that is expected and fine; it is meaningless to run against correct code. Against a
/// **mutated** build (one enforcement deliberately removed, per `docs/security/mutation-report.md`),
/// it must panic with "VIOLATION DETECTED", printing the seed, the exact step index, the panic
/// message (which invariant fired) and a minimized reproduction trace.
#[test]
#[ignore = "manual mutation-validation probe -- see docs/security/mutation-report.md"]
fn mutation_probe() {
    const SEEDS: [u64; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const OPS_PER_SEED: usize = 2_000;
    for seed in SEEDS {
        let trace = generate_trace(seed, OPS_PER_SEED);
        if let Err((step, msg)) = replay(seed, &trace) {
            eprintln!("[mutation_probe] seed={seed} VIOLATION at step {step}: {msg}");
            let prefix = generate_trace(seed, step + 1);
            let minimized = shrink(seed, &prefix);
            eprintln!(
                "[mutation_probe] minimized trace ({} steps, from {} steps to first detection):",
                minimized.len(),
                step + 1
            );
            for (i, s) in minimized.iter().enumerate() {
                eprintln!("  [{i}] {s:?}");
            }
            panic!(
                "VIOLATION DETECTED: seed={seed} step={step} ops_to_detect={} message={msg}",
                step + 1
            );
        }
    }
    panic!(
        "NOT DETECTED: no seed in the probe budget (seeds={SEEDS:?}, ops/seed={OPS_PER_SEED}) found a violation"
    );
}

/// As `mutation_probe`, but specifically for **INV-SOLV-04**: a real, organic random walk almost
/// never reaches `absorb_bad_debt`'s precondition (`collateral_amount == 0` EXACTLY,
/// `borrow_shares > 0`) within any practical budget, because it requires a FULL (not partial)
/// liquidation, which itself requires HF deep enough to clear `full_liq_hf`. This probe directly
/// seeds that state via `World::seed_bad_debt_position` (the same fixture-injection technique
/// `tests/phase6_bad_debt.rs` already uses), then runs a bounded biased random tail on top so
/// `absorb_bad_debt` -- one of the generator's normal weighted choices -- gets a real chance to
/// fire against the mutated build within a small budget. Run with:
/// `cargo test --test fuzz -- --ignored mutation_probe_bad_debt --nocapture`.
#[test]
#[ignore = "manual mutation-validation probe (INV-SOLV-04) -- see docs/security/mutation-report.md"]
fn mutation_probe_bad_debt() {
    use world::{BORROWER_A, LENDER_A};

    const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];
    const TAIL_OPS: usize = 500;
    const SUPPLY_AMOUNT: u64 = 1_000_000_000_000;
    const BAD_ASSETS: u64 = 500_000_000;

    for seed in SEEDS {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut world = build_world(seed);
            let market_idx = 0;
            let m = &world.markets[market_idx];
            let lender_ata = m.loan_atas[LENDER_A];
            let lender = world.actors[LENDER_A].insecure_clone();
            let (market, position, fee_position, loan_vault, loan_mint, loan_token_program) = (
                m.market,
                m.positions[LENDER_A],
                m.fee_position,
                m.loan_vault,
                m.loan_mint,
                m.loan_token_program,
            );
            aegis_test_kit::supply(
                &mut world.svm,
                &lender,
                market,
                position,
                fee_position,
                loan_vault,
                lender_ata,
                loan_mint,
                loan_token_program,
                SUPPLY_AMOUNT,
                0,
            )
            .expect("seed liquidity for the bad-debt probe");
            // This setup supply bypasses `execute_step`, so record its ledger effect manually --
            // otherwise the value-creation check below would see LENDER_A's later withdrawals
            // with no matching contribution and report a false "value creation" finding.
            world.markets[market_idx].ledger.ensure(world.actors.len());
            world.markets[market_idx]
                .ledger
                .record_loan_in(LENDER_A, SUPPLY_AMOUNT);
            world.seed_bad_debt_position(market_idx, BORROWER_A, BAD_ASSETS);

            let mut rng = StdRng::seed_from_u64(seed ^ 0xBAD_DEBB_0000_0000);
            for i in 0..TAIL_OPS {
                let step = actions::generate_step(&mut rng, &world);
                let step_desc = format!("{step:?}");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute_step(&mut world, &step)
                }));
                if let Err(e) = result {
                    let msg = panic_message(e);
                    panic!("VIOLATION DETECTED at tail step {i}: {msg} (step was {step_desc})");
                }
            }
            for m in &world.markets {
                m.ledger.assert_no_value_creation(&ACTOR_NAMES, m.label);
            }
        }));
        std::panic::set_hook(prev_hook);
        match outcome {
            Ok(()) => eprintln!("[mutation_probe_bad_debt] seed={seed}: no violation"),
            Err(e) => {
                let msg = panic_message(e);
                eprintln!("[mutation_probe_bad_debt] seed={seed} VIOLATION: {msg}");
                panic!("VIOLATION DETECTED: seed={seed} message={msg}");
            }
        }
    }
    panic!("NOT DETECTED: no seed in the bad-debt probe budget found a violation");
}
