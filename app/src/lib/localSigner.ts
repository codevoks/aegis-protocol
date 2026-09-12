'use client';

// Local development wallet/signer (`docs/phases/phase-09-sdk-ui.md` item 27: "support a
// local-compatible wallet/signer connection using current Solana Kit-compatible frontend
// patterns").
//
// DELIBERATE SCOPE DECISION, recorded here rather than silently made: this app does not integrate
// a browser wallet-extension adapter (Phantom/Backpack/etc). A real wallet-adapter integration is
// out of scope for Phase 9's local-first acceptance path -- most current wallet-adapter packages
// still depend transitively on `@solana/web3.js` internally (the same class of unavoidable
// transitive dependency `@anchor-lang/core` itself has, see the SDK README), and more
// fundamentally, `make app`'s clean-clone acceptance criterion must not depend on the operator
// having a specific browser extension installed. Instead, the app generates an ephemeral Ed25519
// keypair with `@solana/kit`'s own first-party `createKeyPairSignerFromPrivateKeyBytes` (never a
// legacy dependency, never a hand-rolled key format) and persists its 32-byte seed in
// `localStorage`, scoped to this browser origin, so a reload keeps using the same local identity.
// This is explicitly a LOCAL-SURFPOOL-ONLY convenience: the seed never leaves the browser, is
// never sent anywhere, and this pattern must never be reused against a real cluster holding real
// funds.

import { createKeyPairSignerFromPrivateKeyBytes, type KeyPairSigner } from '@solana/kit';

const STORAGE_KEY = 'aegis-local-dev-signer-seed-b64';

function readStoredSeed(): Uint8Array | null {
  try {
    const b64 = window.localStorage.getItem(STORAGE_KEY);
    if (!b64) return null;
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    return bytes.length === 32 ? bytes : null;
  } catch {
    return null; // private-browsing / storage disabled -- fall back to generating fresh
  }
}

function storeSeed(bytes: Uint8Array): void {
  try {
    const b64 = btoa(String.fromCharCode(...bytes));
    window.localStorage.setItem(STORAGE_KEY, b64);
  } catch {
    // best-effort only; the signer still works for this page load even if it cannot persist
  }
}

/** Loads the persisted local dev signer, or generates and persists a new 32-byte seed. */
export async function loadOrCreateLocalSigner(): Promise<KeyPairSigner> {
  let seed = readStoredSeed();
  if (!seed) {
    seed = crypto.getRandomValues(new Uint8Array(32));
    storeSeed(seed);
  }
  return createKeyPairSignerFromPrivateKeyBytes(seed);
}

/** Clears the persisted local signer -- lets the demo UI offer a "new local wallet" reset. */
export function clearLocalSigner(): void {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // no-op
  }
}
