# Azurite TypeScript → Rust Porting Strategy

> **Author:** Gandalf (Lead Architect)
> **Date:** 2026-03-13
> **Status:** PROPOSED — Awaiting team review
> **Governing Principle:** Fidelity with TypeScript over idiomatic Rust. Every decision asks: "Will this make it easy to propagate future TS changes to Rust?"

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Codebase Anatomy](#2-codebase-anatomy)
3. [Rust Project Structure](#3-rust-project-structure)
4. [Module Porting Order](#4-module-porting-order)
5. [Type Mapping Strategy](#5-type-mapping-strategy)
6. [Async Strategy](#6-async-strategy)
7. [Error Handling Strategy](#7-error-handling-strategy)
8. [Class/OOP Mapping Strategy](#8-classoop-mapping-strategy)
9. [Dependency Mapping](#9-dependency-mapping)
10. [Generated Code Strategy](#10-generated-code-strategy)
11. [Persistence Layer Strategy](#11-persistence-layer-strategy)
12. [Authentication Strategy](#12-authentication-strategy)
13. [Middleware Chain Strategy](#13-middleware-chain-strategy)
14. [Stream/IO Strategy](#14-streamio-strategy)
15. [Testing Strategy](#15-testing-strategy)
16. [Porting Database Format](#16-porting-database-format)
17. [Decision Log](#17-decision-log)

---

## 1. Project Overview

Azurite is an Azure Storage Emulator implementing three Azure Storage services:

| Service | TS Entry Point | CLI Binary |
|---------|---------------|------------|
| **Blob Storage** | `src/blob/main.ts` | `azurite-blob` |
| **Queue Storage** | `src/queue/main.ts` | `azurite-queue` |
| **Table Storage** | `src/table/main.ts` | `azurite-table` |
| **Combined** | `src/azurite.ts` | `azurite` |

Plus a VS Code extension entry at `src/extension.ts` (out of scope for initial Rust port).

### Codebase Statistics

| Category | Files | Lines |
|----------|-------|-------|
| Handwritten TypeScript | 277 | ~49,500 |
| Autorest-generated TypeScript | 96 | ~34,900 |
| **Total** | **373** | **~84,400** |

---

## 2. Codebase Anatomy

### 2.1 Directory Structure

```
src/
├── azurite.ts              # Combined server entry point
├── main.ts                 # Re-export from extension.ts
├── extension.ts            # VS Code extension (OUT OF SCOPE)
├── common/                 # Shared infrastructure (39 files)
│   ├── ServerBase.ts           # Abstract HTTP/HTTPS server with lifecycle state machine
│   ├── ConfigurationBase.ts    # CLI args, certs, OAuth config
│   ├── Environment.ts          # Environment implementation
│   ├── IDataStore.ts           # Base persistence contract
│   ├── IAccountDataStore.ts    # Account credential management
│   ├── AccountDataStore.ts     # Account store implementation
│   ├── ILogger.ts              # Logger contract (5 levels)
│   ├── Logger.ts               # Logger singleton
│   ├── Mutex.ts                # Promise-based async mutex
│   ├── Telemetry.ts            # App Insights telemetry
│   ├── persistence/            # Shared extent storage layer
│   │   ├── IExtentStore.ts         # Extent read/write interface
│   │   ├── IExtentMetadataStore.ts # Extent metadata interface
│   │   ├── FSExtentStore.ts        # File system implementation
│   │   ├── MemoryExtentStore.ts    # In-memory implementation
│   │   ├── LokiExtentMetadataStore.ts # LokiJS metadata
│   │   ├── OperationQueue.ts       # Concurrency-limited job queue
│   │   └── ...
│   ├── authentication/         # Shared SAS/auth primitives
│   ├── utils/                  # Shared utilities (HMAC, dates, etag)
│   └── VSC*.ts                 # VS Code integration (OUT OF SCOPE)
├── blob/                   # Blob Storage service (~120 files)
│   ├── BlobServer.ts           # Server (extends ServerBase)
│   ├── BlobConfiguration.ts    # Config (extends ConfigurationBase)
│   ├── BlobEnvironment.ts      # CLI environment
│   ├── BlobRequestListenerFactory.ts # Express app factory
│   ├── handlers/               # API operation implementations
│   │   ├── ServiceHandler.ts       # Service-level operations
│   │   ├── ContainerHandler.ts     # Container operations
│   │   ├── BlobHandler.ts          # Blob CRUD operations
│   │   ├── BlockBlobHandler.ts     # Block blob specifics
│   │   ├── PageBlobHandler.ts      # Page blob specifics
│   │   ├── AppendBlobHandler.ts    # Append blob specifics
│   │   └── BlobBatchHandler.ts     # Batch operation support
│   ├── persistence/            # Blob-specific persistence
│   │   ├── IBlobMetadataStore.ts   # Blob metadata interface (1165 lines)
│   │   ├── LokiBlobMetadataStore.ts # LokiJS implementation (3565 lines)
│   │   ├── SqlBlobMetadataStore.ts  # SQL implementation (3579 lines)
│   │   └── QueryInterpreter/      # OData query parsing
│   ├── authentication/         # Blob auth (SharedKey, SAS, OAuth, Public)
│   ├── conditions/             # Conditional headers (ETag, If-Match, etc.)
│   ├── context/                # BlobStorageContext
│   ├── errors/                 # StorageError + factory
│   ├── gc/                     # Garbage collection manager
│   ├── lease/                  # Lease state machine (5 states)
│   ├── middlewares/            # Auth, preflight, context, telemetry
│   └── generated/              # Autorest-generated (34 files)
├── queue/                  # Queue Storage service (~50 files)
│   ├── QueueServer.ts
│   ├── handlers/               # Service, Queue, Messages, MessageId
│   ├── persistence/            # IQueueMetadataStore + LokiJS impl
│   ├── authentication/
│   ├── context/
│   ├── errors/
│   ├── gc/
│   ├── middlewares/
│   └── generated/              # Autorest-generated (32 files)
└── table/                  # Table Storage service (~70 files)
    ├── TableServer.ts
    ├── handlers/               # Service, Table
    ├── persistence/            # ITableMetadataStore + LokiJS impl
    ├── batch/                  # Batch orchestration (unique to Table)
    ├── entity/                 # EDM type system (9 types)
    ├── authentication/
    ├── context/
    ├── errors/
    ├── middleware/
    └── generated/              # Autorest-generated (30 files)
```

### 2.2 Architecture Pattern

Every service follows an identical layered architecture:

```
CLI (main.ts)
  → Environment (parses args)
    → Configuration (structured config)
      → Server (extends ServerBase, owns lifecycle)
        → RequestListenerFactory (creates Express app)
          → Middleware Chain (12-step pipeline):
              morgan → Context → Dispatch → Auth → Deserialize
              → Handle → CORS × 2 → Serialize → Options → Error → End
            → Handlers (business logic, call persistence)
              → MetadataStore (LokiJS or SQL persistence)
              → ExtentStore (blob data storage)
```

### 2.3 Key Patterns Identified

| Pattern | Where Used | Rust Mapping |
|---------|-----------|--------------|
| **Constructor injection** | All classes | Struct fields, `new()` constructors |
| **Abstract base + subclass** | ServerBase, ConfigurationBase, BaseHandler, LeaseStateBase | Trait + default methods + structs |
| **Interface-driven design** | All persistence, auth, logging | Trait objects or generics |
| **Factory pattern** | ServerFactory, RequestListenerFactory, LeaseFactory, ErrorFactory | Builder pattern or factory functions |
| **State machine** | ServerBase (4 states), Lease (5 states), GC (4 states) | Enum-based state machines |
| **Visitor pattern** | Lease sync/validate | Trait methods |
| **Strategy pattern** | Logger strategies, authenticators | Trait objects |
| **Template method** | ServerBase lifecycle hooks | Trait with default implementations |
| **Async middleware chain** | Express pipeline (12 steps) | Tower middleware / manual chain |
| **Promise-based concurrency** | Mutex, OperationQueue, GCManager | tokio::sync primitives |
| **EventEmitter signaling** | GC abort, OperationQueue results | tokio channels / notify |
| **Stream composition** | Extent reads, multistream | tokio::io / futures::Stream |

---

## 3. Rust Project Structure

The Rust project mirrors the TypeScript structure 1:1 to maximize change-propagation fidelity.

```
rust/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── azurite/                # Combined binary (→ src/azurite.ts)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── azurite-common/         # Shared library (→ src/common/)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server_base.rs          # → ServerBase.ts
│   │       ├── configuration_base.rs   # → ConfigurationBase.ts
│   │       ├── environment.rs          # → Environment.ts
│   │       ├── i_data_store.rs         # → IDataStore.ts
│   │       ├── i_account_data_store.rs # → IAccountDataStore.ts
│   │       ├── account_data_store.rs   # → AccountDataStore.ts
│   │       ├── i_logger.rs             # → ILogger.ts
│   │       ├── logger.rs               # → Logger.ts
│   │       ├── mutex.rs                # → Mutex.ts
│   │       ├── models.rs               # → models.ts
│   │       ├── persistence/
│   │       │   ├── mod.rs
│   │       │   ├── i_extent_store.rs
│   │       │   ├── i_extent_metadata_store.rs
│   │       │   ├── fs_extent_store.rs
│   │       │   ├── memory_extent_store.rs
│   │       │   ├── loki_extent_metadata_store.rs
│   │       │   ├── operation_queue.rs
│   │       │   └── ...
│   │       ├── authentication/
│   │       │   ├── mod.rs
│   │       │   └── ...
│   │       └── utils/
│   │           ├── mod.rs
│   │           ├── constants.rs
│   │           └── utils.rs
│   ├── azurite-blob/           # Blob service library + binary (→ src/blob/)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs                 # → blob/main.ts
│   │       ├── blob_server.rs          # → BlobServer.ts
│   │       ├── blob_configuration.rs   # → BlobConfiguration.ts
│   │       ├── blob_environment.rs     # → BlobEnvironment.ts
│   │       ├── blob_request_listener_factory.rs
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── base_handler.rs
│   │       │   ├── service_handler.rs
│   │       │   ├── container_handler.rs
│   │       │   ├── blob_handler.rs
│   │       │   ├── block_blob_handler.rs
│   │       │   ├── page_blob_handler.rs
│   │       │   ├── append_blob_handler.rs
│   │       │   └── blob_batch_handler.rs
│   │       ├── persistence/
│   │       │   ├── mod.rs
│   │       │   ├── i_blob_metadata_store.rs
│   │       │   ├── loki_blob_metadata_store.rs
│   │       │   └── query_interpreter/
│   │       ├── authentication/
│   │       ├── conditions/
│   │       ├── context/
│   │       ├── errors/
│   │       ├── gc/
│   │       ├── lease/
│   │       ├── middlewares/
│   │       └── generated/      # See §10 for generated code strategy
│   ├── azurite-queue/          # Queue service (→ src/queue/)
│   │   └── src/  ... (mirrors queue/ structure)
│   └── azurite-table/          # Table service (→ src/table/)
│       └── src/  ... (mirrors table/ structure)
```

### 3.1 File Naming Convention

| TypeScript | Rust |
|-----------|------|
| `BlobServer.ts` | `blob_server.rs` |
| `IBlobMetadataStore.ts` | `i_blob_metadata_store.rs` |
| `StorageErrorFactory.ts` | `storage_error_factory.rs` |

**Rationale:** Keep the `I` prefix for interface files even in Rust. This preserves 1:1 file-level correspondence for change propagation, which is the #1 goal.

### 3.2 Workspace Dependencies

```
azurite (bin) ──depends──→ azurite-blob (lib)
                         → azurite-queue (lib)
                         → azurite-table (lib)
                         
azurite-blob ──depends──→ azurite-common
azurite-queue ──depends─→ azurite-common
azurite-table ──depends─→ azurite-common
```

---

## 4. Module Porting Order

See [PORTING-ORDER.md](./PORTING-ORDER.md) for the detailed file-level plan.

### 4.1 Phase Summary

| Phase | Module | Rationale | Files (approx) |
|-------|--------|-----------|----------------|
| **0** | Project scaffolding | Cargo workspace, CI, tooling | N/A |
| **1** | `common/` core interfaces | Foundation for everything | ~15 |
| **2** | `common/` persistence layer | Extent storage needed by all services | ~10 |
| **3** | `common/` authentication | Shared SAS/auth primitives | ~5 |
| **4** | `common/` utilities + config | Utils, logger, environment, config base | ~10 |
| **5** | Generated framework (`blob/generated/`) | Middleware factory, context, adapters | ~34 |
| **6** | `blob/` errors + context | Error types needed by handlers | ~5 |
| **7** | `blob/` authentication | Blob-specific auth | ~12 |
| **8** | `blob/` lease subsystem | State machine, used by handlers | ~14 |
| **9** | `blob/` conditions | Conditional headers | ~7 |
| **10** | `blob/` persistence | IBlobMetadataStore + LokiJS impl | ~8 |
| **11** | `blob/` handlers | All blob operation handlers | ~12 |
| **12** | `blob/` middleware + server | Wire everything together | ~8 |
| **13** | `blob/` GC | Garbage collection manager | ~2 |
| **14** | `queue/` (full service) | Smaller service, similar patterns | ~50 |
| **15** | `table/` (full service) | Largest handler, batch, EDM types | ~70 |
| **16** | Combined `azurite` binary | Top-level entry point | ~2 |
| **17** | SQL persistence (optional) | SqlBlobMetadataStore, Sequelize→sqlx | ~3 |

### 4.2 Dependency Graph

```
Phase 0 (scaffolding)
  ↓
Phase 1 (common interfaces) ← EVERYTHING depends on this
  ↓
Phase 2 (common persistence) ← All services depend on extent storage
  ↓
Phase 3 (common auth) ← All services depend on SAS primitives
  ↓
Phase 4 (common utils/config) ← All services depend on logger, env, config
  ↓
Phase 5 (generated framework) ← All handlers depend on generated types
  ↓
Phases 6-13 (blob service) ← First complete service
  ↓
Phase 14 (queue service) ← Follows same patterns as blob
  ↓
Phase 15 (table service) ← Most complex service, benefits from blob/queue lessons
  ↓
Phase 16 (combined binary) ← Final integration
  ↓
Phase 17 (SQL persistence) ← Optional, can be deferred
```

---

## 5. Type Mapping Strategy

### 5.1 Primitive Types

| TypeScript | Rust | Notes |
|-----------|------|-------|
| `string` | `String` | Owned strings by default |
| `&str` parameters | `&str` | Use for read-only string params |
| `number` (integer context) | `i64` | Default for integer-like numbers |
| `number` (float context) | `f64` | Default for float-like numbers |
| `number` (port, status code) | `u16` | Context-specific sizing |
| `number` (byte offset/count) | `u64` | Extent offsets |
| `boolean` | `bool` | Direct |
| `Date` | `chrono::DateTime<Utc>` | Use chrono crate |
| `undefined` | `None` (in `Option<T>`) | See §5.2 |
| `null` | `None` (in `Option<T>`) | Same treatment as undefined |
| `Buffer` | `Vec<u8>` or `Bytes` | Use `bytes` crate for zero-copy |
| `Uint8Array` | `Vec<u8>` | Direct |
| `any` | Context-dependent | See §5.3 |

### 5.2 Optional / Nullable Types

TypeScript's `prop?: T` and `T | undefined` both map to `Option<T>` in Rust.

```typescript
// TypeScript
interface IBlobAdditionalProperties {
  accountName: string;             // Required
  leaseDurationSeconds?: number;   // Optional
  leaseId?: string;                // Optional
  leaseExpireTime?: Date;          // Optional
}
```

```rust
// Rust (faithful translation)
pub struct BlobAdditionalProperties {
    pub account_name: String,                          // Required
    pub lease_duration_seconds: Option<i64>,            // Optional
    pub lease_id: Option<String>,                       // Optional
    pub lease_expire_time: Option<chrono::DateTime<Utc>>, // Optional
}
```

### 5.3 The `any` Problem

The TS codebase uses `any` in several places (especially generated code). Strategy:

| Context | Rust Mapping |
|---------|-------------|
| `any` as return from handler | `Box<dyn Any>` or concrete type if inferable |
| `any` in middleware callbacks | `Box<dyn Fn(...) -> ...>` with concrete signature |
| `any` in context storage | `HashMap<String, serde_json::Value>` |
| `any` in generated models | Replace with concrete enum of known types |

**Decision:** Prefer narrowing `any` to concrete types wherever possible. Only use `Box<dyn Any>` as a last resort. Document every `any` resolution in the porting-db record for that file.

### 5.4 Union Types

```typescript
// TypeScript
type BlobModel = IBlobAdditionalProperties &
  IPageBlobAdditionalProperties &
  IBlockBlobAdditionalProperties &
  Models.BlobItemInternal &
  IPersistencyPropertiesOptional;
```

**Strategy for Intersections (`&`):** Flatten into a single struct with all fields.

```rust
// Rust: Flatten intersection types into one struct
pub struct BlobModel {
    // From IBlobAdditionalProperties
    pub account_name: String,
    pub container_name: String,
    pub lease_duration_seconds: Option<i64>,
    // From IPageBlobAdditionalProperties
    pub page_ranges_in_order: Option<Vec<PersistencyPageRange>>,
    // From IBlockBlobAdditionalProperties
    pub is_committed: bool,
    pub committed_blocks_in_order: Option<Vec<PersistencyBlockModel>>,
    // From BlobItemInternal (generated model fields)
    // ...
    // From IPersistencyPropertiesOptional
    pub persistency: Option<ExtentChunk>,
}
```

**Strategy for Unions (`|`):** Use Rust enum.

```typescript
// TypeScript
data: NodeJS.ReadableStream | Buffer
```

```rust
// Rust
pub enum StreamOrBuffer {
    Stream(Box<dyn AsyncRead + Send + Unpin>),
    Buffer(Vec<u8>),
}
```

### 5.5 Interface → Trait Mapping

```typescript
// TypeScript
interface IDataStore {
  init(): Promise<void>;
  isInitialized(): boolean;
  close(): Promise<void>;
  isClosed(): boolean;
}
```

```rust
// Rust
#[async_trait]
pub trait DataStore: Send + Sync {
    async fn init(&mut self) -> Result<()>;
    fn is_initialized(&self) -> bool;
    async fn close(&mut self) -> Result<()>;
    fn is_closed(&self) -> bool;
}
```

### 5.6 Generic Types

The codebase uses minimal generics. The main instances:

| TypeScript Generic | Rust Mapping |
|-------------------|-------------|
| `ILeaseSyncer<T>` | `trait LeaseSyncer { type Output; fn sync(...) -> Self::Output; }` |
| `ConstantNode<T>` | `struct ConstantNode<T> { value: T }` |
| `DateTimeNode<T>` | `struct DateTimeNode<T> { value: T }` |
| `Promise<T>` | `async fn -> Result<T>` |

### 5.7 Enum Mapping

```typescript
// TypeScript: String enums
enum ServerStatus {
  Closed = "Closed",
  Starting = "Starting",
  Running = "Running",
  Closing = "Closing"
}
```

```rust
// Rust: Preserve string representations for debugging
#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Closed,
    Starting,
    Running,
    Closing,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Starting => write!(f, "Starting"),
            Self::Running => write!(f, "Running"),
            Self::Closing => write!(f, "Closing"),
        }
    }
}
```

---

## 6. Async Strategy

### 6.1 Runtime: Tokio

**Decision:** Use **tokio** as the async runtime.

**Rationale:**
- axum (our Express replacement) requires tokio
- tokio is the de facto Rust async ecosystem standard
- tokio::sync provides Mutex, RwLock, Semaphore, channels — mapping directly to TS patterns

### 6.2 Promise → async/Result

```typescript
// TypeScript
async function createContainer(name: string): Promise<ContainerModel> {
  // ...
}
```

```rust
// Rust
async fn create_container(&self, name: &str) -> Result<ContainerModel> {
    // ...
}
```

**Every `Promise<T>` becomes `async fn -> Result<T>`**, because:
- TS Promises can reject (throw) — Rust async fns must return Result
- The `Result` type is `Result<T, StorageError>` (service-specific) or `Result<T, anyhow::Error>` (generic)

### 6.3 Specific Async Pattern Mappings

| TypeScript Pattern | Rust Equivalent |
|-------------------|----------------|
| `async/await` | `async/await` (direct) |
| `Promise.all([a, b])` | `tokio::join!(a, b)` or `futures::join!` |
| `new Promise((resolve, reject) => ...)` | `tokio::sync::oneshot::channel()` |
| `setTimeout(fn, ms)` | `tokio::time::sleep(Duration::from_millis(ms))` |
| `setImmediate(fn)` | `tokio::task::yield_now()` |
| `EventEmitter` | `tokio::sync::broadcast` or `tokio::sync::notify` |
| `process.nextTick(fn)` | `tokio::task::yield_now()` then call fn |
| Fire-and-forget `.then().catch()` | `tokio::spawn(async { ... })` |
| `timer.unref()` | Not needed in Rust (task-based, no event loop ref counting) |

### 6.4 Stream Mapping

| TypeScript | Rust |
|-----------|------|
| `NodeJS.ReadableStream` | `impl AsyncRead + Send + Unpin` or `Pin<Box<dyn AsyncRead + Send>>` |
| `NodeJS.WritableStream` | `impl AsyncWrite + Send + Unpin` |
| `stream.on("data", fn)` | `AsyncReadExt::read()` in a loop, or `StreamExt::next()` |
| `stream.pipe(dest)` | `tokio::io::copy(&mut src, &mut dest)` |
| `multistream` | `tokio_util::io::StreamReader` + chaining |

### 6.5 Mutex/Lock Mapping

The TS `Mutex` class (src/common/Mutex.ts) uses a static key-based lock map.

```rust
// Rust equivalent
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;
use std::sync::Arc;

pub struct KeyedMutex {
    locks: TokioMutex<HashMap<String, Arc<TokioMutex<()>>>>,
}
```

---

## 7. Error Handling Strategy

### 7.1 Error Model

The TS StorageError has: `statusCode`, `storageErrorCode`, `storageErrorMessage`, `storageRequestID`, and `storageAdditionalErrorMessages`.

```rust
// Rust: Direct translation
#[derive(Debug)]
pub struct StorageError {
    pub status_code: u16,
    pub storage_error_code: String,
    pub storage_error_message: String,
    pub storage_request_id: String,
    pub storage_additional_error_messages: HashMap<String, String>,
}

impl std::error::Error for StorageError {}
impl std::fmt::Display for StorageError { ... }

// Convert to HTTP response (for axum)
impl axum::response::IntoResponse for StorageError { ... }
```

### 7.2 Error Factory

The `StorageErrorFactory` static methods become associated functions:

```rust
impl StorageError {
    pub fn container_not_found(context_id: &str) -> Self { ... }
    pub fn blob_not_found(context_id: &str) -> Self { ... }
    pub fn container_already_exists(context_id: &str) -> Self { ... }
    // ... all factory methods
}
```

### 7.3 Error Propagation

| TypeScript | Rust |
|-----------|------|
| `throw new StorageError(...)` | `return Err(StorageError::...)` |
| `try { ... } catch (e) { ... }` | `match result { Ok(v) => ..., Err(e) => ... }` |
| Unhandled promise rejection | `?` operator propagates errors up |
| `Promise.reject(err)` | `Err(err)` |

### 7.4 Error Hierarchy

```
MiddlewareError (generated)
  └── StorageError (per-service: blob, queue, table)

In Rust:
  anyhow::Error (generic)
  └── StorageError (per-service, implements std::error::Error)
  └── MiddlewareError (generated framework errors)
```

---

## 8. Class/OOP Mapping Strategy

### 8.1 Guiding Principle

**Fidelity over idiom.** Keep the same class names, method names, and field names. Map classes to structs, inheritance to trait composition.

### 8.2 Class → Struct

```typescript
// TypeScript
class BlobServer extends ServerBase implements ICleaner {
  private readonly metadataStore: IBlobMetadataStore;
  constructor(metadataStore: IBlobMetadataStore, ...) { ... }
  async clean(): Promise<void> { ... }
}
```

```rust
// Rust
pub struct BlobServer {
    base: ServerBase,  // Composition, not inheritance
    metadata_store: Box<dyn BlobMetadataStore>,
    // ... other fields
}

impl Cleaner for BlobServer {
    async fn clean(&mut self) -> Result<()> { ... }
}

// Delegate ServerBase methods
impl BlobServer {
    pub async fn start(&mut self) -> Result<()> {
        self.before_start().await?;
        self.base.start_listening().await?;
        self.after_start().await?;
        Ok(())
    }
}
```

### 8.3 Inheritance Depth Mapping

The codebase has shallow inheritance (max 2-3 levels):

| TypeScript Hierarchy | Rust Strategy |
|---------------------|---------------|
| `ServerBase → BlobServer` | `BlobServer { base: ServerBase }` (composition) |
| `ConfigurationBase → BlobConfiguration` | `BlobConfiguration { base: ConfigurationBase }` |
| `BaseHandler → ServiceHandler` | `ServiceHandler { base: BaseHandler }` |
| `LeaseStateBase → LeaseLeasedState` | `LeaseLeasedState { base: LeaseStateBase }` |
| `MiddlewareFactory → ExpressMiddlewareFactory` | Trait + implementation |

### 8.4 Abstract Class → Trait + Base Struct

```typescript
// TypeScript abstract class with default methods
abstract class ServerBase {
  protected async beforeStart(): Promise<void> { /* noop */ }
  protected async afterStart(): Promise<void> { /* noop */ }
  abstract getRequestListener(): Express.Application;
}
```

```rust
// Rust: Split into trait (abstract methods) + base struct (state/defaults)
#[async_trait]
pub trait Server: Send + Sync {
    async fn before_start(&mut self) -> Result<()> { Ok(()) }  // default
    async fn after_start(&mut self) -> Result<()> { Ok(()) }   // default
    async fn before_close(&mut self) -> Result<()> { Ok(()) }  // default
    async fn after_close(&mut self) -> Result<()> { Ok(()) }   // default
}

pub struct ServerBase {
    pub status: ServerStatus,
    pub host: String,
    pub port: u16,
    // ...
}
```

### 8.5 Interface → Trait

Every TypeScript interface becomes a Rust trait. Retain the same method signatures.

```typescript
interface IBlobMetadataStore extends IDataStore {
  createContainer(context: Context, container: ContainerModel): Promise<ContainerModel>;
  // ... 40+ methods
}
```

```rust
#[async_trait]
pub trait BlobMetadataStore: DataStore {
    async fn create_container(&self, context: &Context, container: &ContainerModel)
        -> Result<ContainerModel>;
    // ... 40+ methods
}
```

---

## 9. Dependency Mapping

### 9.1 Primary Dependencies

| npm Package | Purpose in Azurite | Rust Crate | Notes |
|------------|-------------------|------------|-------|
| **express** | HTTP framework | **axum** | Tower-based, async-native |
| **lokijs** | In-memory DB | Custom HashMap-based store | See §11 |
| **winston** | Logging | **tracing** + **tracing-subscriber** | Structured logging |
| **xml2js** | XML parse/serialize | **quick-xml** | serde integration |
| **uuid** | UUID generation | **uuid** | Direct equivalent |
| **etag** | ETag generation | Custom (same algorithm) | Simple reimplementation |
| **morgan** | Access logging | **tower-http::trace** | Middleware-based |
| **stoppable** | Graceful shutdown | **tokio** signal handling | Built-in with tokio |
| **args** | CLI argument parsing | **clap** | Derive macros |
| **fs-extra** | File system ops | **tokio::fs** | Async FS |
| **jsonwebtoken** | JWT handling | **jsonwebtoken** | Direct equivalent |
| **multistream** | Stream concatenation | **tokio_util::io** | `StreamReader` + chaining |
| **to-readable-stream** | Buffer→Stream | **futures::stream** | `stream::once()` |
| **uri-templates** | RFC 6570 templates | **uri-template-system** or custom | Evaluate crate maturity |
| **glob-to-regexp** | Glob→regex | **glob** | Built-in glob support |
| **sequelize** | SQL ORM | **sqlx** | Async SQL (Phase 17) |
| **mysql2** | MySQL driver | **sqlx** (mysql feature) | Via sqlx |
| **tedious** | MSSQL driver | **sqlx** (mssql feature) | Via sqlx |
| **rimraf** | Recursive delete | **tokio::fs::remove_dir_all** | Built-in |
| **applicationinsights** | Telemetry | **opentelemetry** | Or skip for MVP |
| **axios** | HTTP client | **reqwest** | Only if needed |
| **@azure/ms-rest-js** | REST client utils | Custom or skip | Evaluate usage |

### 9.2 Dev Dependencies (Testing)

| npm Package | Rust Equivalent |
|------------|----------------|
| **mocha** | Built-in `#[test]` + **tokio::test** |
| **ts-mockito** | **mockall** |
| **@azure/storage-blob** (test client) | **azure_storage_blobs** (Rust SDK) |
| **@azure/storage-queue** | **azure_storage_queues** |
| **@azure/data-tables** | **azure_data_tables** |

---

## 10. Generated Code Strategy

### 10.1 Overview

96 files (~34,900 lines) are autorest-generated from Swagger specs in `swagger/`. These define:
- API operation specifications (URL patterns, parameters, headers)
- Request/response models and serialization mappers
- Middleware factory (dispatch, deserialize, serialize, error, end)
- Context class
- Handler interfaces
- Express adapters (request/response wrappers)

### 10.2 Decision: Manual Translation, Not Re-generation

**We will NOT re-run autorest for Rust.** Instead:
1. Manually translate the generated TS code to Rust
2. Maintain 1:1 file correspondence
3. Record each generated file in porting-db with `source: "autorest-generated"`

**Rationale:**
- Autorest does not have a Rust server generator
- The generated code is relatively stable (API spec rarely changes)
- Manual translation preserves structural correspondence
- Future Swagger changes can be diffed against the TS generated code and propagated

### 10.3 Generated Code Patterns to Translate

| Generated Pattern | Rust Translation |
|------------------|-----------------|
| `specifications.ts` (operation specs) | Static `Operation` array |
| `mappers.ts` (serialization) | `serde` derive macros + custom serializers |
| `models.ts` (API models) | Structs with `serde::Serialize/Deserialize` |
| `parameters.ts` (parameter specs) | Parameter metadata structs |
| `Context.ts` (request context) | Struct with typed fields |
| `MiddlewareFactory.ts` | Trait with factory methods |
| `ExpressMiddlewareFactory.ts` | axum middleware implementation |
| `dispatch.middleware.ts` | axum Router or manual dispatch |
| `deserializer.middleware.ts` | axum extractors or manual deserialization |
| `serializer.middleware.ts` | `IntoResponse` implementations |
| `IRequest.ts` / `IResponse.ts` | Traits wrapping axum types |
| `handlerMappers.ts` | Function dispatch table |

---

## 11. Persistence Layer Strategy

### 11.1 LokiJS Replacement

LokiJS is a JavaScript in-memory database with optional file persistence. There is no direct Rust equivalent.

**Decision:** Implement a custom in-memory store using `HashMap` + `Vec` + `tokio::sync::RwLock`.

**Rationale:**
- LokiJS is essentially a document store with find/insert/update/remove
- The Azurite code uses LokiJS through typed interfaces (IBlobMetadataStore, etc.)
- We implement the trait, not the database — the internal data structure doesn't need to match
- Use `serde_json::Value` for flexible document storage where needed

### 11.2 Persistence Interface Preservation

The key interfaces (`IBlobMetadataStore`, `IQueueMetadataStore`, `ITableMetadataStore`) define the contract. We implement these traits with in-memory data structures.

```rust
pub struct InMemoryBlobMetadataStore {
    containers: RwLock<HashMap<String, ContainerModel>>,
    blobs: RwLock<HashMap<String, BlobModel>>,
    initialized: AtomicBool,
    closed: AtomicBool,
}

#[async_trait]
impl BlobMetadataStore for InMemoryBlobMetadataStore {
    async fn create_container(&self, ...) -> Result<ContainerModel> { ... }
    // ...
}
```

### 11.3 Extent Store

Two implementations exist in TS:
- `MemoryExtentStore` → Implement with `HashMap<String, Vec<u8>>`
- `FSExtentStore` → Implement with `tokio::fs` operations

Both implement `IExtentStore` trait. Direct translation.

### 11.4 SQL Persistence (Phase 17)

`SqlBlobMetadataStore` uses Sequelize (ORM). Map to `sqlx` with raw SQL queries. Defer to Phase 17.

---

## 12. Authentication Strategy

### 12.1 Auth Pattern

Each service has an `IAuthenticator` interface with multiple implementations chained together:

| Authenticator | Blob | Queue | Table |
|--------------|------|-------|-------|
| SharedKey | ✓ | ✓ | ✓ |
| SharedKeyLite | — | — | ✓ |
| AccountSAS | ✓ | ✓ | ✓ |
| ServiceSAS | ✓ (BlobSAS) | ✓ (QueueSAS) | ✓ (TableSAS) |
| OAuth/Token | ✓ | ✓ | ✓ |
| PublicAccess | ✓ | — | — |

### 12.2 Rust Mapping

```rust
#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn validate(
        &self,
        request: &dyn Request,
        context: &mut StorageContext,
    ) -> Result<bool>;
}
```

HMAC-SHA256 signing: Use `hmac` + `sha2` crates (pure Rust, no OpenSSL dependency).

---

## 13. Middleware Chain Strategy

### 13.1 The 12-Step Pipeline

The TS middleware chain is implemented as Express middleware. In Rust, use **axum middleware** (Tower-based).

```
TS Express Pipeline          →    Rust axum Pipeline
─────────────────                 ─────────────────
1. morgan (access log)        →    tower-http::trace
2. Context middleware         →    axum::middleware::from_fn
3. Dispatch middleware        →    Custom dispatch (Router or from_fn)
4. Auth middleware            →    axum::middleware::from_fn
5. Deserializer middleware    →    axum extractors or from_fn
6. Handler middleware         →    axum handler functions
7-8. CORS middlewares         →    tower-http::cors
9. Serializer middleware      →    IntoResponse implementations
10. Options middleware        →    axum::middleware::from_fn
11. Error middleware          →    Error handler layer
12. End middleware            →    Response finalization
```

### 13.2 Middleware Composition

**Decision:** Keep the same ordering and explicit middleware chain rather than relying on axum's built-in routing. This preserves the 1:1 correspondence with the TS middleware pipeline.

---

## 14. Stream/IO Strategy

### 14.1 Node.js Streams → Rust Async IO

| TypeScript | Rust |
|-----------|------|
| `ReadableStream` | `Pin<Box<dyn AsyncRead + Send>>` |
| `WritableStream` | `Pin<Box<dyn AsyncWrite + Send>>` |
| `stream.pipe(dest)` | `tokio::io::copy()` |
| `new BufferStream(buf)` | `std::io::Cursor::new(buf)` |
| `ZeroBytesStream` | Custom `AsyncRead` impl yielding zeros |
| `multistream([s1, s2])` | `AsyncRead` chain via `tokio_util` |
| `stream.on("data/end/error")` | Loop with `AsyncReadExt::read()` |

### 14.2 Body Handling

Express request body is a readable stream. In axum, use `axum::body::Body` which implements `Stream<Item = Result<Bytes>>`.

---

## 15. Testing Strategy

### 15.1 Approach

Port the integration test suite from `tests/` using the Azure SDK Rust clients. The TS tests use `@azure/storage-blob`, `@azure/storage-queue`, and `@azure/data-tables` — the Rust equivalents exist in the Azure SDK for Rust.

### 15.2 Test Execution

- Unit tests: `#[tokio::test]` attribute
- Integration tests: `tests/` directory in each crate
- Same test structure as TS: `tests/blob/`, `tests/queue/`, `tests/table/`

---

## 16. Porting Database Format

### 16.1 Per-File Record Format

Each TypeScript source file gets a YAML record in `rust/porting-db/`. The path mirrors the source:

```
rust/porting-db/
├── STRATEGY.md          # This document
├── PORTING-ORDER.md     # File-level porting order
├── README.md            # Porting-db documentation
├── src/
│   ├── common/
│   │   ├── ServerBase.yaml
│   │   ├── ConfigurationBase.yaml
│   │   └── persistence/
│   │       ├── IExtentStore.yaml
│   │       └── ...
│   ├── blob/
│   │   ├── BlobServer.yaml
│   │   ├── handlers/
│   │   │   ├── ServiceHandler.yaml
│   │   │   └── ...
│   │   └── ...
│   ├── queue/ ...
│   └── table/ ...
```

### 16.2 Record Schema

```yaml
# rust/porting-db/src/blob/BlobServer.yaml
source:
  path: src/blob/BlobServer.ts
  lines: 245
  type: handwritten  # or "autorest-generated"
  
target:
  path: rust/crates/azurite-blob/src/blob_server.rs
  crate: azurite-blob
  module: blob_server

status: not_started  # not_started | in_progress | ported | verified
phase: 12  # From PORTING-ORDER.md

# TypeScript constructs in this file
constructs:
  classes:
    - name: BlobServer
      extends: ServerBase
      implements: [ICleaner]
      rust_strategy: struct_with_composition
      
  interfaces_used:
    - IBlobMetadataStore
    - IExtentMetadataStore
    - IExtentStore
    - IAccountDataStore
    - IGCManager
    
  enums: []
  
  async_methods:
    - beforeStart
    - afterStart
    - beforeClose
    - afterClose
    - clean

# Decisions made during porting
decisions:
  - description: "BlobServer uses composition (base: ServerBase) instead of inheritance"
    rationale: "Rust has no class inheritance; composition preserves the relationship"
  - description: "ICleaner mapped to Cleaner trait"
    rationale: "Direct interface-to-trait mapping"

# Dependencies this file needs
depends_on:
  - src/common/ServerBase.ts
  - src/common/ICleaner.ts
  - src/blob/persistence/IBlobMetadataStore.ts
  - src/common/persistence/IExtentMetadataStore.ts
  - src/common/persistence/IExtentStore.ts
  - src/common/IAccountDataStore.ts
  - src/common/IGCManager.ts
  - src/blob/BlobRequestListenerFactory.ts
  - src/blob/BlobConfiguration.ts

# Known challenges for this specific file
challenges:
  - "http.Server | https.Server union type in httpServer field"
  - "stoppable.WithStop intersection with server type"
  - "Certificate handling with Buffer I/O"

# Notes for change propagation
propagation_notes: |
  If BlobServer.ts changes, check:
  1. Constructor parameters (may add new stores/managers)
  2. Lifecycle hooks (beforeStart/afterStart order matters)
  3. Express middleware configuration delegation
```

### 16.3 Record Lifecycle

1. **Created** by Gandalf or Faramir during analysis (status: `not_started`)
2. **Updated** by Aragorn during implementation (status: `in_progress` → `ported`)
3. **Verified** by Faramir during review (status: `verified`)
4. **Consulted** during future TS change propagation

---

## 17. Decision Log

| # | Decision | Rationale | Date |
|---|----------|-----------|------|
| D-001 | Fidelity over idiom | User directive — foundational constraint | 2026-03-13 |
| D-002 | Rust code in `rust/` subdirectory | User directive | 2026-03-13 |
| D-003 | Porting database in `rust/porting-db/` | User directive | 2026-03-13 |
| D-004 | Tokio as async runtime | Required by axum; ecosystem standard | 2026-03-13 |
| D-005 | axum as HTTP framework (Express replacement) | Tower-based, async-native, strongest ecosystem | 2026-03-13 |
| D-006 | No autorest re-generation for Rust | No Rust server generator; manual translation preserves structure | 2026-03-13 |
| D-007 | Custom in-memory store (not LokiJS equivalent) | Implement trait, not database; HashMap+RwLock sufficient | 2026-03-13 |
| D-008 | tracing for logging (Winston replacement) | Structured logging, async-friendly, ecosystem standard | 2026-03-13 |
| D-009 | quick-xml for XML (xml2js replacement) | serde integration, fast, well-maintained | 2026-03-13 |
| D-010 | Flatten TS intersection types into single Rust structs | No Rust equivalent for type intersection; flattening preserves all fields | 2026-03-13 |
| D-011 | `Option<T>` for all TS optional/undefined/null | Uniform nullable treatment | 2026-03-13 |
| D-012 | `I` prefix preserved in Rust file names | 1:1 file correspondence for change propagation | 2026-03-13 |
| D-013 | VS Code extension (src/extension.ts, VSC*.ts) OUT OF SCOPE | VS Code API is JS-only; not portable to Rust | 2026-03-13 |
| D-014 | SQL persistence deferred to Phase 17 | LokiJS/in-memory is the primary target; SQL is secondary | 2026-03-13 |
| D-015 | Port blob service first, then queue, then table | Blob is most complex; establishes all patterns. Queue is simplest. Table has unique batch/EDM. | 2026-03-13 |
| D-016 | Composition over inheritance for Rust class mapping | Rust has no class inheritance; embed base struct as field | 2026-03-13 |

---

*This strategy is a living document. All changes must be reviewed by Gandalf and recorded in the decision log.*
