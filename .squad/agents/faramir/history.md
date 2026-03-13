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

### Phase 3 TS analysis completed (2026-03-13)
- Analyzed all 5 Phase 3 authentication files and wrote records under `rust/porting-db/src/common/authentication/`.
- **Critical fidelity risks discovered:**
  1. Canonical serializer order is contract-sensitive and differs by helper: permissions serialize as `rwdxlacuptfiy`, services as `btqf`, and resource types as `sco`. `AccountSASServices.toString()` is intentionally not enum declaration order.
  2. `AccountSASPermission.Any` and `AccountSASResourceType.Any` are validation-only sentinels for later blob batch authorization. They are not accepted by the Phase 3 parser/serializer helpers and must stay outside canonical account-SAS string generation.
  3. `IAccountSASSignatureValues` dispatches on `version >= "2020-12-06"` using plain string comparison and types `ipRange` as external `SasIPRange | string` even though serialization goes through local `ipRangeToString()`.
- **TS patterns for Aragorn:**
  - The three account-SAS helper classes are mutable boolean-flag objects with static `parse()` plus canonical `toString()` methods; prefer explicit Rust structs over compressed bitflags for change propagation.
  - `AccountSASServices` and `AccountSASResourceTypes` reuse the duplicate-error message text `Duplicated permission character: ${c}` even outside permission parsing. Preserve that quirk if exact error text matters.
  - `generateAccountSASSignature*()` builds newline-joined string-to-sign payloads with a required trailing empty field; the `2020-12-06` branch inserts `encryptionScope`, and both branches use `truncatedISO8061Date(..., false)` so timestamps are second-precision only.

### Cross-Agent Status (2026-03-13 → 21:30)
- **Aragorn:** Phase 2 persistence translation complete. 7 modules ported; `cargo check` and tests pass. Awaiting Phase 3 analysis output (now ready). Phase 3 contains 3 critical fidelity constraints documented above.
- **Boromir:** Phase 1 parity test coverage now at 26 active tests, 4 ignored placeholders. Workspace clean. Ready to unignore Phase 3 test placeholders incrementally.
- **Samwise:** All systems operational. Three new decisions documented (D-008, D-009, D-010) ready for review before Phase 3 implementation starts.

### Phase 4 TS analysis completed (2026-03-13)
- Analyzed all 11 Phase 4 files and wrote records under `rust/porting-db/src/common/` plus `rust/porting-db/src/common/utils/`.
- **Critical fidelity risks discovered:**
  1. `Telemetry.ts` has two compatibility-sensitive quirks that are easy to “fix” accidentally: `GetRequestUri()` uses `hostname in knownHosts` (array-index check, so local-host redaction never triggers), and `GetInstanceID()` persists/read the misspelled JSON key `instaceID`. Both should stay explicit in the Rust plan unless the team approves divergence.
  2. `Environment.ts` registers `--disableProductStyleUrl` twice and performs several validations lazily inside getters (`inMemoryPersistence()`, `debug()`), not at parse time. Rust CLI parsing should not silently normalize those behaviors without review.
  3. `ServerBase.ts` and `Logger.ts` rely on process-global mutable lifecycle/state patterns: strict server status transitions with asymmetric failure handling (`afterStart()` can leave status `Running`, `afterClose()` can leave status `Closing`), plus a singleton logger whose strategy is swapped at runtime.
- **TS patterns for Aragorn:**
  - `WinstonLoggerStrategy` is a thin runtime-selected sink adapter; map it to a `tracing`/writer-backed strategy object, but preserve the literal output template and the odd default `contextID = "\t"`.
  - `ConfigurationBase` does not parse CLI flags itself; it is a helper layer over already-parsed values, with PEM-vs-PFX precedence, permissive OAuth parsing (`basic` only), and `setExtentMemoryLimit()` mixing validation, logging, and global `SharedChunkStore` mutation.
  - `AccountDataStore` polls `AZURITE_ACCOUNTS` every 60 seconds via `setInterval(...).unref()`, silently falling back to the default emulator account on parse failures. The parser grammar is `account:key1[:key2];...` with permissive trailing semicolons.
  - `Telemetry.ts` is an all-static singleton with separate event/request clients, hard-coded Application Insights connection string, generated-context `instanceof` dispatch, and request properties assembled from headers/query/body length rather than a service-neutral abstraction.


### Phase 4 Analysis Complete; Decisions Ready (2026-03-13 → 22:10)
- **Status:** Phase 4 utilities/config analysis COMPLETE.
- **Scope:** 11 modules — Constants, Utils, BufferStream, Logger, NoLoggerStrategy, WinstonLoggerStrategy, ConfigurationBase, ServerBase, AccountDataStore, Environment, Telemetry.
- **Fidelity hazards documented:** Telemetry `instaceID` misspelling, knownHosts redaction never triggers, WinstonLoggerStrategy tab default, Environment CLI arg duplication.
- **Decision:** D-002 (preserve observable Phase 4 quirks until explicit approval) recorded in `.squad/decisions/decisions.md`.
- **Next:** Awaiting Gandalf/Samwise decision on quirk handling before Phase 4 implementation proceeds.

### Cross-Agent Status (2026-03-13 → 22:10)
- **Aragorn:** Phase 3 translation COMPLETE. 5 auth files ported; tests passing. Account-SAS signing ready for Phase 4 utils consolidation.
- **Boromir:** Phase 2 parity tests ACTIVATED. 7 modules all passing. Test suite clean.
- **Samwise:** Phase 3 translated, Phase 4 analyzed. 44 tests passing, 38 porting-db records, 108 Rust source files.

### Phase 5 TS analysis completed (2026-03-13)
- Analyzed all 34 blob generated-framework files and wrote records under `rust/porting-db/src/blob/generated/`.
- **Critical fidelity risks discovered:**
  1. The generated middleware chain is a strict six-stage pipeline — `dispatch -> deserializer -> handler -> serializer -> error -> end` — and `ExpressMiddlewareFactory` rebuilds adapters/context wrappers at each stage while sharing state through `res.locals[contextPath]`.
  2. `operation.ts`, `specifications.ts`, and `handlers/handlerMappers.ts` are coupled by the zero-based numeric `Operation` enum. Reordering enum members or “simplifying” the lookup tables would silently reroute requests.
  3. `utils/serializer.ts` and `Context.ts` are the main `any` choke points: handler parameters, handler responses, XML/JSON intermediate bodies, and dynamic handler lookup all stay runtime-typed today.
  4. Request/response wrapper contracts carry observable quirks that must survive translation: `ExpressResponseAdapter.setHeader()` stringifies numbers/booleans, `error.middleware.ts` suppresses body/content-type only for HEAD failures, and stream serialization resolves on writable `close` while final `.end()` happens later in `end.middleware.ts`.
- **TS patterns for Aragorn:**
  - Generated models are split across `models.ts` (interfaces/enums/intersection response aliases), `mappers.ts` (123 composite mappers), `parameters.ts` (131 parameter descriptors), and `specifications.ts` (72 operation specs). Treat these four files as one change-propagation unit.
  - Handler dispatch is intentionally stringly/dynamic: `IHandlers` exposes six named handler families, `handlerMappers.ts` maps each operation to `{ handler, method, arguments }`, and `HandlerMiddlewareFactory` indexes that map with `(this.handlers as any)[handlerPath.handler]`.
  - XML handling depends on exact `xml2js` options (`explicitArray: false`, `explicitCharkey: false`, `explicitRoot: false`, `emptyTag: undefined`) plus manual sequence wrapping/unwrapping in `utils/serializer.ts`.
  - `dispatch.middleware.ts` does not short-circuit on first match; it scores every operation by required conditions met and prefers `context.dispatchPattern` over `req.getPath()` when present.

### Cross-Agent Status (2026-03-13 → 23:10 batch completion)
- **Phase 4 Translation:** Aragorn COMPLETE. All 11 Phase 4 files compile. Tests pass. Decision notes recorded. IEnvironment contract tightened to match TS surface (Option<String> for debug, Option<f64> for extentMemoryLimit).
- **Phase 5 Analysis:** COMPLETE. 34 blob-generated framework records seeded. 6-stage middleware pipeline order documented as architecture-critical. Operation/Specs coupling exposed. Serializer any-type usage noted.
- **Phase 3 Tests Refinement:** Boromir COMPLETE. 15 Phase 3 auth parity tests passing. IIPRange type asymmetry bug (D-010) fixed — explicit adapter now preserves structural compatibility between SasIPRange and IIPRange.
- **Overall Stats:** 59 tests passing, 72 porting-db records, 113 Rust source files.
- **User Directive — Continuous Pipeline:** Auto-launch Phase 6 immediately. No pause between batches. Work all night if necessary.
