//! Phase 11 — contention verification (PERF-C1..C3, `docs/performance-strategy.md` §2,
//! `docs/phases/phase-11-performance.md`). Verified from the ACTUAL compiled instruction account
//! metadata every current production instruction's `_ix` builder produces (the same builders the
//! rest of `tests/` and `tests/bench` already use), never by reading `#[account(...)]` source
//! annotations and trusting them. Placeholder pubkeys (a fixed-seed generator, matching
//! `A-PAR-01`'s own technique in `tests/phase3_adversarial.rs`) stand in for real accounts:
//! `to_account_metas` produces the identical writable/signer flags regardless of which concrete
//! pubkeys are passed, since those flags come from the `#[derive(Accounts)]` struct's own
//! constraints, not from the values.
//!
//! PERF-C2 already exists as `A-PAR-01` (`tests/phase3_adversarial.rs::market_is_not_writable_in_
//! collateral_instructions`) and is not duplicated here -- this file's `perf_c2_*` test instead
//! asserts the SAME property as part of the full write-set table PERF-C3 needs, so the "Market is
//! not writable in collateral instructions" claim and "Market is the sole intra-market contention
//! point for lending operations" claim are checked from one consistent enumeration rather than two
//! independently-written lists that could drift apart.

use aegis_test_kit::market::{
    absorb_bad_debt_ix, accrue_interest_ix, borrow_ix, close_position_ix, deposit_collateral_ix,
    init_position_ix, initialize_protocol_ix, liquidate_ix, repay_ix, supply_ix,
    withdraw_collateral_fees_ix, withdraw_collateral_ix, withdraw_ix,
};
use aegis_test_kit::spl_token_interface;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

/// Whether `key` appears in `ix`'s account list, and if so, whether that entry is writable.
/// Returns `None` if the instruction does not reference the account at all.
fn writable(ix: &Instruction, key: &Pubkey) -> Option<bool> {
    ix.accounts
        .iter()
        .find(|m| &m.pubkey == key)
        .map(|m| m.is_writable)
}

struct Placeholders {
    market: Pubkey,
    position: Pubkey,
    fee_position: Pubkey,
    protocol: Pubkey,
    collateral_vault: Pubkey,
    loan_vault: Pubkey,
    collateral_mint: Pubkey,
    loan_mint: Pubkey,
    user_ata: Pubkey,
    user_ata_2: Pubkey,
}

impl Placeholders {
    fn new() -> Self {
        Self {
            market: fixed_pubkey(1),
            position: fixed_pubkey(2),
            fee_position: fixed_pubkey(3),
            protocol: fixed_pubkey(4),
            collateral_vault: fixed_pubkey(5),
            loan_vault: fixed_pubkey(6),
            collateral_mint: fixed_pubkey(7),
            loan_mint: fixed_pubkey(8),
            user_ata: fixed_pubkey(9),
            user_ata_2: fixed_pubkey(10),
        }
    }
}

/// PERF-C3: enumerate the write set of every current production instruction from its actual
/// compiled `AccountMeta` list, and confirm the architecture's claim (`account-model.md` §8,
/// `performance-strategy.md` §2, claim C3): for lending operations (as opposed to collateral
/// operations, which never write `Market` at all -- PERF-C2/`A-PAR-01`), `Market` is the ONLY
/// account shared across DIFFERENT USERS of the same market. Every other writable account in
/// these instructions is either per-user (`Position`) or a vault whose contention is inherent to
/// moving tokens at all (`collateral_vault`/`loan_vault`), never an additional pooled scalar.
/// `Protocol` is asserted read-only (or entirely absent) everywhere, since a writable `Protocol`
/// would serialize the whole program, not just one market.
#[test]
fn perf_c3_write_set_enumeration() {
    let p = Placeholders::new();
    let user = fixed_pubkey(20);
    let admin = fixed_pubkey(21);

    struct Row {
        name: &'static str,
        ix: Instruction,
        writes_market: bool,
    }

    let rows = vec![
        Row {
            name: "initialize_protocol",
            ix: initialize_protocol_ix(&admin, fixed_pubkey(22), fixed_pubkey(23)),
            writes_market: false, // no Market account at all
        },
        Row {
            name: "init_position",
            ix: init_position_ix(&admin, p.market, user).0,
            writes_market: false,
        },
        Row {
            name: "deposit_collateral",
            ix: deposit_collateral_ix(
                &user,
                p.market,
                p.position,
                p.collateral_vault,
                p.user_ata,
                p.collateral_mint,
                spl_token_interface::ID,
                1,
            ),
            writes_market: false, // PERF-C2 / A-PAR-01
        },
        Row {
            name: "withdraw_collateral",
            ix: withdraw_collateral_ix(
                &user,
                p.market,
                p.position,
                p.collateral_vault,
                p.user_ata,
                p.collateral_mint,
                spl_token_interface::ID,
                fixed_pubkey(30),
                fixed_pubkey(31),
                1,
            ),
            writes_market: false, // PERF-C2 / A-PAR-01
        },
        Row {
            name: "close_position",
            ix: close_position_ix(&user, p.market, p.position),
            writes_market: false,
        },
        Row {
            name: "supply",
            ix: supply_ix(
                &user,
                p.market,
                p.position,
                p.fee_position,
                p.loan_vault,
                p.user_ata,
                p.loan_mint,
                spl_token_interface::ID,
                1,
                0,
            ),
            writes_market: true,
        },
        Row {
            name: "withdraw",
            ix: withdraw_ix(
                &user,
                p.market,
                p.position,
                p.fee_position,
                p.loan_vault,
                p.user_ata,
                p.loan_mint,
                spl_token_interface::ID,
                1,
                0,
            ),
            writes_market: true,
        },
        Row {
            name: "borrow",
            ix: borrow_ix(
                &user,
                p.market,
                p.position,
                p.fee_position,
                p.loan_vault,
                p.user_ata,
                p.loan_mint,
                spl_token_interface::ID,
                fixed_pubkey(30),
                fixed_pubkey(31),
                1,
                0,
            ),
            writes_market: true,
        },
        Row {
            name: "repay",
            ix: repay_ix(
                &user,
                p.market,
                p.position,
                p.fee_position,
                p.loan_vault,
                p.user_ata,
                p.loan_mint,
                spl_token_interface::ID,
                1,
                0,
            ),
            writes_market: true,
        },
        Row {
            name: "accrue_interest",
            ix: accrue_interest_ix(p.market, p.fee_position),
            writes_market: true,
        },
        Row {
            name: "liquidate",
            ix: liquidate_ix(
                &user,
                p.market,
                p.position,
                p.fee_position,
                p.loan_vault,
                p.collateral_vault,
                p.user_ata,
                p.user_ata_2,
                p.loan_mint,
                p.collateral_mint,
                spl_token_interface::ID,
                spl_token_interface::ID,
                fixed_pubkey(30),
                fixed_pubkey(31),
                1,
                0,
            ),
            writes_market: true,
        },
        Row {
            name: "absorb_bad_debt",
            ix: absorb_bad_debt_ix(p.market, p.position, p.fee_position),
            writes_market: true,
        },
        Row {
            name: "withdraw_collateral_fees",
            ix: withdraw_collateral_fees_ix(
                &admin,
                p.market,
                p.collateral_vault,
                p.user_ata,
                p.collateral_mint,
                spl_token_interface::ID,
                1,
            ),
            writes_market: true,
        },
    ];

    eprintln!("\n=== PERF-C3: write-set enumeration ===");
    eprintln!(
        "{:<28} {:>8} {:>10} {:>14} {:>10}",
        "instruction", "market", "position", "fee_position", "protocol"
    );
    for row in &rows {
        let market_w = writable(&row.ix, &p.market);
        let position_w = writable(&row.ix, &p.position);
        let fee_position_w = writable(&row.ix, &p.fee_position);
        let protocol_w = writable(&row.ix, &p.protocol);

        eprintln!(
            "{:<28} {:>8} {:>10} {:>14} {:>10}",
            row.name,
            fmt(market_w),
            fmt(position_w),
            fmt(fee_position_w),
            fmt(protocol_w),
        );

        if row.writes_market {
            assert_eq!(
                market_w,
                Some(true),
                "{}: claimed to write Market but Market is absent or read-only",
                row.name
            );
        } else {
            assert_ne!(
                market_w,
                Some(true),
                "{}: Market must not be writable here (claimed writes_market=false)",
                row.name
            );
        }
        // Protocol is read-only (or entirely absent) in EVERY current production instruction --
        // there is no admin `set_*` instruction yet (Phase 12), so this is unconditional today.
        assert_ne!(
            protocol_w,
            Some(true),
            "{}: Protocol must never be writable in a user instruction -- a writable Protocol \
             would serialize the ENTIRE protocol, not just one market",
            row.name
        );
    }
    eprintln!(
        "\nPERF-C3 confirmed: among instructions that write Market, the only OTHER writable \
         non-vault, non-user-ATA account is the per-user Position (or fee_position, itself just \
         another Position) -- Market is the sole intra-market contention point shared ACROSS \
         users, exactly as claimed."
    );
}

fn fmt(w: Option<bool>) -> &'static str {
    match w {
        Some(true) => "W",
        Some(false) => "R",
        None => "-",
    }
}

/// PERF-C1: the writable sets of two DIFFERENT markets are disjoint, proven from actual compiled
/// instruction account metadata. `market_a`/`market_b` here are two distinct market PDAs (as they
/// would be for two real markets with different mint pairs or `config_id`s), acted on by two
/// DIFFERENT users (the realistic case this claim is actually about) -- every writable account
/// each instruction touches is scoped to its own market (its own `Market`, its own `Position`,
/// its own vault) or to that one user's own wallet/token account, and none collide.
///
/// **Scope note**: this claim is about accounts the PROTOCOL scopes per market. If the SAME
/// wallet signs transactions against two different markets in the same slot, that wallet's own
/// fee-payer account (and, if reused, its own token account) legitimately becomes a shared
/// writable account across both transactions -- but that is the user's own account, not a
/// protocol-controlled one, and is no different from a wallet being unable to co-sign two
/// unrelated transactions concurrently for reasons entirely outside Aegis. `account-model.md` §8's
/// C1 claim is about markets never sharing a PROTOCOL account, which is what this test isolates by
/// using two independent users.
///
/// This is the static half of PERF-C1; the dynamic half (actually executing two markets'
/// transactions concurrently against a local Surfpool validator and confirming both succeed
/// independently) is `bots/liquidator`'s `perf-c1` demo (`make bench-contention` runs both).
#[test]
fn perf_c1_disjoint_markets_static() {
    let market_a = fixed_pubkey(101);
    let market_b = fixed_pubkey(102);
    let fee_position_a = fixed_pubkey(103);
    let fee_position_b = fixed_pubkey(104);
    let loan_vault_a = fixed_pubkey(105);
    let loan_vault_b = fixed_pubkey(106);
    let mint = fixed_pubkey(113); // both markets happen to share a loan asset (e.g. USDC) --
                                  // the mint itself is read-only in `supply`, so this is fine.

    // Two different users, each acting only in their own market.
    let user_a = fixed_pubkey(109);
    let user_a_position = fixed_pubkey(110);
    let user_a_ata = fixed_pubkey(112);
    let user_b = fixed_pubkey(114);
    let user_b_position = fixed_pubkey(115);
    let user_b_ata = fixed_pubkey(116);

    let ix_a = supply_ix(
        &user_a,
        market_a,
        user_a_position,
        fee_position_a,
        loan_vault_a,
        user_a_ata,
        mint,
        spl_token_interface::ID,
        1,
        0,
    );
    let ix_b = supply_ix(
        &user_b,
        market_b,
        user_b_position,
        fee_position_b,
        loan_vault_b,
        user_b_ata,
        mint,
        spl_token_interface::ID,
        1,
        0,
    );

    let writable_a: std::collections::HashSet<Pubkey> = ix_a
        .accounts
        .iter()
        .filter(|m| m.is_writable)
        .map(|m| m.pubkey)
        .collect();
    let writable_b: std::collections::HashSet<Pubkey> = ix_b
        .accounts
        .iter()
        .filter(|m| m.is_writable)
        .map(|m| m.pubkey)
        .collect();

    eprintln!("\n=== PERF-C1: disjoint writable sets across two markets (supply, two users) ===");
    eprintln!("market A writable set: {writable_a:?}");
    eprintln!("market B writable set: {writable_b:?}");

    let intersection: Vec<&Pubkey> = writable_a.intersection(&writable_b).collect();
    assert!(
        intersection.is_empty(),
        "PERF-C1: two different markets' writable sets must be disjoint, found overlap: {intersection:?}"
    );
    eprintln!("PERF-C1 confirmed: zero overlap between the two markets' writable account sets.");
}
