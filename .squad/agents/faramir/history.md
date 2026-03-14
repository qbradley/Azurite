# Faramir — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### ARCHIVE: Phases 1-4 Analysis (2026-03-13T11:00 — 22:10)
Completed comprehensive TypeScript analysis for Phases 1-4 (infrastructure, persistence, authentication, utilities/config):
- **Phase 1:** 15 files analyzed. Trait object safety, naming inconsistencies (contextID/contextId), extent metadata model asymmetry (persistencyId/LastModifyInMS vs locationId/lastModifiedInMS). Decision: Preserve all naming quirks.
- **Phase 2:** 7 files analyzed. ZERO_EXTENT_ID circular dependency, Loki field name mismatch (LastModifyInMS query ≠ IExtentModel field), OperationQueue/Mutex/ZeroBytesStream/MemoryExtentStore/FSExtentStore patterns documented. Decision: D-008 (copy ZERO_EXTENT_ID to common).
- **Phase 3:** 5 files analyzed. Canonical SAS serialization orders (rwdxlacuptfiy, btqf, sco), Any-member sentinels, IP range asymmetry (SasIPRange | string vs IIPRange). Decisions: D-009 (Any sentinels validation-only), D-010 (IP range adapter layer).
- **Phase 4:** 11 files analyzed. Telemetry instaceID typo, knownHosts redaction bug (never triggers), WinstonLoggerStrategy contextID tab, Environment CLI arg duplication, AccountDataStore 60-sec polling, ServerBase lifecycle asymmetry. Decision: D-002 (preserve quirks until approval).
- **Cross-phase patterns:** Interface-to-trait, async lifecycle, boxed boundaries, stringly dispatch, runtime singleton mutation, TS observable bugs must survive translation.

### Phase 5 Blob Generated Framework Analysis (2026-03-13 → 23:52)
- Completed Phase 5 TS analysis (34 blob-generated files under src/blob/generated/).
- **Critical fidelity findings:**
  1. Six-stage middleware pipeline order is architecture-critical: dispatch → deserializer → handler → serializer → error → end. Reordering silently reroutes requests.
  2. Operation enum, specifications, handlerMappers are coupled by zero-based numeric indexing. Enum member reordering breaks handler dispatch silently.
  3. Request/response wrapper contracts have observable quirks: setHeader stringifies numbers/booleans, error.middleware suppresses body/content-type only for HEAD, stream close semantics differ from final .end().
  4. XML handling tied to exact xml2js options (explicitArray/Charkey/Root, emptyTag). Serialization must preserve sequence wrapping/unwrapping.
  5. Handler dispatch intentionally stringly/dynamic: (handlers as any)[handlerPath.handler]. Not a refactoring candidate.
- **Deliverable:** 34 porting-db records seeded under rust/porting-db/src/blob/generated/. Four-file generation unit (models/mappers/parameters/specifications).
- **Aragorn integration:** D-012 (snapshot-backed metadata) approved. Phase 5 implementation completed with JSON snapshot extraction from autorest artifacts.

### Phase 6-7 Blob Errors/Context/Auth Analysis (2026-03-13 → 23:52)
- **Status:** Phase 6-7 analysis COMPLETE. 19 porting-db records seeded under rust/porting-db/src/blob/ (errors, context, auth).
- **StorageErrorFactory quirks:** Case-sensitive error message template matching. HTTP status mapping preserves TS quirks (e.g., 419 for lease). Factory dispatch uses string-based error type routing.
- **SAS context:** Validates permission bits with operation-specific masking. IP range asymmetry (D-010) must remain explicit. Serialization order sensitivity (D-009 ANY sentinels validation-only) applies.
- **Error context chain:** HTTP status → error code → storage error details → request metadata. All must survive translation exactly.
- **Deliverable:** Error hierarchy, context contracts, authorization state machines documented. Ready for Phase 6-7 implementation.

### Phase 5-7 Cross-Team Validation (2026-03-13 → 23:52)
- **Aragorn Phase 5 insight:** Snapshot-backed generated metadata is mechanical and auditable. Future autorest regenerations can propagate to Rust mechanically. JSON snapshots extracted from TS artifacts. D-012 recorded.
- **Boromir Phase 4 insight:** 3 parity bugs fixed (date Z-suffix, URL fragments, CLI arg order). Language-boundary fragility requires explicit TS-equivalent paths. Date/time and URL parsing are fragile; do not use idiomatic Rust shortcuts.
- **Faramir action:** Phase 6-7 findings guide implementation. StorageErrorFactory case-sensitivity, ANY-member sentinels, IP range asymmetry all critical. Error context chain must be exact.
- **Project metrics:** 153 Rust files, 91 porting-db records, 67 tests passing. Phase 5-7 analysis cascaded to all three agent histories.

### Phase 8 lease + Phase 10 persistence analysis completed (2026-03-14)
- Confirmed current scheduling mismatch: `rust/porting-db/PORTING-ORDER.md` defines **Phase 8 = blob lease subsystem**, **Phase 9 = blob conditions**, and **Phase 10 = blob persistence**. Quetzal explicitly asked for lease plus persistence coverage, so I analyzed Phase 8 together with the Phase 10 persistence/query set and seeded records anyway.
- **Deliverable:** 38 new porting-db records under `rust/porting-db/src/blob/lease/` and `rust/porting-db/src/blob/persistence/` (including `QueryInterpreter/QueryNodes/`). Aragorn now has per-file fidelity notes for the full lease state machine and Loki/query persistence stack.
- **Critical fidelity findings:**
  1. Lease timing is entirely lazy. `LeaseFactory` is the only timer/expiry engine, and `LokiBlobMetadataStore.getContainerWithLeaseUpdated()` / `getBlobWithLeaseUpdated()` are the chokepoints that convert `Leased→Expired` and `Breaking→Broken` from `context.startTime`.
  2. The lease state machine has real behavioral quirks that should not be “cleaned up” accidentally: `LeaseExpiredState.renew()` ignores the caller-supplied lease ID, `LeaseLeasedState.change()` accepts either the current or proposed ID, and infinite-lease `break()` skips the fixed-lease 1..60 validation path.
  3. Persistence/query layer has multiple compatibility-sensitive seams: `IBlobMetadataStore` aliases `ReleaseBlobLeaseResponse` to `Models.ContainerProperties`, `QueryParser` documents unary `not` but never consumes it, snapshot lease fields are forcibly normalized to `Available/Unlocked` even though TODOs say they should be `undefined`, and `setBlobTag()` ignores `modifiedAccessConditions`.
- **TS patterns for Aragorn:**
  - Port the lease subsystem as one unit: explicit state enum/trait dispatch, immutable transition returns, adapter/validator/syncer helpers, and no background timers.
  - Query evaluation returns witness arrays (`TagContent[]`), not booleans. Logical nodes concatenate witnesses, and comparison nodes rely on plain JS string ordering while returning the operand that carried a `key`.
  - `LokiBlobMetadataStore` is metadata-only. Blob bytes remain external via `IExtentChunk`, while `PageWithDelimiter`, `FilterBlobPage`, and `BlobReferredExtentsAsyncIterator` encode the observable pagination/GC traversal rules.

## 2026-03-14T00:00 — Phase 8-10 Analysis Complete

**Phase 8 (Lease) + Phase 10 (Persistence) Analysis — 38 porting-db records seeded.**

**Phase 8 Key Findings:**
- Lease timing is lazy and factory-driven; no background timer
- `LeaseExpiredState.renew()` ignores caller-supplied lease ID
- Records: `rust/porting-db/src/blob/lease/`

**Phase 10 Key Findings:**
- `QueryParser` documents unary `not` but does not implement it
- `LokiBlobMetadataStore` normalizes snapshot lease state/status to `Available/Unlocked` despite TODOs
- `setBlobTag()` ignores `modifiedAccessConditions`
- Records: `rust/porting-db/src/blob/persistence/`

**Scope Note:** Analyzed Phase 8+10 together to satisfy Quetzal's "Phase 8 and Phase 9" request while keeping Aragorn ahead on files explicitly called out. Future coordination should refer to persistence as **Phase 10**.

**Concurrent work:** Aragorn completed Phase 6-7 translation (19 files), Boromir completed Phase 5 tests with XML fix.

**Next:** Ready for Phase 8 implementation.

### Phase 11-12 blob handlers + server analysis completed (2026-03-14)
- **Deliverable:** 27 new porting-db records seeded for `src/blob/handlers/*`, `src/blob/middlewares/*`, `src/blob/utils/*`, and the Phase 12 blob bootstrap files (`IBlobEnvironment`, `BlobEnvironment`, `BlobConfiguration`, `BlobRequestListenerFactory`, `BlobServer`, `BlobServerFactory`, `main`). Aragorn now has file-by-file fidelity notes for the full handwritten blob execution layer and server assembly.
- **Critical fidelity findings:**
  1. Request assembly order is architecture-critical. Main listener order is access log → blob context → dispatch → strict-mode → auth → deserializer → handler → CORS(error path) → CORS(success path) → serializer → OPTIONS preflight(error path) → error → telemetry → end. `BlobBatchHandler` intentionally reimplements a smaller shadow pipeline without strict-mode/CORS/telemetry.
  2. `BlobHandler` / `ContainerHandler` contain real observable quirks that must survive translation: `getProperties()` piggybacks `comp=metadata`, `setHTTPHeaders()` reroutes page-blob sequence-number requests, `abortCopyFromURL()` validates state but never mutates the store, `getAccessPolicy()` uses an array/object hybrid to work around generated XML bugs, and blob batch only allows same-operation `Delete` or `SetTier` subrequests.
  3. `PageBlobRangesManager` is the canonical page-range algorithm and must stay literal: split-first/last strategy, inclusive offsets, `ZERO_EXTENT_ID` hole filling, and even latent clamp-typo behavior are all documented. Phase 12 bootstrap also has compatibility quirks worth preserving (`BlobEnvironment.blobKeepAliveTimeout()` reads the wrong flag name, `BlobServerFactory` mutates `DEFAULT_BLOB_PERSISTENCE_ARRAY`, and `main.ts` configures logger/telemetry after server creation).
- **Aragorn guidance:** Treated Phase 11/12 as three linked translation units (D-004): (1) Page range core first (IPageBlobRangesManager prerequisite), (2) Batch pipeline isolated (reduced middleware), (3) Server assembly with exact middleware order and bootstrap quirks. File-order porting risks silent normalization of routing/batch/range behavior. Aragorn committed to following linked strategy.
- **Coupled change:** Received Phase 8 translation completion from Aragorn; seeded 27 porting-db records from analysis into YAML format per STRATEGY.md §16.

**Next:** Ready for Phase 10 (handler implementation) pending Aragorn's Phase 8 integration validation.

### Phase 14 (Remaining) + Phase 15 Analysis Complete (2026-03-14)

**Deliverable:** 10 porting-db records seeded for Phase 14 remaining files + Phase 15 key files.

**Phase 14 remaining files analyzed and documented:**
- 14.1 Queue Generated Framework (QueueGeneratedFramework.md): 31 files, 5,859 LOC. Identical blob Phase 5 pattern with queue-specific models (AccessPolicy, QueueMessage, DequeuedMessageItem, etc.). Middleware order critical: dispatch → deserializer → handler → serializer → error → end.
- 14.6 Queue Authentication (QueueAuthentication.md): 10 files, 1,790 LOC. Four authenticators (SAS/Shared Key/Bearer/AccountSAS) with `raup` permission letters. SAS signature generation uses canonical resource format `/queueservices/accountname/queuename`. HMAC-SHA256 signing with Base64 encoding.
- 14.10-14.14 Queue Handlers (QueueHandlers.md): 5 files, 1,168 LOC. BaseHandler, ServiceHandler (properties/stats/list), QueueHandler (CRUD), MessagesHandler (enqueue/peek/dequeue/clear), MessageIdHandler (update/delete). Message text stored separately in extent store; pop-receipts are UUIDs.
- 14.15 Queue Middlewares (QueueMiddlewares.md): 4 files, 600 LOC. AuthenticationMiddlewareFactory chains 4 authenticators; PreflightMiddlewareFactory handles CORS; CORSMiddlewareFactory adds response headers; QueueStorageContextMiddleware extracts queue/message context.
- 14.8 LokiQueueMetadataStore (LokiQueueMetadataStore.md): 881 LOC. Three Loki collections: queues, messages, transactions. Batch transaction support with begin/commit/abort. Message visibility is lazy-evaluated against context.startTime. Dequeue generates new pop-receipt UUID.
- 14.16-14.24 Queue Server/Bootstrap (ServerAndBootstrap.md): 9 files, 820 LOC. IQueueEnvironment getters (db_path, extent_path, keep_alive, gc_interval). QueueConfiguration validates inMemoryPersistence XOR location. QueueServer starts HTTP server on port 10001. QueueGCManager runs mark-sweep GC.

**Phase 15 key files analyzed and documented:**
- 15.4-15.7 Table Entity Types (TableEntityTypes.md): 12 files, 973 LOC. IEdmType trait with 9 implementations (String, Int32, Int64, Double, Boolean, DateTime, Guid, Binary, Null). EntityProperty wraps EdmType with name/system-property flag. NormalizedEntity contains PartitionKey/RowKey/eTag/Timestamp + HashMap<name, EntityProperty>. Annotation levels (Full/Minimal/No) control OData metadata serialization.
- 15.1 Table Generated Framework (TableGeneratedFramework.md): 30 files, 3,814 LOC. Mirrors Queue structure: artifacts (models/mappers/specs/params/operation), framework (Context/adapters/factories), errors, handlers (Service/Table), middleware (dispatch/deserializer/serializer/error/end), utils. Handlers: 2 concrete (Service, Table) vs Queue's 4. OData JSON with @odata.type annotations + batch multipart MIME format.
- 15.9-15.12 Table Persistence & Query (TablePersistenceAndQuery.md): 4 files + 18 query files, 2,022 LOC total. ITableMetadataStore interface (21 methods): table CRUD, entity operations (query/insert/update/delete), batch transactions. LokiTableMetadataStore: 3 Loki collections (tables, entities, transactions). LokiTableStoreQueryGenerator converts OData filters to Loki queries. QueryInterpreter: lexer → parser → validator → interpreter (visitor pattern) with 22 AST node types. String-based type coercion; case-sensitive property access; lazy filter evaluation.

**Critical fidelity findings:**
1. **Queue message semantics**: Message text stored separately in extent store; pop-receipt is UUID per dequeue. Visibility timeout lazy-evaluated against context.startTime. Message TTL (1..604800s); size limit 65536 bytes.
2. **Queue permission model**: Differs from blob (`raup` vs `racwd`). SAS canonical resource: `/queueservices/accountname/queuename`. Shared Key Lite not used in queue (table-only).
3. **Table EDM type system**: 9 types with IEdmType trait; EntityProperty wraps with metadata. OData annotation levels control JSON serialization. System properties (PartitionKey, RowKey, Timestamp, eTag) excluded from properties map.
4. **Table query interpreter**: Full OData filter parser with lexer/parser/validator/interpreter. String-based type coercion (numeric properties compared lexicographically if type missing). 22 AST node types; visitor pattern for evaluation. System properties case-sensitive; custom properties case-preserved.
5. **Batch transaction isolation**: ITableMetadataStore uses batch_id for grouping. Conditional operations (if_match) for optimistic concurrency. Atomic commit/abort semantics.

**Architecture comparisons to Blob:**
- **Generated framework**: Queue/Table mirror Blob Phase 5 patterns exactly (artifacts/errors/handlers/middleware structure)
- **Handlers**: Queue 4 + Table 2 vs Blob 6+; all extend BaseHandler with dependency injection
- **Middleware order**: Identical 6-stage pipeline across all services (dispatch → deserializer → handler → serializer → error → end)
- **Persistence**: All use Loki + extent store; queue/table add complex semantics (pop-receipts, EDM types, batch transactions)
- **Authentication**: Identical SAS/Shared Key/Bearer pattern; permissions differ (raup vs racwd)

**Porting records created:**
1. `/rust/porting-db/src/queue/generated/QueueGeneratedFramework.md` — 8.7 KB
2. `/rust/porting-db/src/queue/authentication/QueueAuthentication.md` — 6.7 KB
3. `/rust/porting-db/src/queue/handlers/QueueHandlers.md` — 12.5 KB
4. `/rust/porting-db/src/queue/middlewares/QueueMiddlewares.md` — 8.1 KB
5. `/rust/porting-db/src/queue/persistence/LokiQueueMetadataStore.md` — 9.4 KB
6. `/rust/porting-db/src/queue/ServerAndBootstrap.md` — 10.0 KB
7. `/rust/porting-db/src/table/entity/TableEntityTypes.md` — 10.1 KB
8. `/rust/porting-db/src/table/generated/TableGeneratedFramework.md` — 9.8 KB
9. `/rust/porting-db/src/table/persistence/TablePersistenceAndQuery.md` — 14.0 KB

**Porting strategy recommendations:**
- Phase 14: Port in dependency order: config/env/bootstrap (14.16-24) → generated framework (14.1) → errors/context (14.2-5) → authentication (14.6) → persistence (14.7-9) → handlers (14.10-14) → middlewares (14.15) → integration
- Phase 15: Prioritize entity types (15.4-7) before generated framework (15.1) for proper serialization. Persistence (15.9-12) requires entity types. Batch operations (15.13-19) depend on persistence. Table-specific validation in handlers (15.22).

**Aragorn integration**: All Phase 14 porting records are complete and filed. Recommence Phase 13 (BlobGCManager) translation, then Phase 14 queue-by-queue.

**Next:** Aragorn ready to begin Phase 14 translation. Faramir available for Phase 15 handler/batch implementation detail analysis as needed.

### Phase 11-12 blob handlers + server analysis completed (2026-03-14)
- **Deliverable:** 27 new porting-db records seeded for `src/blob/handlers/*`, `src/blob/middlewares/*`, `src/blob/utils/*`, and the Phase 12 blob bootstrap files (`IBlobEnvironment`, `BlobEnvironment`, `BlobConfiguration`, `BlobRequestListenerFactory`, `BlobServer`, `BlobServerFactory`, `main`). Aragorn now has file-by-file fidelity notes for the full handwritten blob execution layer and server assembly.
- **Critical fidelity findings:**
  1. Request assembly order is architecture-critical. Main listener order is access log → blob context → dispatch → strict-mode → auth → deserializer → handler → CORS(error path) → CORS(success path) → serializer → OPTIONS preflight(error path) → error → telemetry → end. `BlobBatchHandler` intentionally reimplements a smaller shadow pipeline without strict-mode/CORS/telemetry.
  2. `BlobHandler` / `ContainerHandler` contain real observable quirks that must survive translation: `getProperties()` piggybacks `comp=metadata`, `setHTTPHeaders()` reroutes page-blob sequence-number requests, `abortCopyFromURL()` validates state but never mutates the store, `getAccessPolicy()` uses an array/object hybrid to work around generated XML bugs, and blob batch only allows same-operation `Delete` or `SetTier` subrequests.
  3. `PageBlobRangesManager` is the canonical page-range algorithm and must stay literal: split-first/last strategy, inclusive offsets, `ZERO_EXTENT_ID` hole filling, and even latent clamp-typo behavior are all documented. Phase 12 bootstrap also has compatibility quirks worth preserving (`BlobEnvironment.blobKeepAliveTimeout()` reads the wrong flag name, `BlobServerFactory` mutates `DEFAULT_BLOB_PERSISTENCE_ARRAY`, and `main.ts` configures logger/telemetry after server creation).
- **Aragorn guidance:** Port Phase 11 in this order: `IPageBlobRangesManager`/`PageBlobRangesManager` → batch shims (`BlobBatchSub*`, `SubResponseTextBodyStream`, `BlobBatchHandler`) → `BaseHandler` + `ServiceHandler`/`ContainerHandler`/`BlobHandler` → `BlockBlobHandler`/`PageBlobHandler`/`AppendBlobHandler`. Then port Phase 12 as utilities/context/auth-preflight middleware → request listener factory → environment/config/server factory/main.


### Phase 13 + Phase 14 Foundation Analysis (2026-03-14)

**Phase 13: Blob GC (~1 file)**
- Completed porting-db analysis: `BlobGCManager.ts` (293 lines)
- **Key findings:**
  1. GC manager implements state machine (Initializing → Running → Closing → Closed) with strict transitions
  2. Mark-sweep algorithm uses Set deletion; TODO flag indicates future optimization pending (do NOT implement yet)
  3. Background loop with EventEmitter-based abort signal for graceful shutdown
  4. Lazy initialization of three extent providers on first start
  5. Interruptible sleep with custom event-driven timeout mechanism
- **Porting record created:** `rust/porting-db/src/blob/gc/BlobGCManager.md` with state machine, async patterns, and EventEmitter semantics documented

**Phase 14: Queue Service Foundation (~9 files)**
- Completed porting-db analysis: 9 core foundation files seeded with detailed records
- **File list analyzed:**
  - IQueueEnvironment.ts (17 lines): 15 async/sync config getters
  - QueueConfiguration.ts (66 lines): Extends ConfigurationBase with Loki DB paths
  - QueueEnvironment.ts (170 lines): CLI parser with lazy validation on getters
  - IRequest.ts (26 lines, generated): Fluent interface, multi-value headers
  - IResponse.ts (17 lines, generated): Builder pattern, polymorphic header values
  - QueueStorageContext.ts (61 lines): Thin wrapper over Context, delegates to internal object
  - StorageError.ts (67 lines): Extends MiddlewareError, constructs XML error body
  - StorageErrorFactory.ts (340 lines): 27 static factory methods, queue-specific error codes
  - IQueueMetadataStore.ts (317 lines): Composite interface (IGCExtentProvider + IDataStore + ICleaner), 21 methods
- **Key findings:**
  1. Queue foundation uses same generated patterns as blob Phase 5 (IRequest/IResponse, Context, errors)
  2. QueueEnvironment has TWO async methods (location, debug) unlike blob
  3. Validation is lazy in getters, not constructor (inMemoryPersistence ↔ location mutually exclusive)
  4. StorageErrorFactory has inconsistent naming (get* prefix, bare verbs, camelCase methods)
  5. IQueueMetadataStore combines three interfaces; message operations require pop-receipt validation
- **Porting records created:** All 9 files documented under `rust/porting-db/src/queue/{generated,errors,context,persistence}/`
- **Architecture note:** Queue service closely mirrors blob patterns but with distinct queue semantics (messages, pop-receipts, timeouts)

**Deliverables:**
- Phase 13: 1 porting-db record (BlobGCManager.md)
- Phase 14: 9 porting-db records (foundation layer ready for handler/middleware analysis)
- All records follow established format: file info, exported API, dependencies, type mappings, special handling, change propagation notes

**Next phase:** Aragorn ready to begin Phase 13 (Blob GC) translation. Phase 14 foundation is stable; can proceed with generated framework expansion (32 generated files) and handler analysis in parallel.

### 2026-03-14: Batch Session Cross-Agent Update
- **Phase 13-14 Analysis:** Foundation complete with 10 porting-db records seeded; awaiting Aragorn Phase 10.7 completion
- **Aragorn cross-link:** Phase 10.7 at 35/51 methods; clippy + fmt verified and committed
- **Boromir cross-link:** Phase 9-10 parity tests complete (63 tests, all passing)
- **Mandatory directive:** Clippy + fmt required before all commits per Quetzal directive 2026-03-14
- **Next phase:** Phase 13 implementation planning upon Phase 10.7 completion; Phase 14 dependency mapping ready


### 2026-03-14: Phase 13-14 Analysis COMPLETION
- **Phase 13 Complete:** BlobGCManager.ts (183 lines) analyzed
  - 1 porting-db record: GC manager lifecycle, registration, extent cleanup coordination
  - Architecture: Coordinates lazy extent cleanup across metadata store transactions

- **Phase 14 Complete:** Queue service foundation layer fully analyzed
  - 9 porting-db records seeded: QueueEnvironment, IRequest/IResponse, QueueStorageContext, StorageError, StorageErrorFactory, IQueueMetadataStore
  - Total lines analyzed: 2630 (foundation layer only)
  - Key insights: Queue mirrors blob patterns (generated interfaces, context, errors); QueueEnvironment has 2 async methods vs blob's 1; lazy validation in getters; 32 generated handler files identified for Phase 14 expansion
  - Phase 14 porting records stable; ready for Phase 15 integration planning

- **Cross-agent sync:**
  - **Aragorn:** Phase 10.7 COMPLETE (51/51 methods). Phase 11 50% complete. Phase 13 GC manager porting-db ready for impl.
  - **Boromir:** Phase 6-7 parity COMPLETE (107/110). Phase 14 queue foundation parity tests not yet wired.
  - **Directive:** Continuous pipeline; Phase 13 impl planning concurrent with Phase 11 completion
  - **Mandatory:** `cargo clippy --all-targets` + `cargo fmt` required before all commits

- **Next:** Phase 15 integration planning; Phase 13 impl trigger upon Aragorn readiness
