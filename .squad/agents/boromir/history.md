# Boromir — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### Workspace Ready and Phase 1 Analysis Complete (2026-03-13)
**Aragorn Status:** Rust workspace scaffold complete and compiles. Five-crate structure ready.

**Faramir Status:** Phase 1 TS analysis complete. All 15 common interface files analyzed with critical fidelity concerns:
- `IOperationQueue.operate<T>()` generics may require special Rust handling (trait object safety)
- `IExtentMetadata` vs `IExtentMetadataStore` are intentionally distinct contracts — preserve separation in Rust
- `contextID`/`contextId` naming inconsistencies must be preserved
- `IEnvironment` and `IServerFactory` abstractions need careful translation

**Next Phase:** Test framework can now build upon completed workspace and documented TS patterns from Phase 1 analysis. Reference STRATEGY.md §9 (Testing and Type Coverage) and §14 (Rust equivalents table) for type coverage mapping as Rust implementations proceed.

### Initial Rust Parity Harness Landed (2026-03-13)
- Added shared Rust test dependencies (`assert_matches`, `pretty_assertions`, `tokio-test`) at the workspace level and enabled per-crate dev-dependencies for `azurite-common`, `azurite-blob`, `azurite-queue`, and `azurite-table`.
- Created executable Phase 1 common parity tests in `rust/crates/azurite-common/tests/common/` for `OAuthLevel`, `LogLevels`, `Logger` forwarding, and `IDataStore`/`ICleaner`/`IAccountDataStore` contract behavior.
- Established ignored parity scaffolds mirroring the TS suite layout in `azurite-blob/tests/blob/`, `azurite-queue/tests/queue/`, and `azurite-table/tests/table/` so future ported modules can unignore tests in place.
- Documented the parity workflow in `rust/porting-db/TEST-STRATEGY.md` and verified `cargo test --workspace` passes from `rust/` with active and ignored parity suites compiling cleanly.

### Phase 1 Interfaces Translated; Tests Ready (2026-03-13)
Aragorn has completed Phase 1 Rust translation (15 common interfaces). Faramir provided Phase 2 fidelity risks (ZERO_EXTENT_ID circular dep, LastModifyInMS casing, class/file name asymmetries). Test framework ready: 9 active tests passing, 9 placeholders for Phase 2 modules ready to be unignored.

**Key decisions merged to `decisions.md`:**
1. Phase 1 IEnvironment flattened to local trait (crate-graph safe)
2. Per-crate test trees with ignored placeholders for future translations

Workspace status: ✅ `cargo check` passes, ✅ `cargo test` passes (9 active + 9 ignored), ✅ porting-db updated.

