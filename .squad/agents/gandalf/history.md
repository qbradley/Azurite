# Gandalf — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

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
