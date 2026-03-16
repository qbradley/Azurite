# Gandalf — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### 2026-03-17: Parity Analysis — Lessons Learned & Roadmap

**Requested by:** Quetzal Bradley
**Artifact:** `.squad/decisions/inbox/gandalf-parity-analysis.md`

**Key architectural insights from the TS→Rust port:**

1. **Implicit atomicity is the #1 porting hazard.** Node.js's single-threaded event loop makes read-modify-write sequences atomic within await-free spans. Rust's `Arc<RwLock<>>` breaks this atomicity. All 6 blob race conditions came from this single root cause. Every metadata store method that reads-then-writes must hold a single write lock across the entire operation.

2. **Zero bugs were caught by existing TS tests.** Every bug found (34+ across 5 categories) was discovered by SDK integration tests, handler-by-handler audit, or customer reports. Passing the TS test suite provides happy-path confidence only — no assurance of parity for error paths, concurrency, wire format, or mutation side effects.

3. **Serialization libraries ARE the protocol.** TS's `ms-rest-js` and `xml2js` embed Azure-specific conventions (RFC 1123 dates, xmlIsWrapped, duplicate-sibling-as-array) that aren't in the REST API spec. A faithful code translation that uses different serialization libraries produces different wire format. Defense-in-depth (e.g., `ensure_rfc1123()` safety nets at serialization boundary) is essential.

4. **Table store has unfixed TOCTOU races.** `insertOrUpdateTableEntity` and `insertOrMergeTableEntity` perform check-then-act across separate lock acquisitions. Needs single-lock-scope refactor.

5. **Queue store is safe.** Uses write locks consistently for all read-modify-write paths. Simpler operations (visibility updates, deletes) don't have the multi-field atomicity hazard that blob's page range merging does.

6. **Differential testing is the single highest-value investment.** A dual-server harness comparing raw HTTP responses between TS and Rust would have caught Categories B (serialization, 8+ bugs) and C (semantic, 12+ bugs) automatically. Building this is the #1 priority for parity assurance.

7. **Multi-SDK testing catches what single-SDK testing cannot.** The Rust SDK caught the RFC 1123 date bug immediately because it has a strict date parser. The JS SDK was lenient and never noticed. Testing with Python, .NET, Java, and Go SDKs will surface similar hidden assumptions.

### 2026-03-13: Full Codebase Analysis Complete

**Architecture Discovery:**
- 373 TS files total: 277 handwritten (~49,500 LOC), 96 autorest-generated (~34,900 LOC)
- Three services (blob, queue, table) + common layer — all follow identical layered architecture
- Express-based HTTP framework with 12-step middleware chain
- LokiJS for in-memory persistence, optional SQL (Sequelize) for blob
- Constructor injection throughout (no IoC container, no decorators)
- Shallow inheritance (max 2-3 levels): ServerBase→XServer, ConfigurationBase→XConfiguration, BaseHandler→XHandler
- State machines: ServerBase (4 states), Lease (5 states), GC (4 states)
- Autorest generates from Swagger specs in `swagger/` — generates middleware, handlers, models, mappers, context

**Key File Paths (largest/most complex):**
- `src/blob/persistence/LokiBlobMetadataStore.ts` — 3565 lines, heart of blob storage
- `src/blob/persistence/SqlBlobMetadataStore.ts` — 3579 lines, SQL alternative
- `src/blob/persistence/IBlobMetadataStore.ts` — 1165 lines, the contract
- `src/blob/handlers/BlobHandler.ts` — 1350 lines, blob CRUD operations
- `src/blob/generated/artifacts/mappers.ts` — 7394 lines, serialization mappers
- `src/blob/errors/StorageErrorFactory.ts` — 854 lines, all blob error types
- `src/blob/authentication/IBlobSASSignatureValues.ts` — 818 lines, SAS signing
- `src/table/batch/TableBatchOrchestrator.ts` — 755 lines, batch processing
- `src/table/handlers/TableHandler.ts` — 1188 lines, table operations

**Patterns Requiring Special Rust Handling:**
- TS intersection types (`A & B & C`) → flatten into single Rust struct
- TS union types (`A | B`) → Rust enum
- TS optional properties (`prop?: T`) → `Option<T>` everywhere
- TS `any` in generated code → narrowed to concrete types per-file
- Express middleware chain → axum Tower middleware (preserve ordering)
- EventEmitter-based async signaling → tokio channels/notify
- Promise-based mutex/operation queue → tokio::sync primitives
- Node.js streams → tokio AsyncRead/AsyncWrite

**Decisions Made:**
- D-004 through D-016 — see porting-db/STRATEGY.md §17 Decision Log
- Most impactful: axum for HTTP, tokio for async, composition over inheritance, custom in-memory store (not LokiJS clone)

**VS Code Extension OUT OF SCOPE:** 14 VSC*.ts files + extension.ts — JS-only APIs, cannot port to Rust

### 2026-03-13: Workspace Ready and Phase 1 Analysis Complete

**Aragorn Status:** Rust workspace scaffold complete. Five-crate structure compiles. All Phase 0 tasks done. Ready to begin Phase 1 implementation.

**Faramir Status:** Phase 1 TS analysis complete. All 15 common interface files analyzed. Critical fidelity concerns documented:
- `IOperationQueue.operate<T>()` generics may require special Rust handling (trait object safety)
- `IExtentMetadata` vs `IExtentMetadataStore` are intentionally distinct contracts — preserve separation in Rust
- `contextID`/`contextId` naming inconsistencies must be preserved
- `IEnvironment` and `IServerFactory` abstractions need careful translation to maintain TS semantics

**Next Phase:** Aragorn proceeds with Phase 1 implementation using Faramir's analysis records. Watch for trait object boundaries and model distinctions per Faramir's fidelity concerns.


**Decision Rationale:**
- Per user directive: All Rust artifacts must live under `rust/` (except `.squad/` metadata)
- Strategy docs (STRATEGY.md, PORTING-ORDER.md) are not production code but reference docs for the porting process
- Decision: Move porting-db/ to rust/porting-db/ to maintain single-rooted `rust/` directory

**Changes Applied:**
- Created `rust/porting-db/` directory
- Moved `porting-db/STRATEGY.md` → `rust/porting-db/STRATEGY.md` with all path references updated
- Moved `porting-db/PORTING-ORDER.md` → `rust/porting-db/PORTING-ORDER.md` with all path references updated
- Updated 4 internal references in STRATEGY.md (`porting-db/` → `rust/porting-db/`)
- Template examples in §16 (Record Format) now reference correct paths

**Directory Structure Implications:**
```
rust/
├── Cargo.toml                          # Workspace root
├── crates/                             # Service implementations
│   ├── azurite/                        # Combined binary
│   ├── azurite-common/                 # Shared library
│   ├── azurite-blob/                   # Blob service
│   ├── azurite-queue/                  # Queue service
│   └── azurite-table/                  # Table service
└── porting-db/                         # ⬅️ Strategy and per-file porting records
    ├── STRATEGY.md                     # This Rust porting strategy
    ├── PORTING-ORDER.md                # File-level porting order
    ├── README.md                       # Porting-db documentation
    └── src/                            # YAML records mirror TS structure
        ├── common/
        ├── blob/
        ├── queue/
        └── table/
```

**Why This Matters for Future Change Propagation:**
- Single `rust/` root makes it clear all port artifacts are here
- porting-db records stay close to Rust implementations (in same `rust/` subtree)
- Easy for Aragorn/Faramir to reference strategy docs while implementing features
- Clear boundary: TypeScript source in `/src`, Rust port in `/rust`

## 2026-03-14T00:00 — Batch Completion: Phase 6-7 + Phase 8-10 + Phase 5 QA

**Summary:** Three concurrent agent batches completed. 174 Rust files, 129 porting-db records, 78 passing tests.

**Batch 1 — Aragorn (Phase 6-7 Implementation):**
- 19 blob error/auth files translated
- Decision: Preserve existing blob-authentication quirks for contract fidelity
- cargo check passing

**Batch 2 — Faramir (Phase 8-10 Analysis):**
- 38 porting-db records seeded (lease + persistence)
- Key findings: lazy lease timers, LeaseExpiredState.renew() quirks, QueryParser `not` gap, LokiBlobMetadataStore normalization
- Phase numbering clarified: Phase 8 = lease, Phase 10 = persistence (not Phase 9 as briefly discussed)

**Batch 3 — Boromir (Phase 5 QA + XML Fix):**
- 78 tests passing
- Fixed quick_xml root element serialization bug
- Metadata-driven parity strategy established

**Cross-Agent Learning:**
- Aragorn ready for Phase 8 implementation
- Boromir can expand test coverage incrementally as implementations land
- Faramir's fidelity flags (lazy timers, renew() quirks, QueryParser gap, metadata normalization) now documented for Aragorn's Phase 8-10 work

**Next:** Continuous pipeline — Phase 8 (lease) implementation scheduled to follow.


## 2026-03-14: QUICKSTART.md for Rust Azurite Created

**By:** Gandalf (Lead Architect)

**What:** Created `/rust/QUICKSTART.md` — a comprehensive end-user guide for Rust Azurite. The guide covers:
- Prerequisites (Rust 1.77+) with installation and verification instructions
- Building release binaries (`cargo build --release`) with binary location guidance
- Running combined services and individual services (blob, queue, table separately)
- Common options: custom ports/host, in-memory mode, silent/debug logging, loose mode, SSL/HTTPS, OAuth
- Client connection examples: Python SDK, Node.js SDK, .NET SDK, Azure CLI, curl REST API
- Implementation status matrix (Blob ✓, Queue ✓, Table in progress) with known limitations
- Troubleshooting guide and Docker containerization example
- Links to main README and porting-db for deeper technical details

**Key Design Decisions:**
- Friendly, practical tone assuming Rust installed but not Azurite internals
- Heavy use of code blocks with realistic examples in bash, Python, JavaScript, C#
- All CLI options documented with practical examples and default values
- Default ports (10000/10001/10002) and development credentials explicitly documented
- Both `cargo run` and direct binary execution documented side-by-side
- Clear distinction between combined vs individual service execution patterns
- Pre-flight troubleshooting section (port conflicts, workspace errors, connection failures)

**Why:** End-user documentation is critical for successful adoption of the Rust port. Developers need practical, clear guidance to:
1. Set up and build the project
2. Run services locally (combined or individually)
3. Connect from their applications (all major SDKs)
4. Configure for their specific use case (custom ports, HTTPS, logging)
5. Troubleshoot common issues

Well-written documentation reduces support burden, improves developer experience, and accelerates adoption. This guide follows the style and structure of the main README.md but optimized for hands-on, getting-started scenarios.

**Scope:** Public-facing documentation for Azurite Rust end-users. Complements the technical porting-db records maintained for implementers.

**Impact:** Azurite Rust is now ready for end-user adoption with comprehensive, practical guidance covering all common use cases and client library integrations.

