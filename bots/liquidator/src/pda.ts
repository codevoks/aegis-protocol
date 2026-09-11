// PDA derivation, mirroring `docs/account-model.md` exactly (same seeds as
// `crates/aegis-test-kit/src/market.rs`'s Rust helpers -- kept in sync by hand since there is no
// shared codegen between the Rust program and this bot; the seed layout is frozen, so drift risk
// is low, but a future Phase 9 SDK should generate both from one source).

import { getProgramDerivedAddress, type Address } from '@solana/kit';

const te = new TextEncoder();

export async function protocolPda(programId: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [te.encode('protocol')],
  });
  return addr;
}

export async function marketPda(
  programId: Address,
  collateralMint: Address,
  loanMint: Address,
  configId: number,
): Promise<Address> {
  const configIdBytes = new Uint8Array(2);
  new DataView(configIdBytes.buffer).setUint16(0, configId, true);
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [te.encode('market'), addressBytes(collateralMint), addressBytes(loanMint), configIdBytes],
  });
  return addr;
}

export async function positionPda(
  programId: Address,
  market: Address,
  owner: Address,
): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [te.encode('position'), addressBytes(market), addressBytes(owner)],
  });
  return addr;
}

export async function collateralVaultPda(programId: Address, market: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [te.encode('cvault'), addressBytes(market)],
  });
  return addr;
}

export async function loanVaultPda(programId: Address, market: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [te.encode('lvault'), addressBytes(market)],
  });
  return addr;
}

export async function exampleLiquidatorAuthorityPda(
  exampleLiquidatorProgramId: Address,
): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress: exampleLiquidatorProgramId,
    seeds: [te.encode('authority')],
  });
  return addr;
}

// `Address` is a base58 string at the type level; `getProgramDerivedAddress` and the codecs
// package both work in bytes for seed material. `getAddressEncoder` is the officially exported
// way to get from one to the other (`@solana/addresses`, re-exported by `@solana/kit`).
import { getAddressEncoder } from '@solana/kit';
const addressEncoder = getAddressEncoder();
function addressBytes(a: Address): Uint8Array {
  return Uint8Array.from(addressEncoder.encode(a));
}
