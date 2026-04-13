# Intake: Playwright Testing & PWA Capability Assessment

## Context
User wants to use Playwright to test PWA, and evaluate whether PWA can properly manage OpenCode servers.

## Open Questions
- **Q2: "Management" Definition** [frozen]
  - Conclusion: Core scenario is Session lifecycle management (switch/fork/new session).
- **Q3: Test Environment** [frozen]
  - Conclusion: Use local real `relay-server` + `hostd` processes for E2E testing.
- **Q4: Capability Assessment** [frozen]
  - Conclusion: Backend capability ready (Server Session API + hostd `--session` resume), but PWA lacks Session operation UI.
  - Decision: **Option A** - First implement PWA Session UI/API (switch/fork/new), then write Playwright tests.

## Resolved Questions
- **Q1: Test Scope** [frozen]
  - Conclusion: Focus on E2E "server management operation flow", not just UI regression.
- **Q2: Management Definition** [frozen]
  - Conclusion: Session lifecycle management (switch/fork/new), aligns with H1 in REQUIREMENTS.md.
- **Q3: Test Environment** [frozen]
  - Conclusion: Use local real `relay-server` + `hostd`.

## Frozen Decisions
- Need to implement PWA Session operations before Playwright tests.

## Draft Scope
- Implement PWA Session switcher UI.
- Implement Fork Session feature.
- Implement New Session + bind opencode_session_id.
- Playwright E2E tests covering above operations.

## Draft Verify
- Run `cd web && bun playwright test`.

## Ready for ws-plan: yes