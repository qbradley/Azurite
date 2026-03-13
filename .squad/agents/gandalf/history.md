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
