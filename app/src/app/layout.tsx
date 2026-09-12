import type { Metadata } from 'next';
import { AegisProvider } from '@/lib/AegisProvider';
import { WalletBadge } from '@/components/WalletBadge';
import './globals.css';

export const metadata: Metadata = {
  title: 'Aegis Protocol',
  description: 'Phase 9 local demo app -- markets, positions, and the liquidation demo.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <AegisProvider>
          <div className="shell">
            <header className="topbar">
              <a href="/" className="brand">
                Aegis
              </a>
              <nav>
                <a href="/">Markets</a>
                <a href="/demo">Demo</a>
              </nav>
              <WalletBadge />
            </header>
            <main className="content">{children}</main>
          </div>
        </AegisProvider>
      </body>
    </html>
  );
}
