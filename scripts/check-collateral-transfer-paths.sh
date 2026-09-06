#!/usr/bin/env bash
# CI-CUSTODY-PATHS (A-CUS-04 / INV-CUS-04) — asserts every token movement out of a vault, signed
# by the Market PDA, goes through the one shared helper (`token::transfer::transfer_checked_out`),
# and every token movement into a vault goes through the one shared inbound helper
# (`token::transfer::transfer_checked_in`) — and that each helper is called only from the
# enumerated instructions that are supposed to move tokens (account-model.md §6.3 enumerates the
# complete, six-path custody surface). Phase 3 added the two collateral paths
# (`deposit_collateral`/`withdraw_collateral`); Phase 4 added the three loan-side paths that move
# real tokens (`supply`, `withdraw`, `repay`); Phase 5 enables the real, oracle-validated `borrow`
# path (`loan_vault -> owner`, account-model.md §6.3's fourth row), so `borrow.rs` now legitimately
# joins the outbound allowlist below. A call to `transfer_checked_out`/`transfer_checked_in`
# anywhere else, or a raw `token_interface::transfer_checked` call bypassing both helpers, would be
# a new, unaudited custody path.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail=0

allowed_out_callers=$(cat <<'EOF'
programs/aegis/src/instructions/borrow/borrow.rs
programs/aegis/src/instructions/collateral/withdraw_collateral.rs
programs/aegis/src/instructions/lend/withdraw.rs
EOF
)

allowed_in_callers=$(cat <<'EOF'
programs/aegis/src/instructions/collateral/deposit_collateral.rs
programs/aegis/src/instructions/lend/supply.rs
programs/aegis/src/instructions/borrow/repay.rs
EOF
)

out_callers=$(grep -rl 'transfer_checked_out(' programs/aegis/src --include='*.rs' \
    | grep -v 'programs/aegis/src/token/transfer.rs' | sort || true)
if [ "$out_callers" != "$(echo "$allowed_out_callers" | sort)" ]; then
    echo "check-collateral-transfer-paths: transfer_checked_out call sites do not match the allowlist:" >&2
    echo "expected:" >&2
    echo "$allowed_out_callers" >&2
    echo "found:" >&2
    echo "$out_callers" >&2
    fail=1
fi

in_callers=$(grep -rl 'transfer_checked_in(' programs/aegis/src --include='*.rs' \
    | grep -v 'programs/aegis/src/token/transfer.rs' | sort || true)
if [ "$in_callers" != "$(echo "$allowed_in_callers" | sort)" ]; then
    echo "check-collateral-transfer-paths: transfer_checked_in call sites do not match the allowlist:" >&2
    echo "expected:" >&2
    echo "$allowed_in_callers" >&2
    echo "found:" >&2
    echo "$in_callers" >&2
    fail=1
fi

# No instruction may bypass the shared helpers and call the raw CPI directly.
raw_callers=$(grep -rl 'token_interface::transfer_checked(' programs/aegis/src --include='*.rs' \
    | grep -v 'programs/aegis/src/token/transfer.rs' || true)
if [ -n "$raw_callers" ]; then
    echo "check-collateral-transfer-paths: raw token_interface::transfer_checked call outside token/transfer.rs:" >&2
    echo "$raw_callers" >&2
    fail=1
fi

if [ "$fail" -ne 0 ]; then
    exit 1
fi

echo "check-collateral-transfer-paths: OK — vault token movement goes through exactly the shared helpers, from exactly their enumerated call sites"
