# Aragorn — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### Porting Strategy Available (2026-03-13)
Gandalf has completed comprehensive porting strategy analysis. Review before starting implementation:
- **Read first:** `rust/porting-db/STRATEGY.md` (1100 lines) — complete strategy with all architectural decisions
- **Use as queue:** `rust/porting-db/PORTING-ORDER.md` (424 lines) — 17-phase implementation schedule in order
- **Reference:** Follow type mappings, async patterns, and module organization from STRATEGY.md
- **Record keeping:** Create per-file YAML records in `rust/porting-db/src/` per STRATEGY.md §16 format

### Rust Workspace Rooted Under `rust/` (2026-03-13)
The Rust port is now scaffolded as a Cargo workspace rooted at `rust/Cargo.toml` with five members:
- `rust/crates/azurite/Cargo.toml` — combined binary crate mirroring `src/azurite.ts`
- `rust/crates/azurite-common/Cargo.toml` — shared library crate for `src/common/`
- `rust/crates/azurite-blob/Cargo.toml` — blob service library + binary scaffold
- `rust/crates/azurite-queue/Cargo.toml` — queue service library + binary scaffold
- `rust/crates/azurite-table/Cargo.toml` — table service library + binary scaffold
- Workspace dependencies are centralized in `rust/Cargo.toml` and Phase 0 tasks 0.1-0.6 are marked complete in `rust/porting-db/PORTING-ORDER.md`
- Porting docs now live in `rust/porting-db/{STRATEGY.md,PORTING-ORDER.md,README.md}` and per-file YAML records belong under `rust/porting-db/src/`
- `cargo check` succeeds from the `rust/` directory against the scaffolded workspace

### Phase 1 Analysis Complete: Key TS Patterns and Fidelity Concerns (2026-03-13)
Faramir has analyzed all 15 Phase 1 files (common interfaces and shared types). **Critical implementation notes:**
- **Trait object safety issue:** `IOperationQueue.operate<T>()` is generic and **not object-safe as a trait object in Rust**. Will likely need a concrete implementation or non-object-safe trait pattern. Do not force it into a trait object boundary without careful consideration.
- **Model distinction (critical for propagation):** TS source has two extent metadata contracts:
  - `IExtentMetadata` with fields `persistencyId`, `LastModifyInMS` (capital M)
  - `IExtentMetadataStore` with fields `locationId`, `lastModifiedInMS` (lowercase m)
  - These are intentionally different. Keep separate in Rust with explicit compatibility layer if bridging is needed. Do not collapse them.
- **Naming inconsistency (preserve exactly):** `contextID` and `contextId` coexist in TS source. Preserve this inconsistency in Rust to maintain fidelity.
- **Interface aggregation complexity:** `IEnvironment` aggregates three service traits with overlapping method names. May push toward flattened config type in Rust for ergonomics, but preserve TS semantics.
- **Factory abstraction:** `IServerFactory` is narrower than concrete factory implementations. Preserve abstraction without assuming every TS factory directly implements it.
- **Generic boundaries:** Watch for boxed stream/iterator boundaries when translating async lifecycles.
