# Squad Team

> Azurite — TypeScript to Rust port

## Coordinator

| Name | Role | Notes |
|------|------|-------|
| Squad | Coordinator | Routes work, enforces handoffs and reviewer gates. |

## Members

| Name | Role | Charter | Status |
|------|------|---------|--------|
| Gandalf | Lead / Architect | .squad/agents/gandalf/charter.md | 🏗️ Active |
| Faramir | TypeScript Expert | .squad/agents/faramir/charter.md | ⚛️ Active |
| Aragorn | Rust Expert | .squad/agents/aragorn/charter.md | 🔧 Active |
| Samwise | Azure Storage REST Expert | .squad/agents/samwise/charter.md | 🔒 Active |
| Boromir | QA Expert | .squad/agents/boromir/charter.md | 🧪 Active |
| Scribe | Session Logger | .squad/agents/scribe/charter.md | 📋 Active |
| Ralph | Work Monitor | — | 🔄 Monitor |

## Project Context

- **Project:** Azurite — Azure Storage Emulator
- **User:** Quetzal Bradley
- **Created:** 2026-03-13
- **Universe:** Lord of the Rings
- **Language:** TypeScript → Rust port
- **Stack:** Node.js/TypeScript (source), Rust (target)
- **Goal:** Faithful translation of Azurite from TypeScript to Rust, prioritizing fidelity with the TS source over idiomatic Rust. The port must be easy to update as the TS codebase evolves.

## Porting Philosophy

1. **Fidelity over idiom** — The Rust code should be a near-direct translation of the TypeScript. We favor keeping structure, naming, and logic flow as close to the TS as possible.
2. **No performance tricks** — Do not optimize for Rust performance. Keep it readable and traceable.
3. **Change propagation is king** — The #1 goal is that future TS changes can be easily ported to the Rust version. Every decision must serve this goal.
4. **Porting database** — `porting-db/` contains per-file records of all decisions, transformations, substitutions, and notes needed to propagate future changes.
5. **Rust code lives in `rust/`** — All Rust port code is in the `rust/` subdirectory.
