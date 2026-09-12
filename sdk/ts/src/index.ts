// @aegis/sdk public entry point (docs/phases/phase-09-sdk-ui.md, ADR-0011).
//
// Built on @solana/kit v8.x and @anchor-lang/core. Never @coral-xyz/anchor, never
// @solana/web3.js -- see the SDK README's "no legacy dependencies" proof.

export * from './config.js';
export * from './pda.js';
export * from './accounts.js';
export * from './math.js';
export * from './read.js';
export * from './oracle.js';
export * from './ix.js';
export * from './tx.js';
export * from './errors.js';
export * from './events.js';
export { aegisCoder } from './anchorCoder.js';
export * from './generated/index.js';
