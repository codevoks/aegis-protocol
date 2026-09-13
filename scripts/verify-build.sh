#!/usr/bin/env bash
# I-UPG-03 / INV-UPG-05 (Phase 12): reproduces the verifiable-build evidence recorded in
# docs/project-status.md's Phase 12 section, using the current OtterSec/solana-verify workflow
# (docs/ecosystem-research.md §19.2) -- never apr.dev, which is defunct.
#
# This script is NETWORK-DEPENDENT (Docker build + a live devnet RPC) and is therefore never part
# of `make test`/CI's required path (docs/zero-cost-demo.md §8) -- it is optional, developer-run
# evidence, exactly like `make bench`/`make demo` for their own phases. Requires: Docker running,
# `cargo install solana-verify --locked`, and a devnet-funded keypair as the deployed program's
# current upgrade authority.
#
# What it proves: the program actually live on devnet at the address in `programs/aegis/src/
# lib.rs`'s `declare_id!` is byte-for-byte the same artifact this repository's own source produces
# under a deterministic Docker build -- not merely "the source is public," which says nothing about
# what is actually deployed (governance.md §5).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

PROGRAM_ID="$(grep -oE '"[A-Za-z0-9]{32,44}"' programs/aegis/src/lib.rs | head -1 | tr -d '"')"
RPC_URL="${SOLANA_VERIFY_URL:-https://api.devnet.solana.com}"

echo "verify-build: building deterministically (solana-verify build --arch v1)..."
solana-verify build --library-name aegis --arch v1

LOCAL_HASH="$(solana-verify get-executable-hash target/deploy/aegis.so)"
echo "verify-build: local reproducible-build hash: $LOCAL_HASH"

echo "verify-build: fetching the deployed program's on-chain hash ($PROGRAM_ID @ $RPC_URL)..."
ONCHAIN_HASH="$(solana-verify get-program-hash "$PROGRAM_ID" --url "$RPC_URL")"
echo "verify-build: on-chain program hash: $ONCHAIN_HASH"

if [[ "$LOCAL_HASH" != "$ONCHAIN_HASH" ]]; then
    echo "verify-build: FAILED -- local build does not match the deployed program" >&2
    exit 1
fi

echo "verify-build: OK -- local reproducible build matches the deployed devnet program exactly ($LOCAL_HASH)"
