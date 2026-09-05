You are acting as the principal protocol architect, Solana engineer, DeFi systems designer, and security reviewer for a new project called Aegis Protocol.

Your job in this session is PLANNING ONLY.

Do not implement the protocol.
Do not create production Rust instructions.
Do not scaffold application code merely to make progress.
Do not start Phase 1.

Spend your effort on architecture, economics, security, tooling verification, phase design, repository rules, and implementation specifications that a strong implementation model can later execute phase-by-phase.

The next implementation sessions may use a faster model such as Claude Sonnet. Therefore your output must be sufficiently rigorous, explicit, internally consistent, and actionable that an implementation model should not need to redesign the system while coding.

⸻

PROJECT AMBITION

Aegis is not a tutorial, hackathon submission, toy lending protocol, or keyword-filled portfolio project.

Treat it as:

A serious Solana DeFi protocol that could plausibly evolve into the technical foundation of a real startup.

The public GitHub repository should eventually demonstrate that its engineer can independently reason about:

* Solana runtime and account architecture;
* Rust;
* Anchor;
* PDAs and CPIs;
* SPL Token and Token-2022;
* DeFi economics;
* lending;
* collateralization;
* liquidations;
* oracle safety;
* protocol security;
* adversarial testing;
* compute optimization;
* SDK/frontend integration;
* upgrade/governance concerns;
* protocol observability and operational readiness.

Do not maximize complexity for its own sake.

Prefer coherent product depth over superficial feature count.

⸻

PRODUCT THESIS

Aegis is a risk-first overcollateralized lending protocol on Solana.

A user should eventually be able to:

* deposit supported collateral;
* create/manage a collateralized position;
* borrow supported assets;
* repay debt;
* withdraw collateral subject to solvency rules;
* inspect LTV / health;
* be liquidated permissionlessly when unsafe;
* interact with SPL Token assets;
* interact with selected Token-2022 assets where doing so is meaningful;
* use reliable oracle-backed prices;
* interact through a typed SDK and product UI;
* compose with selected Solana protocols when there is a genuine product reason.

Do NOT transform Aegis into a combined AMM + perpetual exchange + stablecoin + lending + NFT + staking protocol merely to increase topic coverage.

Additional DeFi mechanics should only exist when they naturally support Aegis.

Example:

External swap/liquidity integration may later support liquidation routing.

That is useful composability.

Building an unrelated AMM inside Aegis simply to claim “AMM knowledge” is not.

⸻

FIRST RESPONSIBILITY: CHALLENGE THE PRODUCT

Do not assume the proposed design is optimal.

Before designing implementation details, critically evaluate:

1. Is overcollateralized lending the right core product for maximizing serious Solana engineering evidence?
2. Is the product economically coherent?
3. Which features genuinely belong in Aegis?
4. Which topics should instead be demonstrated through:
    * focused labs;
    * benchmarks;
    * adapters;
    * ADRs;
    * security experiments?
5. What should deliberately remain out of scope?
6. Can this architecture plausibly evolve beyond a portfolio project?

If you materially improve the product thesis, document the change and reasoning.

⸻

CURRENT-ECOSYSTEM RESEARCH

Before finalizing architecture, verify current Solana ecosystem guidance from authoritative/current sources.

Research at minimum:

* current stable Solana toolchain;
* Anchor;
* native Solana Rust development;
* Pinocchio status and maturity;
* SPL Token;
* Token-2022 / Token Extensions;
* current recommended Solana TypeScript client stack;
* current local validator/testing options;
* LiteSVM;
* Surfpool if relevant;
* current Pyth Solana integration;
* current recommended oracle safety practices;
* transaction/compute behavior relevant to this protocol;
* current Solana security guidance.

Prefer official documentation and primary sources.

Record:

* research date;
* important versions;
* relevant URLs/references;
* anything unstable/deprecated;
* decisions affected by that research.

Do not copy outdated tutorials blindly.

⸻

DEFAULT TECHNOLOGY DIRECTION

Unless research provides a strong reason to change it:

Production on-chain protocol:

* Rust
* Anchor

Low-level coverage:

* native Solana Rust and/or Pinocchio through deliberately scoped labs, benchmarks, or comparative implementations.

Token standards:

* SPL Token
* selected Token-2022 extensions where meaningful.

Client:

* current recommended official Solana TypeScript stack, currently expected to prefer @solana/kit for new architecture.

Oracle:

* deterministic local oracle abstraction;
* real current Pyth integration adapter.

Testing:

* Rust unit/property tests;
* Anchor/program integration tests;
* LiteSVM where appropriate;
* full local-validator/Surfpool testing where actual runtime behavior matters.

Do not force unstable tooling into production architecture just to claim coverage.

⸻

ZERO-COST REQUIREMENT

A complete meaningful Aegis demonstration must be possible without paying for external infrastructure.

The required development/demo path should work using:

* local Solana environment;
* deterministic locally-created token mints;
* deterministic local accounts;
* deterministic oracle;
* local tests;
* local frontend/client later.

Devnet integrations may exist.

Paid RPC/oracle/cloud providers may optionally exist.

They must NOT be required to:

* run tests;
* demonstrate core functionality;
* verify security properties;
* evaluate the repository.

⸻

ECONOMIC DESIGN

Do not write protocol code before the economic model is explicitly designed.

Define:

* collateral assets;
* debt assets;
* collateral valuation;
* debt representation;
* fixed-point representation;
* decimals policy;
* LTV;
* maximum borrow limit;
* liquidation threshold;
* health factor;
* liquidation amount rules;
* close-factor if applicable;
* liquidation bonus/penalty;
* protocol fees;
* reserve model;
* interest/index model;
* interest accrual semantics;
* rounding directions;
* bad-debt behavior;
* insolvency assumptions;
* oracle validity rules;
* administrative parameter bounds.

For every economically important formula include:

* mathematical definition;
* units;
* rounding policy;
* worked example;
* edge cases;
* expected property/invariant tests.

No floating-point arithmetic on-chain.

Explicitly identify which economic choices are intentionally simplified for Aegis v1 and which would require deeper research before a production launch with real capital.

Do not pretend a portfolio-grade economic model is automatically production-safe.

⸻

ACCOUNT / STATE DESIGN

Design the Solana account model before implementation.

For every proposed account provide:

* purpose;
* owner;
* PDA seeds if applicable;
* bump handling;
* authority;
* mutable fields;
* approximate size;
* lifecycle;
* initialization;
* reallocation requirements;
* close behavior;
* concurrency implications;
* whether it creates a hot writable account.

Candidate concepts may include:

* protocol/config;
* asset/reserve configuration;
* market;
* collateral vault;
* debt vault;
* user position;
* oracle configuration;
* interest state;
* protocol fee destination.

Do not assume these are all necessary.

Minimize shared writable global state.

Explicitly reason about Sealevel parallelism/account contention.

⸻

INSTRUCTION CATALOGUE

Produce the planned instruction/state-transition catalogue.

For each instruction eventually document:

* caller;
* required signer;
* accounts;
* writable accounts;
* PDAs;
* token programs;
* trusted external programs;
* preconditions;
* state transition;
* token movement;
* arithmetic;
* emitted events;
* invariants affected;
* important failure cases;
* potential attack vectors.

Likely operations may include:

* initialize protocol/market;
* configure asset;
* initialize user position;
* deposit collateral;
* withdraw collateral;
* borrow;
* repay;
* liquidate;
* collect/withdraw protocol fees;
* update controlled configuration.

Challenge whether each operation is needed.

⸻

TOKEN CUSTODY MODEL

Define exactly how protocol-controlled assets work.

For every vault:

* mint;
* token program;
* authority PDA;
* ATA vs explicit token account decision;
* seeds;
* signer mechanism;
* withdrawal paths;
* freeze/mint authority assumptions;
* accounting reconciliation.

Explicitly consider malicious substitution of:

* vault;
* mint;
* token account;
* token program;
* authority;
* CPI target.

⸻

TOKEN-2022 DESIGN

Do not treat Token-2022 as a resume checkbox.

Research current extensions and identify a small number whose behavior creates legitimate engineering/security considerations.

Potential areas include:

* transfer fees;
* transfer hooks;
* metadata;
* CPI guard;
* permanent delegate;
* immutable owner;
* pausable behavior;
* other currently relevant extensions.

Analyze compatibility implications.

A Token-2022 asset may not behave identically to a classic SPL Token transfer.

Design explicit compatibility policy:

* fully supported;
* supported with constraints;
* unsupported.

Avoid pretending arbitrary Token-2022 mints are automatically safe collateral.

⸻

ORACLE ARCHITECTURE

Design an oracle interface supporting:

1. deterministic local oracle;
2. real Pyth adapter.

Define oracle validity checks including:

* feed identity;
* publish timestamp;
* maximum acceptable staleness;
* confidence;
* fixed-point conversion;
* decimal normalization;
* invalid/missing data;
* unavailable oracle;
* suspicious/confidence-wide data.

Threat-model:

* stale prices;
* delayed prices;
* extreme volatility;
* oracle downtime;
* manipulation assumptions;
* price/confidence misuse;
* mismatched feed.

Determine what Aegis should do when oracle validity fails.

Fail-open behavior should require extremely strong justification.

⸻

SECURITY-FIRST ARCHITECTURE

Build a threat model before implementation.

Consider at minimum:

* missing signer checks;
* incorrect owner validation;
* arbitrary accounts;
* account substitution;
* wrong token mint;
* wrong token program;
* wrong CPI target;
* fake oracle account;
* PDA seed sharing;
* bump mistakes;
* duplicate mutable accounts;
* account reinitialization;
* unsafe close;
* stale account state after CPI;
* privilege propagation through CPI;
* integer overflow;
* integer truncation;
* fixed-point precision;
* economically favorable rounding;
* decimal mismatch;
* liquidation edge cases;
* oracle manipulation;
* stale oracle;
* insolvency;
* denial-of-service;
* compute exhaustion;
* account contention;
* unsafe admin configuration;
* compromised upgrade authority;
* token-extension behavior changes;
* malicious external integration.

For every major threat identify:

* asset at risk;
* attacker;
* entry point;
* prerequisite;
* impact;
* mitigation;
* test strategy;
* residual risk.

⸻

FORMAL INVARIANT CATALOGUE

Create an explicit invariant catalogue.

Group invariants under:

Authorization

Token Custody

Accounting

Solvency

Borrowing

Repayment

Oracle

Liquidation

State Lifecycle

Administrative Safety

Upgrade/Governance

Runtime/Resource Safety

Each invariant should eventually map to:

* implementation;
* unit/integration/property/adversarial test;
* documentation;
* phase.

Example style:

INVARIANT A-01:
Only the protocol-authorized PDA may transfer assets from protocol-controlled collateral vaults.

Do not merely create vague statements such as “funds must be secure.”

⸻

TEST ARCHITECTURE

Design the complete future test pyramid.

Include where appropriate:

* pure Rust unit tests;
* mathematical property tests;
* account-validation tests;
* program integration tests;
* LiteSVM;
* local validator / Surfpool;
* adversarial tests;
* fuzzing;
* exploit-regression tests;
* compute benchmarks;
* migration/upgrade tests.

Identify which tool should test which class of behavior.

Avoid redundant test stacks with no reason.

⸻

PERFORMANCE DESIGN

Before optimizing, identify likely performance constraints.

Analyze:

* writable account contention;
* global state;
* PDA derivation;
* CPI count;
* serialization;
* account size;
* realloc;
* token program calls;
* compute units;
* transaction account count;
* instruction composition.

Plan a benchmark strategy.

Later optimization must show:

BEFORE → measurement
CHANGE
AFTER → measurement

Never claim performance improvement without data.

⸻

COMPOSABILITY

Plan external integration only when product-relevant.

Possible future integrations:

* current Pyth;
* Jupiter for liquidation collateral routing;
* selected Solana liquidity/DEX infrastructure.

For every external protocol define trust assumptions and integration boundaries.

Avoid broad shallow integrations.

⸻

GOVERNANCE / UPGRADES

Design a realistic progression:

local development
→ single upgrade authority
→ protected/multisig authority
→ optional immutable/further governance model.

Document:

* upgrade authority risk;
* emergency controls;
* parameter-update permissions;
* pausing philosophy;
* limits on admin powers;
* migration strategy.

A security-conscious design should not rely on “admin can fix anything.”

⸻

REQUIRED TOPIC COVERAGE

Across Aegis production architecture + scoped labs + tests + ADRs, aim to create credible GitHub evidence for as many of these as possible:

* Rust ownership and borrowing;
* Rust traits/enums/errors/generics;
* Solana runtime/account model;
* Sealevel parallel execution;
* signer/writable semantics;
* program ownership;
* allocation/rent/account lifecycle;
* PDAs;
* canonical bumps;
* CPI;
* invoke_signed;
* Anchor;
* native Solana Rust;
* Pinocchio awareness/experiment;
* SPL Token;
* ATAs;
* Token-2022;
* vaults;
* authorities;
* DeFi fixed-point arithmetic;
* collateralized lending;
* interest/index accounting;
* health factor;
* liquidation;
* oracle architecture;
* Pyth integration;
* events;
* cross-program composability;
* transaction construction concepts;
* compute budget awareness;
* CU optimization;
* account contention;
* security;
* property testing;
* fuzzing;
* exploit regression;
* SDK/client;
* frontend integration;
* upgrade/governance;
* architecture documentation;
* security documentation;
* benchmark evidence.

For each topic classify:

PRODUCTION
LAB
TEST
ADR/DOCUMENTATION
NOT COVERED

Every “NOT COVERED” topic must have a reason.

Do not force topics into production merely to turn cells green.

⸻

PHASE ROADMAP

Design a phase-by-phase implementation plan for Claude Sonnet or another implementation model.

A provisional structure is:

Phase 0 — planning/design only.

Phase 1 — repository/toolchain/Anchor/local testing foundation.

Phase 2 — account/PDA/config foundation and custody primitives.

Phase 3 — deposits/withdrawals.

Phase 4 — position/debt accounting + borrow/repay.

Phase 5 — oracle abstraction + local oracle + current Pyth adapter.

Phase 6 — health + liquidation.

Phase 7 — carefully selected Token-2022 support.

Phase 8 — external composability / liquidation routing.

Phase 9 — SDK/client/UI and complete user flows.

Phase 10 — adversarial/property/fuzz/security campaign.

Phase 11 — compute/contention optimization.

Phase 12 — upgrades/governance/migrations.

Phase 13 — integrated demo/security review/GitHub polish.

This is NOT mandatory.

Improve it if necessary.

For every phase specify:

* scope;
* explicit non-scope;
* expected files/components;
* technical concepts demonstrated;
* implementation dependencies;
* security work;
* tests;
* demo;
* documentation;
* acceptance criteria;
* evidence required;
* Git milestone/tag recommendation.

The implementation model must always STOP after completing one phase.

⸻

IMPLEMENTATION HANDOFF QUALITY

Your phase specifications should be good enough that future Sonnet sessions can receive prompts such as:

“Implement Aegis Phase 3 exactly according to the frozen Phase 0 specification and current repository state.”

They should not need to redesign:

* economics;
* account authority;
* invariants;
* data ownership;
* testing philosophy;
* repository structure.

Where flexibility is intentionally allowed, state that explicitly.

⸻

AGENTS.md

Create/specify the exact intended AGENTS.md.

It is the tool/model-independent engineering constitution of the repository.

It must encode:

* project mission;
* product/non-goals;
* engineering quality bar;
* architecture authority;
* phase gating;
* implementation boundaries;
* security-first rules;
* invariant preservation;
* testing requirements;
* zero-cost demo requirement;
* dependency policy;
* current-version verification requirement;
* documentation requirements;
* ADR requirements;
* no fake completion;
* no silent scope deletion;
* no weakening tests merely to make them pass;
* no moving to next phase without explicit instruction;
* Git hygiene;
* secrets policy;
* benchmark evidence rules.

Avoid highly implementation-specific details that will immediately become stale.

⸻

CLAUDE.md

Also create/specify the exact intended root CLAUDE.md.

Its purpose is specifically to make future Claude sessions operate correctly in this repository.

AGENTS.md remains the authoritative general engineering policy.

CLAUDE.md should instruct Claude to:

1. Read AGENTS.md first.
2. Read docs/project-status.md.
3. Read the current phase specification.
4. Inspect relevant ADRs before changing architecture.
5. Treat frozen economic/invariant/security documents as authoritative unless explicitly asked to redesign them.
6. Never silently begin another phase.
7. Never claim tests were run when they were not.
8. Report exact validation commands/results.
9. Avoid large speculative refactors outside current scope.
10. Preserve zero-cost/local execution.
11. Never weaken security checks or tests simply to unblock implementation.
12. Record architectural deviations through ADRs.
13. Keep private learning material out of the repository.
14. Update project status and phase completion evidence after implementation.
15. Prefer minimal coherent changes over unrelated cleanup.
16. Stop and surface true contradictions rather than silently choosing a risky interpretation.
17. Never commit secrets.
18. Respect Git phase/milestone discipline.

Avoid duplicating the entire AGENTS.md.

CLAUDE.md should mostly define the Claude operating workflow and point to authoritative project docs.

⸻

DOCUMENTATION ARCHITECTURE

Plan the eventual public repository documentation.

At minimum:

README.md
AGENTS.md
CLAUDE.md

docs/
product.md
architecture.md
account-model.md
economic-model.md
invariants.md
threat-model.md
oracle-design.md
token-compatibility.md
testing-strategy.md
performance-strategy.md
zero-cost-demo.md
coverage-matrix.md
project-status.md
phase-roadmap.md

adr/
security/
benchmarks/
phases/

Use source-controlled diagrams, preferably Mermaid where suitable.

Keep private educational notes outside the repository.

⸻

PROJECT STATUS SYSTEM

Design docs/project-status.md so future models can immediately determine:

* current phase;
* phase state;
* what has been implemented;
* what has been validated;
* what has been demonstrated;
* what remains;
* latest milestone/commit;
* known issues;
* deferred work;
* current architectural decisions.

Do not allow “implemented” to mean “verified.”

Track them separately.

Example:

IMPLEMENTED
TESTED
DEMOED
DOCUMENTED
COMMITTED

⸻

ADRs

Identify and draft initial Architecture Decision Records for consequential decisions.

Likely candidates:

* Anchor as production framework;
* scoped native/Pinocchio coverage;
* protocol economic model;
* account/PDA architecture;
* minimizing global writable state;
* oracle abstraction;
* SPL/Token-2022 policy;
* fixed-point representation;
* local zero-cost architecture;
* upgrade-authority strategy.

Use ADRs only for meaningful decisions.

⸻

PHASE 0 OUTPUT

This entire session is Phase 0.

Produce planning artifacts only.

Required deliverables:

1. Product thesis.
2. Product critique.
3. Non-goals.
4. Personas/use cases.
5. Functional requirements.
6. Non-functional requirements.
7. Current ecosystem/tooling research.
8. Economic model.
9. Formula specification.
10. Economic assumptions.
11. Account/PDA model.
12. Instruction catalogue.
13. Token custody model.
14. Token-2022 policy.
15. Oracle architecture.
16. Trust-boundary model.
17. Threat model.
18. Formal invariant catalogue.
19. Testing architecture.
20. Performance/compute strategy.
21. Composability strategy.
22. Governance/upgrade model.
23. Zero-cost local architecture.
24. Topic coverage matrix.
25. Explicit gap analysis.
26. Final phase roadmap.
27. Phase acceptance criteria.
28. Repository architecture.
29. Documentation architecture.
30. AGENTS.md.
31. CLAUDE.md.
32. Project-status format.
33. Initial ADRs.
34. Implementation handoff instructions for future Sonnet sessions.

If operating inside an initialized repository, you may create/update documentation and planning files only.

Do NOT create implementation code.

Do NOT install application dependencies unless necessary solely to verify current toolchain information.

Do NOT implement Phase 1.

⸻

FINAL SELF-AUDIT

Before declaring Phase 0 complete, independently attack your proposal.

Ask:

* Is this actually a coherent lending protocol?
* Is any feature included solely for resume coverage?
* Can the account model parallelize?
* Is shared writable state minimized?
* Are authorities unambiguous?
* Could user-provided accounts redirect assets?
* Could the wrong token program be accepted?
* Could Token-2022 semantics invalidate accounting assumptions?
* Could vault balances diverge from internal accounting?
* Could rounding be exploited?
* What happens when oracle data is unavailable?
* What happens during extreme volatility?
* How does bad debt arise?
* How does liquidation fail?
* Which admin action could cause catastrophic damage?
* Which assumptions would be unacceptable for deployment with real money?
* Are tests capable of falsifying important invariants?
* Is every important portfolio claim backed by future observable repository evidence?
* Could a Sonnet implementation model execute the phases without inventing architecture?
* Have unnecessary technologies been rejected explicitly?

Fix weaknesses discovered by this review before finalizing.

⸻

COMPLETION RESPONSE

When planning is complete, output:

AEGIS PHASE 0 PLANNING COMPLETE

PRODUCT

Final product thesis.

MAJOR DESIGN DECISIONS

Most consequential decisions.

ECONOMIC MODEL

Summary.

ACCOUNT ARCHITECTURE

Summary.

SECURITY

Highest-risk areas and invariant structure.

TOOLING

Current choices and notable ecosystem findings.

COVERAGE

Expected breadth and explicit gaps.

PHASE ROADMAP

Final phases.

REPOSITORY RULES

AGENTS.md + CLAUDE.md status.

ARTIFACTS

Planning/docs created or proposed.

IMPLEMENTATION READINESS

State whether Phase 1 is sufficiently specified for a separate implementation model.

OPEN QUESTIONS

Only genuinely unresolved questions.

NEXT

Write:

“Next action: hand Phase 1 to the implementation model. Phase 1 has NOT been started.”

Then STOP.

Do not implement Phase 1.