# Aegis liquidator keeper (Phase 8)

A TypeScript keeper that scans Aegis positions, estimates health off-chain, and submits direct or
callback liquidations (`docs/phases/phase-08-composability.md`, `docs/composability.md`).

**Stack** (ADR-0011, frozen): [`@solana/kit`](https://www.npmjs.com/package/@solana/kit) for
RPC/transaction/signing, [`@anchor-lang/core`](https://www.npmjs.com/package/@anchor-lang/core)
for the real IDL coder only (never its `Program`/`Provider` layer — that would pull in legacy
`@solana/web3.js` internals, which ADR-0011 rejects). See `src/anchorCore.ts` for the exact,
empirically-verified conventions this coder needs (raw snake_case names, `BN`/`.toBuffer()`
argument and return shims) — verified against the installed packages, not assumed from memory
(`AGENTS.md` §11).

## On-chain Aegis is authoritative — this bot is advisory only

Every health-factor estimate here (`src/health.ts`) is a cheap, off-chain filter for picking
*candidates* before spending a transaction. It is never trusted for the actual liquidation
decision: `liquidate` re-validates the oracle and re-computes health from scratch on-chain, under
its own strict rules (`INV-ORA-07`, `INV-LIQ-01`). This keeper is built to tolerate — as ordinary,
expected outcomes, never bugs — a stale local read, a position that became healthy before the
transaction landed, another liquidator winning the race, an oracle price that moved, or a trial
repay amount the chain's own `max_repay` bound rejects.

## Structure

| File | Role |
|---|---|
| `src/config.ts` | RPC/program-id/market config — no secrets, no hardcoded devnet/mainnet endpoint |
| `src/signer.ts` | Loads a local keypair file, or generates an ephemeral one (no committed keys, `AGENTS.md` §19) |
| `src/idl.ts` | Loads the real `target/idl/aegis.json` (never hand-maintained) |
| `src/anchorCore.ts` | The `@anchor-lang/core` CJS/ESM interop + coder-argument shims, centralized once |
| `src/pda.ts` | PDA derivation mirroring `docs/account-model.md` |
| `src/scan.ts` | Account discovery + decoding (`Market`, `Position`) |
| `src/health.ts` | Off-chain, advisory-only health-factor estimate (exact `BigInt` math, no floating point) |
| `src/txBuilders.ts` | Builds Aegis instructions from the IDL coder + explicit account-meta lists |
| `src/send.ts` | Build → sign → send → confirm |
| `src/keeper.ts` | The scan → filter → build → send loop |
| `src/index.ts` | CLI entry point (`npm run keeper`) |
| `src/demo.ts` | The required local, no-network demonstration (`npm run demo`) |

## Running the keeper

```bash
export AEGIS_RPC_URL=http://127.0.0.1:8899        # your own validator/cluster
export AEGIS_MARKETS=<comma-separated Market addresses>
npm install
npm run keeper
```

No secrets are read from environment variables. `AEGIS_KEEPER_KEYPAIR_PATH` may point to a local
keypair file (never commit it); otherwise the keeper generates and uses an ephemeral signer for
the process lifetime.

**Oracle prices and callback funding are pluggable, not built in.** `src/index.ts`'s default
`PriceResolver`/`CallbackProvider` are placeholders that throw/no-op respectively — a real
deployment supplies its own: fetch a fresh Pyth Hermes price update and post it (network-dependent,
deliberately out of this bot's scope, ADR-0008/`docs/ecosystem-research.md` §16.2), and decide when
to route through a callback based on the keeper's own loan-asset balance. `src/demo.ts` implements
both concretely against its own local fixtures — read it for a complete, working example.

## Local demo (required, no network)

```bash
# from the repo root:
make build
# then:
cd bots/liquidator
npm install
npm run demo
```

This starts a **plain, non-forking** local Surfpool validator (`surfpool start --offline` —
verified to make no outbound network calls), deploys the real `aegis.so` and
`example_liquidator.so`, bootstraps a protocol/market/unhealthy position from scratch, and runs
this same keeper logic to find and execute a callback liquidation for a liquidator holding **zero**
loan-asset balance — proving the callback genuinely removes the pre-funding requirement
(`docs/composability.md` §1), and that this keeper can drive it end to end. No devnet, no mainnet
RPC, no Jupiter, no paid API.

`npm run fixtures` regenerates the Pyth price fixtures the demo injects (via Surfpool's
`surfnet_setAccount` cheat-code RPC method) from the real byte-exact fixture logic in
`crates/aegis-test-kit/src/pyth_fixture.rs` — Pyth's account layout is never reimplemented in
TypeScript.

## Security

- No committed keypairs, no committed secrets (`AGENTS.md` §19).
- No hardcoded RPC endpoint defaults beyond `http://127.0.0.1:8899` (a local validator).
- Every liquidation attempt is a normal transaction subject to normal simulation/execution; a
  rejection is logged and the keeper moves on to the next candidate.
