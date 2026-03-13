# Work Routing — Azurite TS→Rust Port

## Domain Routing

| Domain / Signal | Primary Agent | Secondary | Notes |
|----------------|---------------|-----------|-------|
| Architecture, porting strategy, scope decisions | Gandalf | — | Final say on structure |
| TypeScript source analysis, TS comprehension | Faramir | — | Reads and explains TS code |
| Rust implementation, translation | Aragorn | Faramir (TS context) | Writes the Rust port |
| Azure Storage REST API, protocol fidelity | Samwise | — | Ensures API correctness |
| Testing, QA, parity validation | Boromir | — | Tests and quality gates |
| Porting database entries | Faramir + Aragorn | Gandalf (decisions) | Both contribute per-file notes |
| Code review (TS fidelity) | Faramir | Gandalf | Checks Rust matches TS |
| Code review (Rust correctness) | Aragorn | Gandalf | Checks Rust compiles and works |
| Code review (API parity) | Samwise | Boromir | Checks REST behavior matches |
| Session logging | Scribe | — | Automatic — never needs routing |

## Porting Workflow

1. **Faramir** analyzes TS source file → documents structure, dependencies, patterns
2. **Gandalf** reviews analysis → decides porting approach, records in porting-db
3. **Aragorn** translates to Rust → follows porting-db decisions
4. **Samwise** reviews API-facing code → ensures Azure Storage REST fidelity
5. **Boromir** validates → tests parity between TS and Rust behavior
6. **Faramir** reviews final Rust → confirms TS fidelity

## Reviewer Gates

| Artifact | Reviewer | Gate |
|----------|----------|------|
| Porting strategy / approach | Gandalf | Must approve before implementation |
| Rust translation | Faramir (TS fidelity) + Aragorn (Rust correctness) | Both must approve |
| API-facing code | Samwise | Must approve REST behavior |
| Test coverage | Boromir | Must approve before merge |

## Rules

1. **Eager by default** — spawn all agents who could usefully start work, including anticipatory downstream work.
2. **Scribe always runs** after substantial work, always as `mode: "background"`. Never blocks.
3. **Quick facts → coordinator answers directly.**
4. **When two agents could handle it**, pick the one whose domain is the primary concern.
5. **"Team, ..." → fan-out.** Spawn all relevant agents in parallel as `mode: "background"`.
6. **Anticipate downstream work.** If TS analysis is done, Aragorn can start translating while Boromir writes test scaffolding.
