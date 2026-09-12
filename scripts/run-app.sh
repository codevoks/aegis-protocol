#!/usr/bin/env bash
# `make app` (docs/phases/phase-09-sdk-ui.md item 46-47): one command, clean-clone developer
# experience. Starts a local, offline Surfpool validator, deploys the already-built `aegis.so`
# (run `make build` first), then runs the Next.js dev server in the foreground. Ctrl+C stops the
# dev server and the background validator together.
#
# Prerequisites (documented, not hidden): `make build` has been run so `target/deploy/aegis.so`
# and `target/idl/aegis.json` exist; `npm install` has been run in `sdk/ts/` and `app/`
# (`make sdk-install app-install`, or plain `npm install` in each); `surfpool`/`solana`/
# `solana-keygen` are on PATH (the same toolchain `make setup` verifies).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

RPC_PORT=8899
WS_PORT=8900
RPC_URL="http://127.0.0.1:${RPC_PORT}"

if [ ! -f target/deploy/aegis.so ] || [ ! -f target/idl/aegis.json ]; then
    echo "run-app: target/deploy/aegis.so or target/idl/aegis.json is missing -- run 'make build' first." >&2
    exit 1
fi

DEPLOYER_KEYPAIR=$(mktemp -t aegis-app-deployer.json)
cleanup() {
    if [ -n "${SURFPOOL_PID:-}" ]; then
        kill "$SURFPOOL_PID" 2>/dev/null || true
    fi
    rm -f "$DEPLOYER_KEYPAIR"
}
trap cleanup EXIT INT TERM

echo "run-app: starting a local, offline Surfpool validator on port ${RPC_PORT}..."
surfpool start --offline --port "$RPC_PORT" --ws-port "$WS_PORT" --no-tui >/tmp/aegis-app-surfpool.log 2>&1 &
SURFPOOL_PID=$!

echo "run-app: waiting for the validator to become healthy..."
for _ in $(seq 1 60); do
    if curl -s -X POST "$RPC_URL" -H 'Content-Type: application/json' \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q '"result":"ok"'; then
        break
    fi
    sleep 0.5
done

echo "run-app: deploying aegis.so..."
solana-keygen new --no-bip39-passphrase --silent --force -o "$DEPLOYER_KEYPAIR"
solana airdrop 20 --url "$RPC_URL" --keypair "$DEPLOYER_KEYPAIR" >/dev/null
solana program deploy target/deploy/aegis.so \
    --program-id target/deploy/aegis-keypair.json \
    --url "$RPC_URL" \
    --keypair "$DEPLOYER_KEYPAIR" >/dev/null

echo "run-app: aegis program deployed. Starting the Next.js dev server (Ctrl+C to stop everything)..."
npm --prefix app run dev
