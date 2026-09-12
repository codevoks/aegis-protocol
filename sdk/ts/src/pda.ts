// Canonical PDA derivation, mirroring `docs/account-model.md` exactly -- the same raw seed bytes
// `programs/aegis/src/constants.rs` / `state/*.rs` use on-chain, never a stringification shortcut
// (item 4: "Use raw seed bytes exactly as Rust does. No stringification hacks.").
//
// Seeds (account-model.md §3-6):
//   protocol         [b"protocol"]
//   market           [b"market", collateral_mint, loan_mint, config_id: u16 LE]
//   position         [b"position", market, owner]        (also used for the market's fee_position,
//                                                           with `market.fee_recipient` as `owner`)
//   collateral_vault [b"cvault", market]
//   loan_vault       [b"lvault", market]

import { getAddressEncoder, getProgramDerivedAddress, type Address } from '@solana/kit';

const textEncoder = new TextEncoder();
const addressEncoder = getAddressEncoder();

function addressBytes(a: Address): Uint8Array {
  return Uint8Array.from(addressEncoder.encode(a));
}

function u16LeBytes(n: number): Uint8Array {
  const bytes = new Uint8Array(2);
  new DataView(bytes.buffer).setUint16(0, n, true);
  return bytes;
}

export async function protocolPda(programId: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [textEncoder.encode('protocol')],
  });
  return addr;
}

export async function marketPda(
  programId: Address,
  collateralMint: Address,
  loanMint: Address,
  configId: number,
): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [
      textEncoder.encode('market'),
      addressBytes(collateralMint),
      addressBytes(loanMint),
      u16LeBytes(configId),
    ],
  });
  return addr;
}

/** `PDA([b"position", market, owner])` -- also the correct derivation for a market's
 *  `fee_position` (`account-model.md` §9: `PDA(market, market.fee_recipient)`); pass
 *  `market.feeRecipient` as `owner` for that case rather than a separate function, since the seed
 *  layout is identical. */
export async function positionPda(
  programId: Address,
  market: Address,
  owner: Address,
): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [textEncoder.encode('position'), addressBytes(market), addressBytes(owner)],
  });
  return addr;
}

export async function collateralVaultPda(programId: Address, market: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [textEncoder.encode('cvault'), addressBytes(market)],
  });
  return addr;
}

export async function loanVaultPda(programId: Address, market: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [textEncoder.encode('lvault'), addressBytes(market)],
  });
  return addr;
}

/** Every canonical Aegis PDA for a given `(collateralMint, loanMint, configId)` market, derived in
 *  one call -- the shape most read/UI code actually wants. */
export interface MarketPdas {
  market: Address;
  collateralVault: Address;
  loanVault: Address;
}

export async function deriveMarketPdas(
  programId: Address,
  collateralMint: Address,
  loanMint: Address,
  configId: number,
): Promise<MarketPdas> {
  const market = await marketPda(programId, collateralMint, loanMint, configId);
  const [collateralVault, loanVault] = await Promise.all([
    collateralVaultPda(programId, market),
    loanVaultPda(programId, market),
  ]);
  return { market, collateralVault, loanVault };
}
