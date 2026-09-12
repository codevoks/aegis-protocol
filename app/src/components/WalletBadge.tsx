'use client';

import { useAegisClient } from '@/lib/AegisProvider';
import { formatBaseUnitsToDecimal } from '@/lib/format';

export function WalletBadge() {
  const { signer, balanceLamports, airdrop } = useAegisClient();

  if (!signer) return <span className="wallet-badge">connecting…</span>;

  const short = `${signer.address.slice(0, 4)}…${signer.address.slice(-4)}`;
  const sol = balanceLamports === null ? '—' : formatBaseUnitsToDecimal(balanceLamports, 9);

  return (
    <div className="wallet-badge">
      <span title={signer.address} className="wallet-address">
        {short}
      </span>
      <span className="wallet-balance">{sol} SOL</span>
      <button type="button" onClick={() => void airdrop(10)} className="btn-small">
        Airdrop 10 SOL (local only)
      </button>
    </div>
  );
}
