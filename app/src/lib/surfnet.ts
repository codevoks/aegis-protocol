// Surfpool cheat-code RPC methods used only by the local demo (never by `@aegis/sdk` itself, which
// must work against any ordinary RPC node). `surfnet_setAccount` injects a raw account
// (ADR-0008's fixture-injection pattern, the same technique `bots/liquidator/src/surfnet.ts` uses);
// `surfnet_timeTravel` advances the validator's clock without any real wall-clock waiting
// (`docs/phases/phase-09-sdk-ui.md` item 36: "Do not wait in wall-clock real time").

async function rpcCall(rpcUrl: string, method: string, params: unknown[]): Promise<unknown> {
  const resp = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  const json = (await resp.json()) as { result?: unknown; error?: { message: string } };
  if (json.error) throw new Error(`${method} failed: ${json.error.message}`);
  return json.result;
}

export async function surfnetSetAccount(
  rpcUrl: string,
  pubkey: string,
  owner: string,
  dataBase64: string,
  lamports: number,
): Promise<void> {
  const dataHex = Buffer.from(dataBase64, 'base64').toString('hex');
  await rpcCall(rpcUrl, 'surfnet_setAccount', [
    pubkey,
    { lamports, data: dataHex, owner, executable: false },
  ]);
}

/** Advances the validator's clock by `forwardSeconds`, so interest can be shown accruing without
 *  any real wall-clock wait. */
export async function surfnetWarpForward(rpcUrl: string, forwardSeconds: number): Promise<void> {
  const absoluteTimestamp = Date.now() + forwardSeconds * 1000;
  await rpcCall(rpcUrl, 'surfnet_timeTravel', [{ absoluteTimestamp }]);
}

const CLOCK_SYSVAR_ADDRESS = 'SysvarC1ock11111111111111111111111111111111';

/** Reads the validator's ACTUAL current `Clock.unix_timestamp` (the sysvar every Aegis oracle
 *  check reads, `oracle-design.md` O-5/O-6) rather than the browser's own wall-clock time --
 *  required after `surfnetWarpForward`, since time-travel moves only the validator's clock, not
 *  the host machine's. Layout: `slot:u64, epoch_start_timestamp:i64, epoch:u64,
 *  leader_schedule_epoch:u64, unix_timestamp:i64` (40 bytes; `unix_timestamp` is the last 8). */
export async function getOnChainUnixTimestamp(rpcUrl: string): Promise<bigint> {
  const result = (await rpcCall(rpcUrl, 'getAccountInfo', [
    CLOCK_SYSVAR_ADDRESS,
    { encoding: 'base64' },
  ])) as { value: { data: [string, string] } | null };
  if (!result.value) throw new Error('Clock sysvar not found');
  const data = Buffer.from(result.value.data[0], 'base64');
  return data.readBigInt64LE(32);
}
