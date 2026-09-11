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
  type KeyPairSigner,
  type Rpc,
  type RpcSubscriptions,
  type SolanaRpcApi,
  type SolanaRpcSubscriptionsApi,
  type TransactionWithBlockhashLifetime,
} from '@solana/kit';
import { getSetComputeUnitLimitInstruction } from '@solana-program/compute-budget';

/**
 * Aegis's own oracle-validated instructions (`borrow`, `liquidate`, ...) genuinely need more than
 * the network default 200,000 CU -- the same fact `crates/aegis-test-kit/src/market.rs`'s own
 * `HIGHER_COMPUTE_UNIT_LIMIT`/`LIQUIDATE_COMPUTE_UNIT_LIMIT` constants document, hit here the same
 * way: empirically, via a real `exceeded CUs meter` failure. A single generous default for every
 * transaction this bot sends is simpler than tracking a per-instruction budget, and costs nothing
 * extra on a real cluster beyond the (tiny) priority-fee-eligible compute units actually used.
 */
const DEFAULT_COMPUTE_UNIT_LIMIT = 600_000;

/**
 * Builds, signs, and sends a transaction, waiting for confirmation. Returns the signature on
 * success and throws (with the RPC's own simulation/execution logs attached) on failure -- the
 * keeper is responsible for treating an on-chain rejection as an ordinary, expected outcome
 * (spec #21), not for hiding it.
 */
export async function buildSignAndSend(
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: Instruction[],
): Promise<string> {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
  const computeBudgetIx = getSetComputeUnitLimitInstruction({ units: DEFAULT_COMPUTE_UNIT_LIMIT });

  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(payer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions([computeBudgetIx, ...instructions], m),
  );

  const signedTransaction = await signTransactionMessageWithSigners(message);
  const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });
  // `signTransactionMessageWithSigners`'s return type is inferred as the general
  // `TransactionWithLifetime` union (blockhash OR durable nonce); `sendAndConfirm` narrows to the
  // blockhash-only variant. This message was built with
  // `setTransactionMessageLifetimeUsingBlockhash`, so it genuinely has a blockhash lifetime -- the
  // cast reflects a real, already-established fact about `message` above, not an unchecked escape.
  await sendAndConfirm(signedTransaction as typeof signedTransaction & TransactionWithBlockhashLifetime, {
    commitment: 'confirmed',
  });
  return getSignatureFromTransaction(signedTransaction);
}
