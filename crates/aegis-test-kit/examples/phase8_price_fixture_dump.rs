//! Phase 8 bot demo support: dumps byte-exact Pyth `PriceUpdateV2` fixture accounts (the same
//! `aegis_test_kit::PriceFixture` every Rust test/demo already uses, ADR-0008) to a JSON file, so
//! the TypeScript keeper's local demo (`bots/liquidator/src/demo.ts`) can inject them into a local
//! Surfpool validator via its `surfnet_setAccount` RPC method -- without reimplementing Pyth's
//! account layout in TypeScript at all.
//!
//! Run with: `cargo run -p aegis-test-kit --example phase8_price_fixture_dump -- <output.json>`

use aegis_test_kit::PriceFixture;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];
const FIXTURE_LAMPORTS: u64 = 1_000_000_000;

fn dump_one(pubkey_seed: u8, fixture: PriceFixture) -> serde_json::Value {
    let pubkey = solana_pubkey::Pubkey::new_from_array([pubkey_seed; 32]);
    let owner = fixture.owner;
    let data = fixture.serialize();
    serde_json::json!({
        "pubkey": pubkey.to_string(),
        "owner": owner.to_string(),
        "lamports": FIXTURE_LAMPORTS,
        "dataBase64": STANDARD.encode(data),
    })
}

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .expect("usage: phase8_price_fixture_dump <output.json>");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Same two pubkeys throughout (seeds 15/16) -- exactly how a real Pyth pull price account
    // works: the same account's data is updated over time, not re-created at a new address.
    // "valid": SOL=$150.00, USDC=$1.00 (used to open the borrow). "crash": SOL=$95.00±$0.20,
    // USDC=$1.0000±$0.0002 (the exact figures tests/phase6_liquidation.rs's worked example uses).
    let valid_collateral = dump_one(
        15,
        PriceFixture::valid(COLLATERAL_FEED_ID, 15_000_000_000, 0, -8, now),
    );
    let valid_loan = dump_one(
        16,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 0, -8, now),
    );
    let crash_collateral = dump_one(
        15,
        PriceFixture::valid(COLLATERAL_FEED_ID, 9_500_000_000, 20_000_000, -8, now),
    );
    let crash_loan = dump_one(
        16,
        PriceFixture::valid(LOAN_FEED_ID, 100_000_000, 20_000, -8, now),
    );

    let out = serde_json::json!({
        "collateralFeedId": hex::encode(COLLATERAL_FEED_ID),
        "loanFeedId": hex::encode(LOAN_FEED_ID),
        "valid": { "collateralPriceUpdate": valid_collateral, "loanPriceUpdate": valid_loan },
        "crash": { "collateralPriceUpdate": crash_collateral, "loanPriceUpdate": crash_loan },
    });

    std::fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap())
        .expect("write price fixture JSON");
    println!("Wrote price fixtures to {out_path}");
}
