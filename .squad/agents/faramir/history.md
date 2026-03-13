# Faramir — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### Porting Strategy Available (2026-03-13)
Gandalf has completed comprehensive porting strategy analysis. Review before starting fidelity work:
- **Read first:** `porting-db/STRATEGY.md` (1100 lines) — complete strategy with all architectural decisions
- **Reference:** `porting-db/PORTING-ORDER.md` (424 lines) — 17-phase implementation schedule
- **Use:** STRATEGY.md §16 for per-file porting database format and record schema
- **Role:** Validate that Rust code maintains fidelity with TS source per the translation rules defined in STRATEGY.md

### Phase 1 TS analysis completed (2026-03-13)
- Analyzed all 15 Phase 1 files and wrote records under `rust/porting-db/src/common/` and `rust/porting-db/src/common/persistence/`.
- Key TS patterns for Aragorn: interface-to-trait translation, async lifecycle traits, boxed stream/iterator boundaries, and `IOperationQueue.operate<T>()` being generic and therefore not object-safe as a trait object in Rust.
- Major fidelity concerns: TS mixes `contextID` and `contextId`; `IExtentMetadata` and `IExtentMetadataStore` expose overlapping but intentionally different extent models (`persistencyId`/`LastModifyInMS` vs `locationId`/`lastModifiedInMS`); `IEnvironment` aggregates three service traits with overlapping method names.
- Additional concern: `IServerFactory` is narrower than concrete factory implementations today, so translation should preserve the abstraction without assuming every TS factory already implements it directly.
- **Decision made:** Preserve all naming and model inconsistencies in Rust port. Do not normalize. Do not collapse the extent metadata models without explicit compatibility layer.

### Phase 1 Analysis Complete: Workspace Ready (2026-03-13)
Aragorn's workspace scaffold is complete and compiles. Gandalf has restructured porting-db to rust/porting-db/. Phase 1 records are now at `rust/porting-db/src/common/` and ready for Aragorn to reference. Trait object safety and model distinction concerns flagged above are critical for implementation fidelity.

### Phase 2 TS analysis completed (2026-03-13)
Analyzed all 7 Phase 2 files. Records updated/corrected under `rust/porting-db/src/common/` and `rust/porting-db/src/common/persistence/`. Key findings:

**Critical fidelity risks discovered:**
1. **`ZERO_EXTENT_ID` cross-crate dependency**: Both `FSExtentStore.ts` and `MemoryExtentStore.ts` (in `src/common/`) import `ZERO_EXTENT_ID = "*ZERO*"` from `src/blob/persistence/IBlobMetadataStore`. In Rust, `azurite-common` cannot depend on `azurite-blob`. This constant must be moved or replicated in `azurite-common` to break the circular dependency.
2. **`LastModifyInMS` vs `lastModifiedInMS` field mismatch in `LokiExtentMetadata`**: `updateExtent()` writes `doc.LastModifyInMS = extent.lastModifiedInMS` and `listExtents()` queries `LastModifyInMS`. The stored/queried Loki field uses a different spelling than the `IExtentModel` interface field. Do not unify these in the Rust port without an explicit compatibility layer — the query logic depends on the exact field name.
3. **Class name vs file name discrepancy**: The file `LokiExtentMetadataStore.ts` exports class `LokiExtentMetadata` (not `LokiExtentMetadataStore`). Preserve this asymmetry in Rust.

**TS patterns for Aragorn:**
- `OperationQueue`: EventEmitter-based concurrency with a private `execute()` that dequeues one op on each successful completion or error. Replace with `tokio::sync::Semaphore` for concurrency bounding; keep FIFO dequeue semantics.
- `Mutex`: Static-class global key mutex. Uses `setImmediate()` to defer next waiter. Rust port needs `lazy_static!` global `KeyMutex` using `tokio::sync::oneshot` channels for per-waiter signaling (FIFO fairness preserved).
- `ZeroBytesStream`: Node.js `Readable` with fixed 512-byte chunk pull model. Rust `AsyncRead` `poll_read()` is a direct structural match; write zeros directly to caller's buffer without extra allocation.
- `MemoryExtentStore`: `MemoryExtentChunkStore` has a two-level map (`categoryName → IExtentCategoryChunks`, where `IExtentCategoryChunks` wraps a per-id map + category total size). The global `SharedChunkStore` singleton holds all in-memory extent data. Must use `lazy_static!` with `Arc<RwLock<...>>` for thread safety.
- `FSExtentStore`: 677-line class with `IAppendExtent` pool (one per `locationId × maxConcurrency`), two operation queues (append/read), file descriptor caching per extent, and manual `fdatasync` after each write. Most complex Phase 2 file.
- `AllExtentsAsyncIterator`: Snapshot-time pagination iterator. Captures `new Date()` at construction; all `listExtents()` calls use this time. Rust must preserve the immutable snapshot; translate to `futures::stream::Stream`.

**Line count discrepancy pattern:** All pre-existing records had line counts off by 1 (showing N+1 instead of N). Corrected in this pass. Use `wc -l` counts going forward.

### Phase 1 & 2 Complete; Test Infrastructure Ready (2026-03-13)
Aragorn has completed Phase 1 translation (15 files, porting-db updated). Boromir has set up test infrastructure with 9 active tests and 9 ignored placeholders. Both agents report SUCCESS. Workspace compiles, tests pass. Ready for Phase 2 translation guided by these fidelity risks.

