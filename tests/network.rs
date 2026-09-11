//! Entry point for optional, network-tagged tests (`docs/phases/phase-08-composability.md`,
//! `AGENTS.md` §16 zero-cost requirement). Every test reachable from this file is `#[ignore]`d and
//! therefore excluded from `cargo test --workspace` / `make test` by default — `docs/
//! zero-cost-demo.md` §8's anti-patterns list network dependencies in a *required* path as the
//! specific thing never to do; this file exists precisely so such tests have a home that is
//! structurally incapable of being required.

mod jupiter_route;
