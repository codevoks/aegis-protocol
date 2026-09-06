//! Read-only decoding for token accounts and mints created by fixtures or by the program under
//! test (`zero-cost-demo.md` §6: "account fetching/decoding where required").

use litesvm::LiteSVM;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use spl_token_2022_interface::extension::{
    BaseStateWithExtensions, ExtensionType, StateWithExtensions,
};
use spl_token_2022_interface::state::{Account as SplTokenAccount, Mint as SplMint};

/// Legacy SPL Token and Token-2022 share an identical 165-byte base token-account layout
/// regardless of any Token-2022 extensions appended after it, so this decodes either uniformly.
pub fn fetch_token_account_base(svm: &LiteSVM, token_account: &Pubkey) -> SplTokenAccount {
    let account = svm
        .get_account(token_account)
        .expect("token account must exist");
    SplTokenAccount::unpack(&account.data[..SplTokenAccount::LEN])
        .expect("valid base token account layout")
}

/// The mint's Token-2022 extension inventory (empty for a classic SPL Token mint), for asserting
/// against the `MarketCreated` event's recorded extension list.
pub fn fetch_mint_extension_types(svm: &LiteSVM, mint: &Pubkey) -> Vec<ExtensionType> {
    let account = svm.get_account(mint).expect("mint account must exist");
    StateWithExtensions::<SplMint>::unpack(&account.data)
        .expect("valid mint account")
        .get_extension_types()
        .expect("parseable extension list")
}
