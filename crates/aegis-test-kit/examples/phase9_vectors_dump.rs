//! Phase 9 cross-language math vectors (`docs/phases/phase-09-sdk-ui.md` "Critical requirement —
//! cross-language math", ADR-0011, `I-SDK-01`).
//!
//! `sdk/ts/src/math.ts` re-implements `aegis-math`, and re-implementations drift. The only
//! accepted defense: this binary calls the real, frozen `aegis-math` functions directly (the exact
//! same crate `programs/aegis` links against) and emits their inputs/outputs as JSON. TS tests
//! consume these files verbatim (`sdk/ts/test/vectors.test.ts`) and assert bit-for-bit identical
//! results -- nobody hand-maintains a second set of expected values anywhere.
//!
//! Determinism: every input below is a fixed literal (no RNG, no wall-clock, no environment
//! dependence), so re-running this binary against an unchanged `aegis-math` produces byte-identical
//! output. `scripts/check-vectors.sh` uses exactly this property to detect staleness: regenerate
//! into a temp directory and diff against the committed `tests/vectors/`.
//!
//! Run with: `cargo run -p aegis-test-kit --example phase9_vectors_dump -- <output-dir>`
//! (`make vectors` / `make vectors-check` wrap this; see the Makefile and SDK README.)

use aegis_math::{
    borrow_rate, collateral_value, compute_liquidation_by_repay, compute_liquidation_by_seize,
    conservative_price_band, debt_value, health_factor, is_within_max_ltv, max_repay, mul_div_ceil,
    mul_div_floor, scale_to_wad_ceil, scale_to_wad_floor, taylor3, taylor_x, to_assets_down,
    to_assets_up, to_shares_down, to_shares_up, utilization, LiquidationParams, SECONDS_PER_YEAR,
    VIRTUAL_ASSETS, VIRTUAL_SHARES, WAD,
};
use aegis_test_kit::{
    collateral_vault_pda, loan_vault_pda, market_pda, position_pda, protocol_pda,
};
use serde_json::{json, Value};
use solana_pubkey::Pubkey;
use std::fs;
use std::path::Path;

const VECTOR_VERSION: &str = "phase-09-v1";

fn write_file(dir: &Path, name: &str, value: &Value) {
    let path = dir.join(name);
    let mut pretty = serde_json::to_string_pretty(value).expect("serialize vector file");
    pretty.push('\n');
    fs::write(&path, pretty).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

// --- fixed.rs: mul_div_floor / mul_div_ceil ---

fn fixed_vectors() -> Value {
    let mut cases = Vec::new();

    let ok = |name: &str, a: u128, b: u128, d: u128| {
        let floor = mul_div_floor(a, b, d);
        let ceil = mul_div_ceil(a, b, d);
        json!({
            "name": name,
            "inputs": {"a": a.to_string(), "b": b.to_string(), "d": d.to_string()},
            "floor": floor.map(|v| v.to_string()).ok(),
            "floorError": floor.err().map(|e| format!("{e:?}")),
            "ceil": ceil.map(|v| v.to_string()).ok(),
            "ceilError": ceil.err().map(|e| format!("{e:?}")),
        })
    };

    cases.push(ok("known_vectors_3_5_2", 3, 5, 2));
    cases.push(ok("exact_division", 10, 10, 5));
    cases.push(ok("zero_numerator", 0, 100, 7));
    cases.push(ok("identity", 1, 1, 1));
    cases.push(ok("ceil_only_rounds_up_on_remainder_7_1_2", 7, 1, 2));
    cases.push(ok("ceil_only_rounds_up_on_remainder_8_1_2", 8, 1, 2));
    cases.push(ok("division_by_zero", 1, 1, 0));
    cases.push(ok(
        "large_multiplication_survives_256_bit_intermediate",
        18_000_000_000_000_000_000_000_000,
        18_000_000_000_000_000_000,
        1_000_000_000_000_000_000,
    ));
    cases.push(ok("result_overflow", u128::MAX, u128::MAX, 1));

    json!({
        "formula": "mul_div_floor / mul_div_ceil",
        "source": "crates/aegis-math/src/fixed.rs",
        "version": VECTOR_VERSION,
        "cases": cases,
    })
}

// --- shares.rs ---

fn shares_vectors() -> Value {
    let mut cases = Vec::new();

    let case = |name: &str, assets: u64, total_assets: u64, total_shares: u128| {
        json!({
            "name": name,
            "inputs": {
                "assets": assets.to_string(),
                "totalAssets": total_assets.to_string(),
                "totalShares": total_shares.to_string(),
            },
            "toSharesDown": to_shares_down(assets, total_assets, total_shares).map(|v| v.to_string()).ok(),
            "toSharesUp": to_shares_up(assets, total_assets, total_shares).map(|v| v.to_string()).ok(),
        })
    };
    let case_shares = |name: &str, shares: u128, total_assets: u64, total_shares: u128| {
        json!({
            "name": name,
            "inputs": {
                "shares": shares.to_string(),
                "totalAssets": total_assets.to_string(),
                "totalShares": total_shares.to_string(),
            },
            "toAssetsDown": to_assets_down(shares, total_assets, total_shares).map(|v| v.to_string()).ok(),
            "toAssetsUp": to_assets_up(shares, total_assets, total_shares).map(|v| v.to_string()).ok(),
        })
    };

    // Economic-model.md §3.3: first supply into an empty market.
    cases.push(case("first_supply_empty_market", 1_000_000_000, 0, 0));
    // U-ROUND-01..04 fixture (nonzero remainder).
    cases.push(case("round_fixture_assets_3_7", 5, 3, 7));
    cases.push(case("round_fixture_assets_11_13", 6, 11, 13));
    cases.push(case_shares("round_fixture_shares_5_3", 7, 5, 3));
    cases.push(case_shares("round_fixture_shares_13_11", 6, 13, 11));
    // Ratio drift (U-SHARE-02): later depositor tax after accrual perturbs the founding ratio.
    cases.push(case(
        "ratio_drift_later_depositor",
        1_000_000_000,
        1_000_000_001,
        1_000_000_000_000_000,
    ));
    // P-ARITH-3 / maximum legal state: 256-bit intermediate required.
    cases.push(case_shares(
        "maximum_legal_share_asset_state",
        18_000_000_000_000_000_000_000_000,
        18_000_000_000_000_000_000,
        18_000_000_000_000_000_000_000_000,
    ));

    json!({
        "formula": "to_shares_down / to_shares_up / to_assets_down / to_assets_up",
        "source": "crates/aegis-math/src/shares.rs",
        "version": VECTOR_VERSION,
        "constants": {"virtualShares": VIRTUAL_SHARES.to_string(), "virtualAssets": VIRTUAL_ASSETS.to_string()},
        "cases": cases,
    })
}

// --- irm.rs ---

fn irm_vectors() -> Value {
    const REF_SLOPE1: u128 = 1_268_391_679;
    const REF_SLOPE2: u128 = 31_709_791_983;
    const REF_U_KINK: u128 = 800_000_000_000_000_000;
    const REF_MAX_RATE: u128 = 317_097_919_837;

    let mut utilization_cases = Vec::new();
    for (borrow, supply) in [
        (0u64, 0u64),
        (500, 0),
        (900_000_000, 1_000_000_000),
        (1_000_000_000, 1_000_000_000),
    ] {
        utilization_cases.push(json!({
            "name": format!("u_{borrow}_{supply}"),
            "inputs": {"totalBorrowAssets": borrow.to_string(), "totalSupplyAssets": supply.to_string()},
            "utilization": utilization(borrow, supply).map(|v| v.to_string()).ok(),
        }));
    }

    let mut rate_cases = Vec::new();
    for (name, u) in [
        ("zero", 0u128),
        ("at_kink", REF_U_KINK),
        ("ninety_pct", 900_000_000_000_000_000u128),
        ("full", WAD),
    ] {
        rate_cases.push(json!({
            "name": name,
            "inputs": {
                "u": u.to_string(), "baseRatePs": "0", "slope1Ps": REF_SLOPE1.to_string(),
                "slope2Ps": REF_SLOPE2.to_string(), "uKink": REF_U_KINK.to_string(),
                "maxRatePs": REF_MAX_RATE.to_string(),
            },
            "rate": borrow_rate(u, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).map(|v| v.to_string()).ok(),
        }));
    }
    // Rate cap firing (steep slopes).
    let steep = REF_MAX_RATE;
    rate_cases.push(json!({
        "name": "capped_by_max_rate",
        "inputs": {
            "u": WAD.to_string(), "baseRatePs": "0", "slope1Ps": steep.to_string(),
            "slope2Ps": steep.to_string(), "uKink": REF_U_KINK.to_string(),
            "maxRatePs": REF_MAX_RATE.to_string(),
        },
        "rate": borrow_rate(WAD, 0, steep, steep, REF_U_KINK, REF_MAX_RATE).map(|v| v.to_string()).ok(),
    }));

    let mut taylor_cases = Vec::new();
    for (name, r, dt) in [
        ("zero_dt", REF_MAX_RATE, 0u64),
        ("worked_example_r_dt", 17_123_287_670u128, 86_400u64),
        ("one_wad", WAD, 1u64),
    ] {
        let x = taylor_x(r, dt);
        taylor_cases.push(json!({
            "name": name,
            "inputs": {"ratePerSecondWad": r.to_string(), "dtSeconds": dt.to_string()},
            "x": x.map(|v| v.to_string()).ok(),
            "growth": x.ok().map(|xv| taylor3(xv).unwrap().to_string()),
        }));
    }

    // Full worked accrual example (economic-model.md §4.4), chained end to end.
    let u = utilization(900_000_000, 1_000_000_000).unwrap();
    let r = borrow_rate(u, 0, REF_SLOPE1, REF_SLOPE2, REF_U_KINK, REF_MAX_RATE).unwrap();
    let dt = 86_400u64;
    let x = taylor_x(r, dt).unwrap();
    let growth = taylor3(x).unwrap();
    let interest = mul_div_floor(900_000_000u128, growth, WAD).unwrap();
    let fee_amount = mul_div_floor(interest, 100_000_000_000_000_000u128, WAD).unwrap();
    let worked_accrual = json!({
        "name": "worked_accrual_ninety_pct_one_day",
        "inputs": {
            "totalBorrowAssets": "900000000", "totalSupplyAssets": "1000000000",
            "dtSeconds": dt.to_string(), "feeWad": "100000000000000000",
        },
        "utilization": u.to_string(),
        "ratePerSecondWad": r.to_string(),
        "x": x.to_string(),
        "growth": growth.to_string(),
        "interest": interest.to_string(),
        "feeAmount": fee_amount.to_string(),
    });

    json!({
        "formula": "utilization / borrow_rate / taylor3 / taylor_x",
        "source": "crates/aegis-math/src/irm.rs",
        "version": VECTOR_VERSION,
        "constants": {"secondsPerYear": SECONDS_PER_YEAR.to_string(), "wad": WAD.to_string()},
        "utilizationCases": utilization_cases,
        "rateCases": rate_cases,
        "taylorCases": taylor_cases,
        "workedAccrual": worked_accrual,
    })
}

// --- health.rs ---

fn health_vectors() -> Value {
    let mut scale_cases = Vec::new();
    for (name, raw, expo) in [
        ("typical_neg8", 15_000_000_000u128, -8i32),
        ("zero_expo", 1_000_000u128, 0i32),
        ("deep_negative_expo", 1u128, -12i32),
    ] {
        scale_cases.push(json!({
            "name": name,
            "inputs": {"raw": raw.to_string(), "expo": expo},
            "floor": scale_to_wad_floor(raw, expo).map(|v| v.to_string()).ok(),
            "ceil": scale_to_wad_ceil(raw, expo).map(|v| v.to_string()).ok(),
        }));
    }

    let mut band_cases = Vec::new();
    for (name, price, conf, expo, max_conf_bps) in [
        (
            "sol_150_conf_030",
            15_000_000_000i64,
            30_000_000u64,
            -8i32,
            100u16,
        ),
        ("usdc_1_conf_tiny", 100_000_000i64, 20_000u64, -8i32, 50u16),
        ("confidence_boundary_pass", 10_000i64, 100u64, 0i32, 100u16),
        ("confidence_boundary_fail", 10_000i64, 101u64, 0i32, 100u16),
        ("zero_price_rejected", 0i64, 0u64, -8i32, 100u16),
    ] {
        let band = conservative_price_band(price, conf, expo, max_conf_bps);
        band_cases.push(json!({
            "name": name,
            "inputs": {"price": price, "conf": conf.to_string(), "expo": expo, "maxConfBps": max_conf_bps},
            "lo": band.as_ref().ok().map(|b| b.lo.to_string()),
            "hi": band.as_ref().ok().map(|b| b.hi.to_string()),
            "error": band.err().map(|e| format!("{e:?}")),
        }));
    }

    // economic-model.md §6.5 worked examples.
    let price_c_lo_healthy = 149_700_000_000_000_000_000u128;
    let price_l_hi = 1_000_200_000_000_000_000u128;
    let collateral_amount = 10_000_000_000u64;
    let debt_assets = 900_000_000u64;
    let liq_threshold = 800_000_000_000_000_000u128;
    let max_ltv = 750_000_000_000_000_000u128;
    let cv1 = collateral_value(collateral_amount, price_c_lo_healthy, 9).unwrap();
    let dv1 = debt_value(debt_assets, price_l_hi, 6).unwrap();
    let hf1 = health_factor(cv1, liq_threshold, dv1).unwrap();
    let within1 = is_within_max_ltv(cv1, dv1, max_ltv).unwrap();

    let price_c_lo_crash = 94_800_000_000_000_000_000u128;
    let cv2 = collateral_value(collateral_amount, price_c_lo_crash, 9).unwrap();
    let dv2 = debt_value(debt_assets, price_l_hi, 6).unwrap();
    let hf2 = health_factor(cv2, liq_threshold, dv2).unwrap();

    let worked = json!({
        "healthy": {
            "inputs": {
                "collateralAmount": collateral_amount.to_string(), "collateralDecimals": 9,
                "debtAssets": debt_assets.to_string(), "loanDecimals": 6,
                "priceCLo": price_c_lo_healthy.to_string(), "priceLHi": price_l_hi.to_string(),
                "liqThreshold": liq_threshold.to_string(), "maxLtv": max_ltv.to_string(),
            },
            "collateralValue": cv1.to_string(),
            "debtValue": dv1.to_string(),
            "healthFactor": hf1.to_string(),
            "isWithinMaxLtv": within1,
        },
        "crashed": {
            "inputs": {
                "collateralAmount": collateral_amount.to_string(), "collateralDecimals": 9,
                "debtAssets": debt_assets.to_string(), "loanDecimals": 6,
                "priceCLo": price_c_lo_crash.to_string(), "priceLHi": price_l_hi.to_string(),
                "liqThreshold": liq_threshold.to_string(),
            },
            "collateralValue": cv2.to_string(),
            "debtValue": dv2.to_string(),
            "healthFactor": hf2.to_string(),
        },
    });

    json!({
        "formula": "scale_to_wad_* / conservative_price_band / collateral_value / debt_value / health_factor / is_within_max_ltv",
        "source": "crates/aegis-math/src/health.rs",
        "version": VECTOR_VERSION,
        "scaleCases": scale_cases,
        "bandCases": band_cases,
        "workedExamples": worked,
    })
}

// --- liquidation.rs ---

fn ref_params() -> LiquidationParams {
    LiquidationParams {
        collateral_decimals: 9,
        loan_decimals: 6,
        price_c_lo: 94_800_000_000_000_000_000,
        price_l_hi: 1_000_200_000_000_000_000,
        liq_bonus: 50_000_000_000_000_000,
        liq_protocol_fee: 100_000_000_000_000_000,
        close_factor: 500_000_000_000_000_000,
        full_liq_hf: 950_000_000_000_000_000,
        min_debt: 10_000_000,
    }
}

fn params_json(p: &LiquidationParams) -> Value {
    json!({
        "collateralDecimals": p.collateral_decimals,
        "loanDecimals": p.loan_decimals,
        "priceCLo": p.price_c_lo.to_string(),
        "priceLHi": p.price_l_hi.to_string(),
        "liqBonus": p.liq_bonus.to_string(),
        "liqProtocolFee": p.liq_protocol_fee.to_string(),
        "closeFactor": p.close_factor.to_string(),
        "fullLiqHf": p.full_liq_hf.to_string(),
        "minDebt": p.min_debt.to_string(),
    })
}

fn outcome_json(o: &Result<aegis_math::LiquidationOutcome, aegis_math::LiquidationError>) -> Value {
    match o {
        Ok(o) => json!({
            "repayAssets": o.repay_assets.to_string(),
            "totalSeize": o.total_seize.to_string(),
            "baseSeize": o.base_seize.to_string(),
            "bonusAmount": o.bonus_amount.to_string(),
            "protocolCut": o.protocol_cut.to_string(),
            "toLiquidator": o.to_liquidator.to_string(),
            "clamped": o.clamped,
            "error": Value::Null,
        }),
        Err(e) => json!({"error": format!("{e:?}")}),
    }
}

fn liquidation_vectors() -> Value {
    let params = ref_params();
    let debt_assets = 900_000_000u64;
    let hf_healthy = WAD; // == WAD, strictly not liquidatable
    let hf_crashed = 842_498_167_033_260_014u128; // economic-model.md §6.5

    let mut max_repay_cases = Vec::new();
    max_repay_cases.push(json!({
        "name": "full_liquidation_band",
        "inputs": {"debtAssets": debt_assets.to_string(), "hf": hf_crashed.to_string()},
        "maxRepay": max_repay(debt_assets, hf_crashed, &params).map(|v| v.to_string()).ok(),
    }));
    let hf_close_factor_band = 960_000_000_000_000_000u128;
    let mut p_no_dust = params;
    p_no_dust.close_factor = 500_000_000_000_000_000;
    p_no_dust.min_debt = 10_000_000;
    let no_dust_result = max_repay(100_000_000, hf_close_factor_band, &p_no_dust)
        .map(|v| v.to_string())
        .ok();
    max_repay_cases.push(json!({
        "name": "close_factor_band",
        "inputs": {"debtAssets": "100000000", "hf": hf_close_factor_band.to_string(), "closeFactor": "500000000000000000", "minDebt": "10000000"},
        "maxRepay": no_dust_result,
    }));

    let mut p_dust = params;
    p_dust.close_factor = 500_000_000_000_000_000;
    p_dust.min_debt = 60_000_000;
    let dust_result = max_repay(100_000_000, hf_close_factor_band, &p_dust)
        .map(|v| v.to_string())
        .ok();
    max_repay_cases.push(json!({
        "name": "dust_rule_forces_full",
        "inputs": {"debtAssets": "100000000", "hf": hf_close_factor_band.to_string(), "closeFactor": "500000000000000000", "minDebt": "60000000"},
        "maxRepay": dust_result,
    }));

    let mut liquidatable_cases = Vec::new();
    for (name, hf) in [
        ("equal_to_wad", WAD),
        ("one_below_wad", WAD - 1),
        ("one_above_wad", WAD + 1),
    ] {
        liquidatable_cases.push(json!({"name": name, "hf": hf.to_string(), "isLiquidatable": aegis_math::is_liquidatable(hf)}));
    }

    let collateral_amount_full = 10_000_000_000u64;
    let by_repay_worked = compute_liquidation_by_repay(
        &params,
        debt_assets,
        collateral_amount_full,
        hf_crashed,
        debt_assets,
    );

    let collateral_amount_clamped = 9_000_000_000u64;
    let by_repay_clamped = compute_liquidation_by_repay(
        &params,
        debt_assets,
        collateral_amount_clamped,
        hf_crashed,
        debt_assets,
    );

    let by_seize_matches = by_repay_worked.as_ref().ok().map(|o| {
        compute_liquidation_by_seize(
            &params,
            debt_assets,
            collateral_amount_full,
            hf_crashed,
            o.total_seize,
        )
    });

    let healthy_rejected = compute_liquidation_by_repay(
        &params,
        debt_assets,
        collateral_amount_full,
        hf_healthy,
        debt_assets,
    );

    json!({
        "formula": "max_repay / is_liquidatable / compute_liquidation_by_repay / compute_liquidation_by_seize",
        "source": "crates/aegis-math/src/liquidation.rs",
        "version": VECTOR_VERSION,
        "params": params_json(&params),
        "maxRepayCases": max_repay_cases,
        "isLiquidatableCases": liquidatable_cases,
        "worked": {
            "debtAssets": debt_assets.to_string(),
            "hf": hf_crashed.to_string(),
            "collateralAmountSufficient": collateral_amount_full.to_string(),
            "byRepaySufficient": outcome_json(&by_repay_worked),
            "collateralAmountInsufficient": collateral_amount_clamped.to_string(),
            "byRepayClamped": outcome_json(&by_repay_clamped),
            "bySeizeMatchesByRepay": by_seize_matches.as_ref().map(outcome_json),
        },
        "healthyPositionRejected": {
            "hf": hf_healthy.to_string(),
            "outcome": outcome_json(&healthy_rejected),
        },
    })
}

// --- PDA derivation parity (I-SDK-03): the SAME on-chain seed-derivation helpers
// `crates/aegis-test-kit/src/market.rs` uses for every Rust test in this repository, called
// directly here so `sdk/ts/src/pda.ts` can be asserted byte-for-byte against real
// `Pubkey::find_program_address` output -- never a re-derivation or a hand-computed expectation.

fn pk(bytes: [u8; 32]) -> Pubkey {
    Pubkey::new_from_array(bytes)
}

fn pda_vectors() -> Value {
    let mut cases = Vec::new();

    // Representative and boundary pubkeys/config_ids (`docs/phases/phase-09-sdk-ui.md` item 5:
    // "multiple public keys, multiple config IDs ... any nontrivial integer seed serialization.
    // Catch endian mistakes explicitly.").
    let mint_a = pk([0x01; 32]);
    let mint_b = pk([0x02; 32]);
    let owner_a = pk([0xAA; 32]);
    let owner_b = pk([0xBB; 32]);
    let fee_recipient = pk([0xCC; 32]);
    let all_zero = pk([0x00; 32]);
    let all_ff = pk([0xFF; 32]);

    let (protocol, protocol_bump) = protocol_pda();
    cases.push(json!({
        "name": "protocol",
        "address": protocol.to_string(),
        "bump": protocol_bump,
    }));

    // config_id boundary values, specifically chosen so a byte-order (endian) mistake in the TS
    // port's u16 seed encoding would produce a DIFFERENT address than the real one: 0 and 65535
    // are endian-invariant (all zero bytes / all one bytes), but 1 and 256 are not -- config_id=1
    // is seed bytes [0x01, 0x00] in little-endian (Rust's `to_le_bytes`) vs [0x00, 0x01] in
    // big-endian, and config_id=256 is the exact mirror ([0x00,0x01] LE vs [0x01,0x00] BE) -- a TS
    // implementation that encoded big-endian would derive a DIFFERENT market address for these
    // two, not merely a wrong one, making the mistake mechanically detectable rather than merely
    // "some byte string differs".
    for (name, collateral_mint, loan_mint, config_id) in [
        ("market_a_b_config_0", mint_a, mint_b, 0u16),
        ("market_a_b_config_1", mint_a, mint_b, 1u16),
        ("market_a_b_config_256", mint_a, mint_b, 256u16),
        ("market_a_b_config_max", mint_a, mint_b, u16::MAX),
        ("market_b_a_config_0", mint_b, mint_a, 0u16),
        ("market_zero_ff_config_42", all_zero, all_ff, 42u16),
    ] {
        let (market, market_bump) = market_pda(&collateral_mint, &loan_mint, config_id);
        let (collateral_vault, cv_bump) = collateral_vault_pda(&market);
        let (loan_vault, lv_bump) = loan_vault_pda(&market);
        cases.push(json!({
            "name": name,
            "inputs": {
                "collateralMint": collateral_mint.to_string(),
                "loanMint": loan_mint.to_string(),
                "configId": config_id,
            },
            "market": { "address": market.to_string(), "bump": market_bump },
            "collateralVault": { "address": collateral_vault.to_string(), "bump": cv_bump },
            "loanVault": { "address": loan_vault.to_string(), "bump": lv_bump },
        }));
    }

    // Position (user and fee-position -- account-model.md §9: fee_position is
    // PDA(market, market.fee_recipient), the identical seed layout as an ordinary user Position).
    let (reference_market, _) = market_pda(&mint_a, &mint_b, 0);
    for (name, market, owner) in [
        ("position_owner_a", reference_market, owner_a),
        ("position_owner_b", reference_market, owner_b),
        ("position_fee_recipient", reference_market, fee_recipient),
        ("position_all_zero_owner", reference_market, all_zero),
    ] {
        let (position, bump) = position_pda(&market, &owner);
        cases.push(json!({
            "name": name,
            "inputs": { "market": market.to_string(), "owner": owner.to_string() },
            "position": { "address": position.to_string(), "bump": bump },
        }));
    }

    json!({
        "formula": "protocol_pda / market_pda / position_pda / collateral_vault_pda / loan_vault_pda",
        "source": "crates/aegis-test-kit/src/market.rs (mirrors docs/account-model.md exactly)",
        "version": VECTOR_VERSION,
        "programId": aegis::ID.to_string(),
        "cases": cases,
    })
}

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/vectors".to_string());
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).expect("create output dir");

    write_file(out_dir, "fixed.json", &fixed_vectors());
    write_file(out_dir, "shares.json", &shares_vectors());
    write_file(out_dir, "irm.json", &irm_vectors());
    write_file(out_dir, "health.json", &health_vectors());
    write_file(out_dir, "liquidation.json", &liquidation_vectors());
    write_file(out_dir, "pdas.json", &pda_vectors());

    println!("Phase 9 vectors written to {}", out_dir.display());
}
