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

## 2026-03-13: Rust Workspace Scaffold Rooted in `rust/`
**By:** Aragorn (Rust Expert)
**What:** Scaffolded the Rust port as a Cargo workspace rooted at `rust/`, with 5 service/library crates under `rust/crates/` and porting database under `rust/porting-db/`. Workspace members: azurite, azurite-common, azurite-blob, azurite-queue, azurite-table. Shared dependency versions centralized in `[workspace.dependencies]` at workspace root. Workspace compiles with `cargo check` from `rust/` directory. Phase 0 scaffolding complete.
**Why:** Keeps every Rust artifact under single root, matches Gandalf's planned crate layout, facilitates future TS→Rust propagation. Service crates expose both lib.rs and service binary in main.rs to stay structurally parallel to TypeScript entry points.

## 2026-03-13: Move porting-db/ to rust/porting-db/
**By:** Gandalf (Lead Architect)
**What:** Moved porting database from repository root to `rust/porting-db/`. All path references in STRATEGY.md and PORTING-ORDER.md updated (`porting-db/` → `rust/porting-db/`). Template examples in §16 (Record Format) now reference correct paths. New structure: rust/porting-db/ contains STRATEGY.md, PORTING-ORDER.md, README.md, and per-file YAML records under src/ mirroring TS structure.
**Why:** All Rust artifacts (including documentation and porting records) live under single `rust/` root except `.squad/` team metadata. Improves directory hygiene, clarifies change propagation boundaries, makes porting database first-class artifact of Rust port rather than separate root-level artifact. When Aragorn implements Rust module, he can immediately reference strategy docs and per-file records without crossing TypeScript/Rust boundary.

## 2026-03-13: Phase 1 Analysis Preserves Naming and Model Inconsistencies
**By:** Faramir (TypeScript Expert)
**What:** Decided to preserve TS source naming and model inconsistencies in Rust porting plan rather than normalizing them. Keep `contextID`/`contextId` differences documented. Treat `src/common/persistence/IExtentMetadata.ts` and `src/common/persistence/IExtentMetadataStore.ts` as distinct contracts with separate extent models (`persistencyId`/`LastModifyInMS` vs `locationId`/`lastModifiedInMS`). Do not collapse them without explicit compatibility layer.
**Why:** These differences are real TypeScript source behavior, not noise. Normalizing in port would make future TS change propagation harder and could hide compatibility-sensitive behavior like Loki metadata field bridging. Preserves exact TS semantics for accurate long-term propagation.
**Key Concerns Flagged:** (1) `IOperationQueue.operate<T>()` is generic and not object-safe as trait object in Rust — may need concrete implementation or non-object-safe trait pattern. (2) `IEnvironment` aggregates three service traits with overlapping method names, may push Rust port toward flattened config type for ergonomics while still preserving TS semantics. (3) `IServerFactory` abstraction narrower than current TS implementations — translation should preserve abstraction without assuming every factory directly implements it.

## Governance

- All porting decisions must be recorded in `rust/porting-db/`
- Architecture decisions require Gandalf's approval
- API-facing changes require Samwise's approval
- TS fidelity is reviewed by Faramir
