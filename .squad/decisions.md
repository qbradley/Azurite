# Squad Decisions

## 2026-03-13: Porting Philosophy Established
**By:** Quetzal Bradley (via Copilot)
**What:** The Rust port of Azurite must prioritize fidelity with the TypeScript source over idiomatic Rust. No performance optimization. The primary goal is easy propagation of future TS changes to the Rust port.
**Why:** User directive — foundational project constraint.

## 2026-03-13: Porting Database Required
**By:** Quetzal Bradley (via Copilot)
**What:** A `porting-db/` directory will maintain per-file records of all decisions, transformations, substitutions, and notes for every source file in the TypeScript codebase. This is essential for future change propagation.
**Why:** User directive — core infrastructure for the port.

## 2026-03-13: Rust Code Location
**By:** Quetzal Bradley (via Copilot)
**What:** All Rust port code lives in the `rust/` subdirectory at the repository root.
**Why:** User directive — project structure decision.

## 2026-03-13: Fidelity Over Idiom
**By:** Quetzal Bradley (via Copilot)
**What:** Favor keeping fidelity with the TypeScript source rather than idiomatic Rust. Do not prioritize performance or use Rust tricks. The code should be a near-direct translation.
**Why:** User directive — ensures future TS changes can be mechanically propagated to Rust.

## 2026-03-13: Porting Strategy Established
**By:** Gandalf (Lead Architect)
**What:** Comprehensive porting strategy for Azurite TypeScript → Rust defined in `porting-db/STRATEGY.md` (1100 lines) and `porting-db/PORTING-ORDER.md` (424 lines). Covers type mappings, async strategy (Tokio + axum), dependency mapping, module organization, API translation, middleware patterns, error handling, and per-file porting database format.
**Decisions:** 8 critical architectural decisions documented:
- D-004: Tokio as async runtime (required by axum)
- D-005: axum as Express replacement (Tower-based, async-native)
- D-006: No autorest re-generation (manual translation preserves 1:1 structure)
- D-007: Custom in-memory store (HashMap+RwLock, NOT LokiJS port)
- D-010: Flatten intersection types (TS `A & B & C` → single Rust struct)
- D-013: VS Code extension out of scope (JS-only APIs)
- D-015: Blob first, queue second, table third (blob establishes patterns)
- D-016: Composition over inheritance (embed base struct as field)
**Why:** Required to guide implementation. All team members must review STRATEGY.md before starting work.
**Governance:** 17-phase porting schedule. Per-file YAML records in `porting-db/src/` mirror TS source tree.

## Governance

- All porting decisions must be recorded in `porting-db/`
- Architecture decisions require Gandalf's approval
- API-facing changes require Samwise's approval
- TS fidelity is reviewed by Faramir
