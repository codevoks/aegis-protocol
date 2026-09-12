//! Phase 10 exploit-regression suite (`docs/phases/phase-10-security.md`): every bug found during
//! the security campaign is frozen here as a permanent, named test citing its threat/invariant ID
//! and its finding in `docs/security/findings.md`, fixed only after the regression test exists and
//! is confirmed to fail against the vulnerable code (`docs/security/mutation-report.md`'s
//! bug-fix-order discipline).

#[path = "adversarial/dust_debt.rs"]
mod dust_debt;
