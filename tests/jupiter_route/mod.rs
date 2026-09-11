//! `N-JUP-01` (optional, network-tagged) — `docs/phases/phase-08-composability.md`,
//! `docs/ecosystem-research.md` §16.2 (RV-8). `#[ignore]`d: never part of `cargo test --workspace`
//! / `make test` (`AGENTS.md` §16, zero-cost/offline requirement). Run explicitly with:
//!
//! ```text
//! cargo test --test network -- --ignored --nocapture
//! ```
//!
//! ## What this test actually does, and what it does not
//!
//! It performs **real** HTTP calls to Jupiter's public Swap API (`api.jup.ag`) — a genuine quote
//! for wSOL → USDC followed by a genuine `swap-instructions` request — and asserts the response
//! has the shape RV-8 documents (a `routePlan`, a real `swapInstruction` with a program id and
//! account list). This is the part of RV-8 that is actually verifiable without deploying anything:
//! that the documented integration surface is live and returns what the docs say it returns.
//!
//! It does **not** go on to execute that instruction against a Surfpool mainnet fork with a real
//! Aegis liquidation. Doing so needs a dedicated Jupiter-relay callback program (the specific
//! route a live quote returns can target any of several different underlying AMM programs, each
//! with its own account layout, so forwarding it correctly from inside the fixed callback account
//! contract is its own scoped piece of work) and a fork-side market seeded with real mainnet
//! mints — genuinely optional, out-of-scope engineering for this pass. This is recorded here
//! honestly rather than faked: **the on-chain execution leg of N-JUP-01 was NOT RUN.**
//! `docs/project-status.md` records this same status. The zero-cost, offline, required path
//! (`I-LIQ-CB-01`) already exercises the entire on-chain callback mechanism end to end with a
//! deterministic local price — this test adds coverage only for the live Jupiter API surface.

use std::process::Command;

const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
/// A syntactically valid but arbitrary pubkey -- Jupiter's `swap-instructions` endpoint only uses
/// `userPublicKey` to derive/label accounts in the returned instructions, it does not need to be
/// funded or real for this test's purpose (inspecting the returned instruction shape).
const PLACEHOLDER_USER_PUBKEY: &str = "11111111111111111111111111111112";

fn curl_get(url: &str) -> serde_json::Value {
    let output = Command::new("curl")
        .args(["-s", "-m", "15", url])
        .output()
        .expect("failed to execute curl -- is curl installed and is there network access?");
    assert!(
        output.status.success(),
        "curl exited non-zero fetching {url}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "response from {url} was not valid JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn curl_post_json(url: &str, body: &serde_json::Value) -> serde_json::Value {
    let output = Command::new("curl")
        .args([
            "-s",
            "-m",
            "15",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body.to_string(),
            url,
        ])
        .output()
        .expect("failed to execute curl -- is curl installed and is there network access?");
    assert!(
        output.status.success(),
        "curl exited non-zero posting to {url}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "response from {url} was not valid JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
#[ignore = "N-JUP-01: optional, network-tagged (docs/phases/phase-08-composability.md) -- real calls to api.jup.ag, never part of `make test`"]
fn n_jup_01_real_jupiter_quote_and_swap_instructions_have_the_documented_shape() {
    // 1 wSOL -> USDC, 50 bps slippage.
    let quote_url = format!(
        "https://api.jup.ag/swap/v1/quote?inputMint={WSOL_MINT}&outputMint={USDC_MINT}&amount=1000000000&slippageBps=50"
    );
    let quote = curl_get(&quote_url);
    println!("Jupiter quote response: {quote}");

    let route_plan = quote
        .get("routePlan")
        .and_then(|v| v.as_array())
        .expect("RV-8: quote response must include a routePlan array");
    assert!(
        !route_plan.is_empty(),
        "RV-8: routePlan must name at least one AMM leg"
    );
    let out_amount: u64 = quote
        .get("outAmount")
        .and_then(|v| v.as_str())
        .expect("RV-8: quote response must include outAmount")
        .parse()
        .expect("outAmount must be a valid u64 string");
    assert!(out_amount > 0, "a live SOL/USDC quote must be nonzero");

    let swap_instructions_body = serde_json::json!({
        "userPublicKey": PLACEHOLDER_USER_PUBKEY,
        "quoteResponse": quote,
    });
    let swap_instructions = curl_post_json(
        "https://api.jup.ag/swap/v1/swap-instructions",
        &swap_instructions_body,
    );
    println!("Jupiter swap-instructions response: {swap_instructions}");

    let swap_ix = swap_instructions
        .get("swapInstruction")
        .expect("RV-8: swap-instructions response must include a swapInstruction");
    let program_id = swap_ix
        .get("programId")
        .and_then(|v| v.as_str())
        .expect("swapInstruction must name a programId");
    assert!(
        !program_id.is_empty(),
        "the swap instruction's target program id must be present"
    );
    let accounts = swap_ix
        .get("accounts")
        .and_then(|v| v.as_array())
        .expect("swapInstruction must include an accounts list");
    assert!(
        !accounts.is_empty(),
        "a real swap instruction must reference at least one account"
    );

    println!(
        "N-JUP-01: live Jupiter Swap API integration surface confirmed -- quote outAmount={out_amount}, \
         swap instruction targets program {program_id} with {} accounts. On-chain execution \
         against a Surfpool mainnet fork was NOT attempted in this pass (see this file's module \
         doc comment) -- documented as NOT RUN, not faked as passing.",
        accounts.len()
    );
}
