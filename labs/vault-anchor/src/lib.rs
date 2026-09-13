//! Phase 11 lab (`docs/adr/0003-native-pinocchio-as-labs.md`, `docs/phases/phase-11-performance.md`
//! #23-24): the SAME custody primitive `programs/aegis` itself implements for its two vaults --
//! initialize a vault PDA, deposit into it, withdraw from it via `invoke_signed` -- written in
//! Anchor 1.x, as the production-idiom baseline `labs/cu-bench` measures `labs/vault-native` and
//! `labs/vault-pinocchio` against.
//!
//! **Scope, deliberately bounded** (ADR-0003): classic SPL Token only, one mint, one owner per
//! vault. This is not a re-implementation of Aegis's Market/Position/oracle/liquidation logic --
//! it is exactly the custody shape (a state PDA authorizing a token-vault PDA) and nothing else,
//! so the three labs stay comparable and cannot grow into a second protocol.
//!
//! ## Security checks (equivalent across all three labs, `phase-11-performance.md` #27)
//! - **Signer**: `withdraw` requires `owner`'s signature (`has_one = owner`).
//! - **Mint**: every transfer is `transfer_checked` against the pinned `mint`.
//! - **Token program**: `TokenAccount`/`Mint` typed accounts are owned by the real SPL Token
//!   program; no other program's account can be substituted.
//! - **Vault / PDA**: `vault_authority` and `vault_token_account` are both canonical PDAs
//!   (`seeds = [...], bump = stored`), and `vault_token_account`'s address is additionally pinned
//!   via `has_one` on `VaultAuthority` -- the same double-validation discipline as
//!   `account-model.md` §1's "every relationship is checked twice."
//! - **Authority**: the vault token account's on-chain SPL authority is `vault_authority`, and
//!   ONLY `vault_authority`'s own PDA signature (via `invoke_signed`) can move tokens out of it.
//! - **Source/destination ownership**: Anchor's `TokenAccount` deserialization validates the
//!   account is a real SPL Token account; `transfer_checked` further validates the mint.
//! - **Amount semantics**: `amount > 0` is required; the transferred amount is exactly what the
//!   caller requested (no measured-delta accounting here -- classic SPL Token only, no
//!   transfer-fee extension in scope for this lab).

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

declare_id!("6heV9XdGiwVBqqHQBdXS4pU8WZ6g5E2sQh52BsNF9ZTQ");

pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const VAULT_TOKEN_SEED: &[u8] = b"vault_token";

#[program]
pub mod vault_anchor {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault_authority = &mut ctx.accounts.vault_authority;
        vault_authority.owner = ctx.accounts.owner.key();
        vault_authority.mint = ctx.accounts.mint.key();
        vault_authority.vault_token_account = ctx.accounts.vault_token_account.key();
        vault_authority.bump = ctx.bumps.vault_authority;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.depositor_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
        token::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        let mint_key = ctx.accounts.vault_authority.mint;
        let bump = ctx.accounts.vault_authority.bump;
        let signer_seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, mint_key.as_ref(), &[bump]];
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.vault_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let signer_seeds_list = [signer_seeds];
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts)
            .with_signer(&signer_seeds_list);
        token::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
    }
}

#[account]
pub struct VaultAuthority {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub bump: u8,
}

impl VaultAuthority {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 1;
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: only used as the vault's recorded owner pubkey; never read or written otherwise.
    pub owner: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = payer,
        space = VaultAuthority::LEN,
        seeds = [VAULT_AUTHORITY_SEED, mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(
        init,
        payer = payer,
        seeds = [VAULT_TOKEN_SEED, mint.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: Signer<'info>,
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, vault_authority.mint.as_ref()],
        bump = vault_authority.bump,
        has_one = vault_token_account,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(mut, address = vault_authority.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = depositor_token_account.mint == vault_authority.mint @ VaultError::MintMismatch)]
    pub depositor_token_account: Account<'info, TokenAccount>,
    #[account(address = vault_authority.mint)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub owner: Signer<'info>,
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, vault_authority.mint.as_ref()],
        bump = vault_authority.bump,
        has_one = owner @ VaultError::OwnerMismatch,
        has_one = vault_token_account,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,
    #[account(mut, address = vault_authority.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = recipient_token_account.mint == vault_authority.mint @ VaultError::MintMismatch)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(address = vault_authority.mint)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum VaultError {
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("token account mint does not match the vault's mint")]
    MintMismatch,
    #[msg("signer does not match the vault's recorded owner")]
    OwnerMismatch,
}
