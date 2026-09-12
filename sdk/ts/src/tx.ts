// Transaction lifecycle helpers: build -> sign -> send -> confirm -> decode. The SDK never owns a
// signer (item 17) -- every function here takes a caller-supplied `TransactionSigner` (a
// `KeyPairSigner`, or any wallet-adapter-compatible signer implementing the same interface) and
// never persists, generates for the user, or reads a private key from disk.

import {
  appendTransactionMessageInstructions,
  createTransactionMessage,
  getSignatureFromTransaction,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Instruction,
  type TransactionSigner,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
  type TransactionWithBlockhashLifetime,
} from '@solana/kit';
import { getSetComputeUnitLimitInstruction } from '@solana-program/compute-budget';
import { decodeEventsFromLogs, type DecodedEvent } from './events.js';
import { decodeError } from './errors.js';

/** Aegis's oracle-validated instructions (`borrow`, `liquidate`, ...) need more than the network
 *  default 200,000 CU -- the same fact `crates/aegis-test-kit`'s own
 *  `HIGHER_COMPUTE_UNIT_LIMIT`/`LIQUIDATE_COMPUTE_UNIT_LIMIT` constants and
 *  `bots/liquidator/src/send.ts` both already document (INV-RES-01 itself is Phase 11 scope). One
 *  generous default per transaction is simpler than a per-instruction budget table. */
export const DEFAULT_COMPUTE_UNIT_LIMIT = 600_000;

export type ConfirmationStage =
  | 'building'
  | 'awaiting-signature'
  | 'submitted'
  | 'confirming'
  | 'confirmed'
  | 'failed';

export interface BuildSignSendOptions {
  computeUnitLimit?: number;
  onStage?: (stage: ConfirmationStage) => void;
}

export interface SentTransactionResult {
  signature: string;
  /** Populated only once the transaction is actually confirmed -- callers must never treat
   *  `signature` alone as success (`docs/phases/phase-09-sdk-ui.md` item 43: "avoid displaying
   *  success before confirmation"). */
  events: DecodedEvent[];
}

/**
 * Builds, signs, sends, and confirms a transaction, then decodes its emitted Aegis events from the
 * confirmed transaction's logs. Throws (after decoding any recognized Aegis program error via
 * `errors.ts`) on simulation/execution failure -- never resolves with a signature for a transaction
 * that did not actually land.
 */
export async function buildSignSendAndConfirm(
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  feePayer: TransactionSigner,
  instructions: Instruction[],
  options: BuildSignSendOptions = {},
): Promise<SentTransactionResult> {
  const { computeUnitLimit = DEFAULT_COMPUTE_UNIT_LIMIT, onStage } = options;
  onStage?.('building');

  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
  const computeBudgetIx = getSetComputeUnitLimitInstruction({ units: computeUnitLimit });

  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(feePayer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions([computeBudgetIx, ...instructions], m),
  );

  onStage?.('awaiting-signature');
  const signedTransaction = await signTransactionMessageWithSigners(message);

  onStage?.('submitted');
  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

  onStage?.('confirming');
  try {
    await sendAndConfirm(
      signedTransaction as typeof signedTransaction & TransactionWithBlockhashLifetime,
      { commitment: 'confirmed' },
    );
  } catch (err) {
    onStage?.('failed');
    throw decodeError(err);
  }

  const signature = getSignatureFromTransaction(signedTransaction);
  onStage?.('confirmed');

  let events: DecodedEvent[] = [];
  try {
    const txResp = await rpc
      .getTransaction(signature, {
        commitment: 'confirmed',
        maxSupportedTransactionVersion: 0,
        encoding: 'json',
      })
      .send();
    const logs = txResp?.meta?.logMessages ?? [];
    events = decodeEventsFromLogs(logs);
  } catch {
    // Event decoding is best-effort observability, never load-bearing for the result: the
    // transaction already confirmed above regardless of whether logs were retrievable afterward.
  }

  return { signature, events };
}
