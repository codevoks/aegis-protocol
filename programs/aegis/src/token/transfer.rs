//! `transfer_checked` helpers for both SPL Token and Token-2022, shared by every collateral and
//! loan instruction (`architecture.md` §2). Both token programs are handled by one code path:
//! `anchor_spl::token_interface::transfer_checked` dispatches to whichever program the caller
//! passes via `CpiContext`'s `program_id` (never hardcoded to either program), and
//! `token_interface::TransferChecked` accepts either program's accounts uniformly.
//!
//! **Measured-delta accounting is mandatory on every inbound transfer**
//! (`account-model.md` §6.4, `token-compatibility.md` §5.3, T-14): a Token-2022 transfer-fee mint
//! delivers strictly less than the requested amount, so the protocol must observe what actually
//! arrived rather than trust the instruction argument. The post-CPI `reload()` is the load-bearing
//! line — reading the pre-CPI deserialized balance after a CPI is the classic stale-account bug.

use crate::error::AegisError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, TokenAccount, TransferChecked};

/// Inbound transfer: CPI `transfer_checked` from `source` to `vault`, then reload `vault` and
/// return the measured `credited = after − before` delta — never the requested `amount`.
///
/// `authority` is whoever can move tokens out of `source` (the depositor, or their delegate); it
/// is **not** required to be the position owner (`account-model.md` §5.1) — the token program
/// itself enforces that `authority` actually controls `source`.
pub fn transfer_checked_in<'info>(
    source: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    vault: &mut InterfaceAccount<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    amount: u64,
    decimals: u8,
) -> Result<u64> {
    let before = vault.amount;

    token_interface::transfer_checked(
        CpiContext::new(
            token_program.key(),
            TransferChecked {
                from: source.clone(),
                mint: mint.clone(),
                to: vault.to_account_info(),
                authority: authority.clone(),
            },
        ),
        amount,
        decimals,
    )?;

    // MANDATORY: pre-CPI `vault.amount` is stale after the CPI mutates the account's underlying
    // data buffer. Reload before computing the delta (T-14).
    vault.reload()?;
    let after = vault.amount;

    after
        .checked_sub(before)
        .ok_or_else(|| error!(AegisError::VaultAccountingError))
}

/// Outbound transfer: CPI `transfer_checked` from `vault` to `destination`, signed by the
/// `Market` PDA (the vault's sole token authority, `account-model.md` §6.2) via `invoke_signed`.
///
/// The recipient bears any transfer fee — Aegis debits its own internal accounting by exactly
/// `amount`, the amount it recorded as leaving the vault (`account-model.md` §6.4).
#[allow(clippy::too_many_arguments)]
pub fn transfer_checked_out<'info>(
    vault: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    market_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    amount: u64,
    decimals: u8,
    market_signer_seeds: &[&[u8]],
) -> Result<()> {
    token_interface::transfer_checked(
        CpiContext::new(
            token_program.key(),
            TransferChecked {
                from: vault.clone(),
                mint: mint.clone(),
                to: destination.clone(),
                authority: market_authority.clone(),
            },
        )
        .with_signer(&[market_signer_seeds]),
        amount,
        decimals,
    )
}
