// Keeper signer loading (AGENTS.md §19: no committed keypairs, ever).

import { readFileSync } from 'node:fs';
import {
  createKeyPairSignerFromBytes,
  generateKeyPairSigner,
  type KeyPairSigner,
} from '@solana/kit';

/**
 * Loads the keeper's signer from `keypairPath` (a `solana-keygen`-format JSON array of 64 secret
 * key bytes) if given, otherwise generates a fresh, ephemeral keypair for this process only.
 *
 * The ephemeral default is deliberate: this keeper never needs a stable identity to function
 * (liquidation is permissionless, `docs/account-model.md` §5.1), so the safest default is a key
 * that cannot leak because it is never written anywhere.
 */
export async function loadOrGenerateSigner(keypairPath: string | undefined): Promise<KeyPairSigner> {
  if (!keypairPath) {
    return generateKeyPairSigner();
  }
  const raw = JSON.parse(readFileSync(keypairPath, 'utf8')) as number[];
  return createKeyPairSignerFromBytes(new Uint8Array(raw));
}
