//! Scenario builders for the Phase 11 CU benchmark harness. Every builder drives REAL
//! instructions through `LiteSVM` (via `aegis_test_kit`/`aegis_test_kit::market`, the same
//! functions Phases 2-10's own test suites use) to reach the state a given benchmark measures --
//! never a hand-constructed account struct. Mirrors the `Fixture`/`SeedGen`/`wallet_with_ata`
//! shape already established in `tests/phase5_oracle_adversarial.rs` and `tests/
//! phase6_liquidation.rs` so this harness stays consistent with the rest of the suite.

#![allow(dead_code)]

use aegis_test_kit::{
    create_market, create_spl_mint, create_token_2022_mint, create_token_account, deploy,
    init_position, initialize_protocol, mint_to, reference_market_args, set_price,
    spl_token_2022_interface, spl_token_interface, PriceFixture, Token2022Extension,
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub const COLLATERAL_FEED_ID: [u8; 32] = [0xAAu8; 32];
pub const LOAN_FEED_ID: [u8; 32] = [0xBBu8; 32];

/// SOL(9dp) reference collateral price: $150.00.
pub const COLLATERAL_PRICE: i64 = 15_000_000_000;
/// USDC(6dp) reference loan price: $1.00.
pub const LOAN_PRICE: i64 = 100_000_000;
pub const PRICE_EXPONENT: i32 = -8;

/// Deterministic seed generator -- no `Keypair::new()` anywhere in a benchmark fixture, so a
/// regression is reproducible from the scenario alone (`docs/zero-cost-demo.md` §6).
pub struct SeedGen(u8);
impl SeedGen {
    pub fn new() -> Self {
        Self(20)
    }
    pub fn next(&mut self) -> u8 {
        let s = self.0;
        self.0 = self.0.checked_add(1).expect("used more than 255 seeds");
        s
    }
}

pub fn fixed_pubkey(seed: u8) -> Pubkey {
    Keypair::new_from_array([seed; 32]).pubkey()
}

pub fn now(svm: &LiteSVM) -> i64 {
    svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

pub fn program_bytes() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/aegis.so"))
}

/// Which token program each side of a market uses -- the two variants every applicable
/// instruction is benchmarked under (phase-11-performance.md #7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenVariant {
    /// Both sides classic SPL Token, no extensions.
    ClassicSpl,
    /// Both sides Token-2022; collateral carries a 2% `TransferFeeConfig` (the worst realistic
    /// fee-bearing configuration a market may accept -- loan-side transfer-fee mints are rejected
    /// by `create_market` itself, `token-compatibility.md` §3), matching PERF-I6's requirement to
    /// measure Token-2022 "where required by the Phase 11 spec" and "both sides configured as
    /// required by current architecture/spec": the loan side is Token-2022 with NO extensions
    /// (the only Token-2022 configuration `create_market` accepts for the loan asset), and the
    /// collateral side carries the fee -- this is genuinely the most expensive *accepted*
    /// configuration, not an artificially cheap one.
    Token2022BothSides,
}

pub struct Fixture {
    pub svm: LiteSVM,
    pub admin: Keypair,
    pub seeds: SeedGen,
    pub market: Pubkey,
    pub fee_position: Pubkey,
    pub collateral_vault: Pubkey,
    pub loan_vault: Pubkey,
    pub collateral_mint: Pubkey,
    pub loan_mint: Pubkey,
    pub collateral_token_program: Pubkey,
    pub loan_token_program: Pubkey,
    pub fee_recipient: Pubkey,
    pub variant: TokenVariant,
}

/// Fee-bearing collateral rate used by [`TokenVariant::Token2022BothSides`]: 2%, capped at
/// u64::MAX (no maximum-fee ceiling) -- the same shape `tests/phase7_token2022.rs::a_tok_10_*`
/// already exercises against the full protocol lifecycle.
const TRANSFER_FEE_BPS: u16 = 200;
const TRANSFER_FEE_MAX: u64 = u64::MAX;

impl Fixture {
    /// Boots a fresh world with one market of the given token variant, protocol/admin already
    /// initialized. `config_id` lets callers building several markets in one `LiteSVM` (contention
    /// tests) avoid PDA collisions.
    pub fn new(variant: TokenVariant, config_id: u16) -> Self {
        let program_id = aegis::id();
        let (mut svm, admin) = deploy(program_id, program_bytes());
        let mut seeds = SeedGen::new();

        let guardian = fixed_pubkey(seeds.next());
        let fee_recipient = fixed_pubkey(seeds.next());
        initialize_protocol(&mut svm, &admin, guardian, fee_recipient)
            .expect("initialize_protocol");

        let (collateral_mint, loan_mint, collateral_token_program, loan_token_program) =
            match variant {
                TokenVariant::ClassicSpl => {
                    let c =
                        create_spl_mint(&mut svm, &admin, seeds.next(), 9, admin.pubkey(), None);
                    let l =
                        create_spl_mint(&mut svm, &admin, seeds.next(), 6, admin.pubkey(), None);
                    (c, l, spl_token_interface::ID, spl_token_interface::ID)
                }
                TokenVariant::Token2022BothSides => {
                    let c = create_token_2022_mint(
                        &mut svm,
                        &admin,
                        seeds.next(),
                        9,
                        admin.pubkey(),
                        None,
                        &[Token2022Extension::TransferFeeConfig {
                            basis_points: TRANSFER_FEE_BPS,
                            maximum_fee: TRANSFER_FEE_MAX,
                        }],
                    );
                    let l = create_token_2022_mint(
                        &mut svm,
                        &admin,
                        seeds.next(),
                        6,
                        admin.pubkey(),
                        None,
                        &[],
                    );
                    (
                        c,
                        l,
                        spl_token_2022_interface::ID,
                        spl_token_2022_interface::ID,
                    )
                }
            };

        let args = reference_market_args(config_id, COLLATERAL_FEED_ID, LOAN_FEED_ID, false);
        let (result, market, collateral_vault, loan_vault, fee_position) = create_market(
            &mut svm,
            &admin,
            collateral_mint,
            loan_mint,
            collateral_token_program,
            loan_token_program,
            fee_recipient,
            args,
        );
        result.expect("create_market must succeed");

        Self {
            svm,
            admin,
            seeds,
            market,
            fee_position,
            collateral_vault,
            loan_vault,
            collateral_mint,
            loan_mint,
            collateral_token_program,
            loan_token_program,
            fee_recipient,
            variant,
        }
    }

    /// A funded wallet with an ATA for `mint`, minted `balance` up front. The account is sized
    /// for whatever Token-2022 extensions `mint` itself carries (e.g. `TransferFeeAmount` for a
    /// transfer-fee mint) -- `create_token_account`'s own requirement, since Token-2022 holder
    /// accounts must carry the extensions their mint requires.
    pub fn wallet_with_ata(
        &mut self,
        mint: Pubkey,
        token_program: Pubkey,
        balance: u64,
    ) -> (Keypair, Pubkey) {
        let wallet = Keypair::new_from_array([self.seeds.next(); 32]);
        self.svm
            .airdrop(&wallet.pubkey(), 10_000_000_000)
            .expect("airdrop");
        let mint_extension_types = if token_program == spl_token_2022_interface::ID {
            aegis_test_kit::fetch_mint_extension_types(&self.svm, &mint)
        } else {
            vec![]
        };
        let ata = create_token_account(
            &mut self.svm,
            &self.admin,
            self.seeds.next(),
            mint,
            wallet.pubkey(),
            token_program,
            &mint_extension_types,
        );
        if balance > 0 {
            mint_to(
                &mut self.svm,
                &self.admin,
                mint,
                ata,
                &self.admin,
                balance,
                token_program,
            );
        }
        (wallet, ata)
    }

    /// An ATA for `mint` owned by an EXISTING `owner` (as opposed to [`Fixture::wallet_with_ata`],
    /// which also mints a fresh throwaway wallet as the owner) -- for when a real signer (e.g. a
    /// borrower who will later `repay`) needs a token account it can actually authorize transfers
    /// from.
    pub fn ata_for(
        &mut self,
        owner: Pubkey,
        mint: Pubkey,
        token_program: Pubkey,
        balance: u64,
    ) -> Pubkey {
        let mint_extension_types = if token_program == spl_token_2022_interface::ID {
            aegis_test_kit::fetch_mint_extension_types(&self.svm, &mint)
        } else {
            vec![]
        };
        let ata = create_token_account(
            &mut self.svm,
            &self.admin,
            self.seeds.next(),
            mint,
            owner,
            token_program,
            &mint_extension_types,
        );
        if balance > 0 {
            mint_to(
                &mut self.svm,
                &self.admin,
                mint,
                ata,
                &self.admin,
                balance,
                token_program,
            );
        }
        ata
    }

    pub fn init_position_for(&mut self, owner: Pubkey) -> Pubkey {
        let (_, position) = init_position(&mut self.svm, &self.admin, self.market, owner);
        position
    }

    /// Injects valid (non-stale, zero-confidence) prices at the world's current clock time.
    pub fn valid_prices(&mut self) -> (Pubkey, Pubkey) {
        let at = now(&self.svm);
        let c = set_price(
            &mut self.svm,
            self.seeds.next(),
            PriceFixture::valid(COLLATERAL_FEED_ID, COLLATERAL_PRICE, 0, PRICE_EXPONENT, at),
        );
        let l = set_price(
            &mut self.svm,
            self.seeds.next(),
            PriceFixture::valid(LOAN_FEED_ID, LOAN_PRICE, 0, PRICE_EXPONENT, at),
        );
        (c, l)
    }

    /// Advances the world's clock by `secs` unix seconds -- no real wall-clock waiting
    /// (`docs/zero-cost-demo.md` §6), so an accrual-bearing scenario is deterministic.
    pub fn warp_seconds(&mut self, secs: i64) {
        let mut clock = self.svm.get_sysvar::<solana_clock::Clock>();
        clock.unix_timestamp += secs;
        self.svm.set_sysvar(&clock);
    }

    /// Injects a distressed collateral price (`factor` applied to the reference price, e.g. 0.60
    /// for a 40% drop) so a previously-healthy borrow becomes liquidatable.
    pub fn distressed_prices(&mut self, collateral_price: i64) -> (Pubkey, Pubkey) {
        let at = now(&self.svm);
        let c = set_price(
            &mut self.svm,
            self.seeds.next(),
            PriceFixture::valid(COLLATERAL_FEED_ID, collateral_price, 0, PRICE_EXPONENT, at),
        );
        let l = set_price(
            &mut self.svm,
            self.seeds.next(),
            PriceFixture::valid(LOAN_FEED_ID, LOAN_PRICE, 0, PRICE_EXPONENT, at),
        );
        (c, l)
    }
}
