# Faramir — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### External server mode harness learnings (2026-03-14)
- Blob/Queue/Table integration suites only rely on a narrow server surface from the factories: `server.config.host`, `server.config.port`, `start()`, `close()`, and occasionally `clean()`. That means external-server mode can stay minimally invasive by returning a no-op stub with the same observable shape instead of restructuring the tests.
- Table coverage has one extra coupling point beyond `TableTestServerFactory`: REST submitters and shared table test helpers read host/port from `tests/table/models/table.entity.test.config.ts` and `tests/table/utils/table.entity.test.utils.ts`. Without env-backed host/port there, external mode would still point many table suites at hardcoded `127.0.0.1:11002`.

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

## 2026-03-13 — Phases 0-9 Analysis Archive (Historical)

All analysis phases 0-9 completed before 2026-03-14:
- Phase 0-3: Workspace/common analysis (70 porting-db records)
- Phase 4: Utilities/config analysis (11 files)
- Phase 5: Blob generated framework analysis (34 files)
- Phase 6-7: Error/auth analysis (74 StorageErrorFactory + 27 auth/blob handlers)
- Phase 8: Lease subsystem analysis (15 Rust files)
- Phase 9-10: Condition/persistence analysis (42 porting-db records)

### Integration/E2E test infrastructure investigation completed (2026-03-14)
- Azurite already has a strong reusable black-box test bed: 41 TypeScript integration/E2E suites under `tests/blob/`, `tests/queue/`, and `tests/table/`, plus table-specific cross-language conformance coverage in `tests/table/dotnet/AzuriteTableTest/*.cs` and `tests/table/go/main.go`.
- The reusable suites mostly drive Azurite through official Azure SDKs (`@azure/storage-blob`, `@azure/storage-queue`, `@azure/data-tables`, legacy `azure-storage`) and therefore validate observable protocol behavior rather than TS internals.
- Main blocker for Rust reuse is harness shape, not test quality: Blob/Queue/Table test factories directly start TypeScript server objects on hardcoded ports `11000/11001/11002`, and some table helpers/raw REST payloads also hardcode endpoint details (`tests/table/models/table.entity.test.config.ts`, `tests/table/apis/table.entity.rest.test.ts`).
- Best reuse strategy is a thin external-server mode in the existing TS harness so the same Mocha suites can point at Rust without rewriting assertions. Recommended execution order remains blob first, queue second, table third; use table .NET/Go conformance only after table API parity stabilizes.

### Porting-db backfill for queue and table services (2026-07-15)
- Created 159 new porting-db records: 56 for queue (`rust/porting-db/src/queue/`), 103 for table (`rust/porting-db/src/table/`). Total porting-db now has 335 records across all services.
- Queue records: 72 total (was 16). Covers all 81 Rust source files across authentication (10), context, errors (3), gc, generated framework (26 per-file), handlers (5), middlewares (4), persistence (3), utils (2), and root files (2). Per-file records reference blob equivalents where patterns are identical.
- Table records: 106 total (was 3). Covers all ~100 Rust source files across authentication (11 — includes SharedKeyLite unique to table), batch (1 aggregate for 18 TS files), context, entity (12 per-file EDM types), errors (3), generated framework (26 per-file), handlers (6 — includes batch handler/sub-request/sub-response), middleware (4), persistence (22 — includes full QueryInterpreter AST node tree with 17 query nodes), utils (2), and root files (5).
- Updated all pre-existing aggregate records from `analyzed`/`not_started` status to `ported`.
- Table-unique subsystems documented with extra fidelity detail: batch (OData multipart protocol, transaction semantics), QueryInterpreter (recursive-descent parser, OData $filter, typed literal handling), EDM type system (9 types with OData annotation levels), TableHandler (1188 LOC — largest handler covering all entity CRUD), SharedKeyLite auth (legacy SDK compatibility).
- Queue-unique subsystems: MessagesHandler (dequeue with visibility timeout), MessageIdHandler (pop receipt validation), QueueGCManager (message expiry sweep).

## FINAL STATUS: ✅ PORT COMPLETE (2026-03-18T03:30)

**Completion Milestone:** Porting-DB backfill complete. 335 total records (queue: 72, table: 106, blob: 127, other: 30). All 435 Rust files documented for change propagation. Full porting database is now authoritative source for future TS→Rust updates.

