//! **TEST-KIT STATE INJECTION — for constructing pre-existing debt only.**
//!
//! Phase 4's `borrow` is hard-gated to always fail (`OracleNotYetAvailable`) until Phase 5's
//! oracle exists (`docs/phase-roadmap.md` "Sequencing the oracle dependency") — so no real
//! transaction can ever produce a position with `borrow_shares > 0` yet. Several required Phase 4
//! tests (`repay` against real debt, interest accrual on a market with utilization, the 100%
//! utilization case) genuinely need such a position to exist. `docs/phases/phase-04-lending.md`
//! and the Phase 4 task brief are explicit that this must be done through **test-kit state
//! injection**, never by weakening `borrow` — the same legitimate technique Phase 2/3 already
//! established (`svm.set_account` to inject an attacker-owned or phantom-debt fixture account).
//!
//! [`seed_borrow_state`] writes directly to the already-real `Market`/`Position`/`loan_vault`
//! accounts (created by real `create_market`/`init_position`/`supply` transactions) to make it
//! *as if* a legitimate `borrow` had already succeeded: it sets `total_borrow_assets`,
//! `total_borrow_shares` and `position.borrow_shares`, and debits `loan_vault`'s own token balance
//! by exactly `total_borrow_assets` so **INV-CUS-01 holds immediately after injection**, not just
//! after the next real instruction. This is a fixture, not an instruction — it must never be
//! reachable from production code, and nothing in `programs/aegis` imports this crate at all
//! (`architecture.md` §3).

use crate::market::{fetch_market, fetch_position};
use anchor_lang::AccountSerialize;
use litesvm::LiteSVM;
use solana_account::Account as RawAccount;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use spl_token_2022_interface::state::Account as SplTokenAccount;

/// Overwrites a token account's `amount` field in place (the first 8 bytes after `mint`+`owner`
/// in the shared 165-byte SPL Token / Token-2022 base layout), preserving every other byte
/// including any Token-2022 TLV extensions appended after the base region.
pub fn set_token_account_amount(svm: &mut LiteSVM, token_account: Pubkey, new_amount: u64) {
    let existing = svm
        .get_account(&token_account)
        .expect("token account must exist");
    let mut data = existing.data.clone();
    let mut base = SplTokenAccount::unpack(&data[..SplTokenAccount::LEN])
        .expect("valid base token account layout");
    base.amount = new_amount;
    SplTokenAccount::pack(base, &mut data[..SplTokenAccount::LEN])
        .expect("re-packing the base layout into the same-sized region cannot fail");
    svm.set_account(
        token_account,
        RawAccount {
            lamports: existing.lamports,
            data,
            owner: existing.owner,
            executable: existing.executable,
            rent_epoch: existing.rent_epoch,
        },
    )
    .expect("failed to inject token-account balance fixture");
}

/// Seeds `position` with `position_borrow_shares` borrow shares and grows the market's
/// `total_borrow_assets`/`total_borrow_shares` totals to match — simulating a `borrow` that
/// cannot actually be executed in Phase 4. `loan_vault`'s real SPL balance is debited by
/// `total_borrow_assets_delta` so the vault-reconciliation identity (INV-CUS-01) holds exactly
/// immediately afterward, exactly as it would if a real `borrow` had moved those tokens out.
///
/// `total_borrow_assets_delta` and `borrow_shares_delta` are **added** to the market's current
/// totals (rather than replacing them outright) so this can be called against a market that
/// already has real supply/prior injected debt without the caller having to recompute the whole
/// state by hand.
pub fn seed_borrow_state(
    svm: &mut LiteSVM,
    market: Pubkey,
    position: Pubkey,
    total_borrow_assets_delta: u64,
    borrow_shares_delta: u128,
) {
    let mut market_state = fetch_market(svm, &market);
    let loan_vault = market_state.loan_vault;

    market_state.total_borrow_assets = market_state
        .total_borrow_assets
        .checked_add(total_borrow_assets_delta)
        .expect("fixture overflow");
    market_state.total_borrow_shares = market_state
        .total_borrow_shares
        .checked_add(borrow_shares_delta)
        .expect("fixture overflow");

    let existing_market_account = svm.get_account(&market).expect("market account must exist");
    let mut data = Vec::new();
    market_state
        .try_serialize(&mut data)
        .expect("serialize injected market state");
    svm.set_account(
        market,
        RawAccount {
            lamports: existing_market_account.lamports,
            data,
            owner: existing_market_account.owner,
            executable: existing_market_account.executable,
            rent_epoch: existing_market_account.rent_epoch,
        },
    )
    .expect("failed to inject market borrow-state fixture");

    let mut position_state = fetch_position(svm, &position);
    position_state.borrow_shares = position_state
        .borrow_shares
        .checked_add(borrow_shares_delta)
        .expect("fixture overflow");
    let existing_position_account = svm
        .get_account(&position)
        .expect("position account must exist");
    let mut pos_data = Vec::new();
    position_state
        .try_serialize(&mut pos_data)
        .expect("serialize injected position state");
    svm.set_account(
        position,
        RawAccount {
            lamports: existing_position_account.lamports,
            data: pos_data,
            owner: existing_position_account.owner,
            executable: existing_position_account.executable,
            rent_epoch: existing_position_account.rent_epoch,
        },
    )
    .expect("failed to inject position borrow-state fixture");

    // Debit the vault's real balance to match -- INV-CUS-01 must hold immediately after
    // injection, exactly as if the tokens had actually left via a real `borrow`.
    let vault_before = crate::token_accounts::fetch_token_account_base(svm, &loan_vault);
    let vault_after_amount = vault_before
        .amount
        .checked_sub(total_borrow_assets_delta)
        .expect("fixture: loan_vault does not hold enough real tokens for this injected debt");
    set_token_account_amount(svm, loan_vault, vault_after_amount);
}
