// Cheat-code helper for injecting a raw account directly into a local Surfpool validator, used
// ONLY to seed deterministic Pyth price-update fixtures for the offline demo (`src/demo.ts`) --
// exactly the same technique `aegis_test_kit::pyth_fixture` uses via LiteSVM's `set_account`, just
// over RPC instead of in-process. `surfnet_setAccount`'s `data` parameter is hex-encoded (verified
// empirically against the installed Surfpool binary: it rejects base64 with "Invalid hex data").

export interface SurfnetAccount {
  pubkey: string;
  owner: string;
  lamports: number;
  dataBase64: string;
}

export async function surfnetSetAccount(rpcUrl: string, account: SurfnetAccount): Promise<void> {
  const dataHex = Buffer.from(account.dataBase64, 'base64').toString('hex');
  const resp = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'surfnet_setAccount',
      params: [
        account.pubkey,
        { lamports: account.lamports, data: dataHex, owner: account.owner, executable: false },
      ],
    }),
  });
  const json = (await resp.json()) as { error?: { message: string } };
  if (json.error) {
    throw new Error(`surfnet_setAccount(${account.pubkey}) failed: ${json.error.message}`);
  }
}
