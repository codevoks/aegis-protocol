# Architecture Decision Records

An ADR is required for: a change to any frozen document · a consequential new dependency · a deviation
from a phase specification · a change to the account model, economics, invariants, or security posture ·
a rejected alternative worth recording. Routine implementation choices do not need one.

**An ADR that does not state what was rejected and why is not finished.**

| # | Decision | Status |
|---|---|---|
| [0001](0001-anchor-as-production-framework.md) | Anchor as the production framework | Accepted |
| [0002](0002-test-stack.md) | LiteSVM-primary test stack | Accepted |
| [0003](0003-native-pinocchio-as-labs.md) | Native Rust and Pinocchio as scoped labs, not production | Accepted |
| [0004](0004-isolated-markets.md) | **Isolated two-asset markets** | Accepted |
| [0005](0005-collateral-escrow-and-vault-design.md) | Collateral never lent; explicit PDA vaults | Accepted |
| [0006](0006-peer-to-pool-internal-shares.md) | Peer-to-pool with internal shares, no share token | Accepted |
| [0007](0007-stateless-irm.md) | Stateless piecewise-linear IRM | Accepted |
| [0008](0008-oracle-abstraction-no-mock-program.md) | **Oracle abstraction; no mock oracle program** | Accepted |
| [0009](0009-fixed-point-representation.md) | WAD fixed point, 256-bit mul-div intermediates | Accepted |
| [0010](0010-zero-cost-architecture.md) | Zero-cost, local-first architecture | Accepted |
| [0011](0011-client-stack.md) | `@solana/kit` client stack | Accepted |
| [0012](0012-upgrade-authority-strategy.md) | Progressive upgrade hardening; bounded admin power | Accepted |

The three most consequential are **0004** (which shapes the whole account model), **0008** (which
shapes both security and testability), and **0005** (which makes custody exactly reconcilable).
