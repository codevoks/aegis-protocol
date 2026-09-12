# Aegis Protocol — Phase 9 app

Local-first Next.js application: market list, position screen, deposit/withdraw/borrow/repay/
supply/withdraw flows, and a scripted demo (seed a market, warp time to show interest accruing,
crash the collateral price, liquidate) — all against a local Surfpool validator.

## Run it

```bash
# from the repo root
make build          # compiles the aegis program, produces target/idl/aegis.json
make sdk-install
make app-install
make app            # starts a local Surfpool, deploys aegis.so, runs the dev server (Ctrl+C stops both)
```

Then open <http://localhost:3000>, wait a moment for the local dev wallet to connect (top right),
click **Airdrop 10 SOL (local only)**, and visit **/demo** to seed a market and run the scripted
lifecycle. **/** lists every market this app's configured RPC endpoint knows about (there are none
until you've seeded one). **/market/[address]** is the position screen for a specific market.

No external network, devnet, mainnet, Hermes, or paid RPC is used or required anywhere in this app.

## Configuration

No hardcoded production/devnet/mainnet endpoint. Overridable via env vars (all optional; defaults
are a local validator):

| Variable | Default |
|---|---|
| `NEXT_PUBLIC_AEGIS_RPC_URL` | `http://127.0.0.1:8899` |
| `NEXT_PUBLIC_AEGIS_RPC_WS_URL` | `ws://127.0.0.1:8900` |
| `NEXT_PUBLIC_AEGIS_PROGRAM_ID` | the address embedded in `@aegis/sdk`'s committed IDL snapshot |

## The local wallet (deliberate scope decision)

This app does not integrate a browser wallet-extension adapter (Phantom/Backpack/etc). See
`src/lib/localSigner.ts`'s doc comment for the full reasoning — in short: `make app`'s clean-clone
acceptance criterion must not depend on the operator having a specific browser extension installed,
and current wallet-adapter packages generally still depend transitively on `@solana/web3.js`
internally. Instead, the app generates an ephemeral Ed25519 keypair with `@solana/kit`'s own
`createKeyPairSignerFromPrivateKeyBytes` and persists its 32-byte seed in this browser's
`localStorage`. **Local-Surfpool-only** — never reuse this pattern against a cluster holding real
funds.

## Why `--webpack`

Next.js 16 defaults to Turbopack. This app's `dev`/`build` scripts pass `--webpack` because
`@aegis/sdk` is consumed as a workspace-local `file:../sdk/ts` dependency shipping TypeScript source
(not a pre-built `dist/`), and resolving that combination (an external directory outside the app's
own root, whose own relative imports use the `.js`-suffixed NodeNext convention) needs
`experimental.externalDir` plus a webpack `resolve.extensionAlias`, both configured in
`next.config.mjs`. Turbopack did not honor the same configuration as of Next.js 16.3.5 (verified
empirically: the Turbopack build failed to resolve `@aegis/sdk` even with `externalDir` set).

## The demo's price-oracle fixtures

Aegis has no on-chain registry mapping a market's configured Pyth `feed_id` to the specific account
address currently holding that feed's price (resolving that is inherently an off-chain concern —
normally Hermes's job). `src/lib/demoRegistry.ts` is a local-only, `localStorage`-backed registry
the `/demo` seed flow populates so the position/market pages can look the right price accounts back
up. `src/lib/priceUpdate.ts` encodes/decodes a Pyth `PriceUpdateV2` account's real byte layout
(cross-checked against the byte-exact fixture `crates/aegis-test-kit/src/pyth_fixture.rs` produces,
the same real-SDK-serialization technique ADR-0008 establishes) — never a mock oracle program.

## Testing this app

```bash
make app-typecheck   # tsc --noEmit, no running validator required
make app-build       # next build --webpack, no running validator required
```

Full user-flow verification (`I-UI-01`) was performed against a real local Surfpool validator
using the Claude Browser tool: market list rendering, the full 3-step demo (seed → warp time /
accrue interest → crash price / liquidate), the position screen's risk-parameter and health-factor
display, and a supply action's honest failure path (insufficient token balance surfaced as a real
on-chain simulation error, never a false success). See `docs/project-status.md`'s Phase 9 evidence
section for the transcript.
