# Phase 2 Porting Database Records — Summary

This document indexes all Phase 2 porting analysis files created for the Azurite TypeScript-to-Rust migration. Each file contains detailed analysis of source code structure, dependencies, type mappings, recommended Rust translations, and change propagation notes.

## Files Analyzed (7 total)

### 1. OperationQueue.ts — Phase 2.1
- **Path**: `src/common/persistence/OperationQueue.ts` (124 lines)
- **Record**: `rust/porting-db/src/common/persistence/OperationQueue.md`
- **Key aspects**:
  - Implements concurrent operation queue with configurable max concurrency.
  - Uses Node.js EventEmitter for job completion signaling and queue continuation.
  - **Critical patterns**: Event-driven concurrency, lazy callbacks, promise resolution via events.
  - **Rust translation challenge**: Replace EventEmitter with tokio::sync::Semaphore or channel-based approach.
  - **Key consumers**: FSExtentStore (appendQueue, readQueue).

### 2. MemoryExtentStore.ts — Phase 2.2
- **Path**: `src/common/persistence/MemoryExtentStore.ts` (395 lines)
- **Record**: `rust/porting-db/src/common/persistence/MemoryExtentStore.md`
- **Key aspects**:
  - In-memory extent storage with global SharedChunkStore singleton.
  - Implements IExtentStore interface; delegates metadata management to IExtentMetadataStore.
  - **Critical patterns**: Mutable shared state (Map<Map>), size-limited chunking, stream composition (multistream), callback-based error constructor.
  - **Rust translation challenge**: Thread-safe Arc<RwLock<HashMap>> for chunks, replace multistream with futures combinator.
  - **Global singleton**: Use lazy_static! or once_cell for shared store.

### 3. FSExtentStore.ts — Phase 2.3
- **Path**: `src/common/persistence/FSExtentStore.ts` (677 lines, largest)
- **Record**: `rust/porting-db/src/common/persistence/FSExtentStore.md`
- **Key aspects**:
  - File system backed extent storage with configurable storage locations.
  - Pre-allocated pool of write extents (IAppendExtent) with shared mutable state.
  - Uses OperationQueue (twice: appendQueue, readQueue) for concurrency control.
  - **Critical patterns**: File descriptor caching, stream piping with explicit fdatasync, status flags (Idle/Appending), pool-based resource management.
  - **Rust translation challenge**: Manage File handle lifetime, map IAppendExtent to Arc<Mutex<AppendExtent>>, implement streamPipe with tokio::io and explicit sync calls.
  - **Control flow complexity**: Nested closures wrapped in queue operations; critical for hot path (read/write).

### 4. LokiExtentMetadataStore.ts — Phase 2.4
- **Path**: `src/common/persistence/LokiExtentMetadataStore.ts` (218 lines)
- **Record**: `rust/porting-db/src/common/persistence/LokiExtentMetadataStore.md`
- **Key aspects**:
  - Metadata store backed by Loki embedded database (in-memory or file-based).
  - Implements paginated listing with queryTime-based GC protection filtering.
  - **Critical patterns**: Callback-based Loki API, pagination via $loki internal ID, conditional document insertion/update.
  - **Rust translation challenge**: Replace Loki with sled (embedded KV store) or rocksdb. Adapt callback-based API to async/await.
  - **Query semantics**: Complex filter chains (id, lastModifiedInMS, marker); must preserve pagination guarantees.

### 5. AllExtentsAsyncIterator.ts — Phase 2.5
- **Path**: `src/common/persistence/AllExtentsAsyncIterator.ts` (46 lines)
- **Record**: `rust/porting-db/src/common/persistence/AllExtentsAsyncIterator.md`
- **Key aspects**:
  - Async iterator over all extent IDs, batched retrieval with pagination.
  - Captures snapshot time at construction; used by GC subsystem.
  - **Critical patterns**: AsyncIterator protocol, snapshot-time consistency, marker-based pagination.
  - **Rust translation challenge**: Implement Stream trait or custom async iterator; maintain Arc<dyn IExtentMetadataStore> reference.
  - **Key consumer**: LokiExtentMetadata.iteratorExtents(), blob/queue GC managers.

### 6. ZeroBytesStream.ts — Phase 2.6
- **Path**: `src/common/ZeroBytesStream.ts` (31 lines, smallest)
- **Record**: `rust/porting-db/src/common/ZeroBytesStream.md`
- **Key aspects**:
  - Readable stream that emits specified number of zero bytes; used for empty/sparse extents.
  - Extends Node.js Readable class; implements _read() callback.
  - **Critical patterns**: Stream pull-based demand, chunked emission, EOF signaling.
  - **Rust translation challenge**: Implement tokio::io::AsyncRead trait; write zeros directly to caller's buffer (avoid allocation).
  - **Optimization note**: Pre-allocated singleton zero buffer in TS; Rust should avoid allocation overhead.

### 7. Mutex.ts — Phase 2.7
- **Path**: `src/common/Mutex.ts` (76 lines)
- **Record**: `rust/porting-db/src/common/Mutex.md`
- **Key aspects**:
  - Static-only per-key mutex lock manager using event-driven callbacks.
  - Global state (keys map + listeners queue per key).
  - **Critical patterns**: FIFO waiter queue, setImmediate deferral, callback-based signaling.
  - **Rust translation challenge**: Replace EventEmitter with tokio::sync::Semaphore or oneshot channels; use lazy_static! for global state.
  - **No RAII guard**: TS requires explicit lock()/unlock() pairing; consider RAII alternative in Rust.

## Cross-File Dependencies

```
FSExtentStore (2.3)
  ├─> OperationQueue (2.1)
  ├─> MemoryExtentStore (2.2) [IExtentStore interface]
  ├─> ZeroBytesStream (2.6)
  └─> LokiExtentMetadataStore (2.4) [IExtentMetadataStore interface]

LokiExtentMetadataStore (2.4)
  ├─> AllExtentsAsyncIterator (2.5)
  └─> Mutex (2.7) [potential sync points]

MemoryExtentStore (2.2)
  ├─> ZeroBytesStream (2.6)
  └─> LokiExtentMetadataStore (2.4) [IExtentMetadataStore interface]
```

## Key Porting Challenges

### 1. Concurrency Model
- **TS**: Node.js single-threaded; event loop naturally serializes non-async operations.
- **Rust**: Tokio multi-threaded; must explicitly manage synchronization with Mutex/RwLock/Semaphore.
- **Impact**: OperationQueue, Mutex, and FSExtentStore's activeWriteExtents pool all require Arc<Mutex/RwLock<...>>.

### 2. Stream Abstractions
- **TS**: Node.js Readable/Writable (callback-based pull/push).
- **Rust**: tokio::io::AsyncRead/AsyncWrite (poll-based) or futures::stream::Stream.
- **Impact**: ZeroBytesStream, FSExtentStore's streamPipe, multistream composition all need adapters.

### 3. EventEmitter Replacement
- **TS**: Used in OperationQueue for job completion and queue continuation.
- **Rust**: Replace with:
  - tokio::sync::Semaphore for FIFO queueing.
  - tokio::sync::mpsc or channels for message-based pattern.
  - tokio::sync::oneshot for one-time signals (Mutex).
- **Impact**: Core concurrency pattern; affects OperationQueue and Mutex.

### 4. Global Mutable State
- **TS**: Direct static fields (MemoryExtentStore::SharedChunkStore, Mutex static keys/listeners).
- **Rust**: Use lazy_static! or once_cell; wrap with Arc<Mutex/RwLock> for thread-safety.
- **Impact**: Singleton initialization, cleanup, and shared access patterns.

### 5. Callback-based APIs
- **TS**: Callbacks everywhere (Loki loadDatabase, fs.stat, stream events, error constructor).
- **Rust**: Native async/await with tokio; promisify-like wrappers where needed.
- **Impact**: LokiExtentMetadataStore (replace Loki callbacks), FSExtentStore (stream events).

### 6. Pagination and Snapshot Semantics
- **TS**: AllExtentsAsyncIterator captures time snapshot; listExtents() uses this for consistent GC filtering.
- **Rust**: Must preserve snapshot time immutability; ensure pagination state is thread-safe.
- **Impact**: Iterator state machines, query parameter passing, GC subsystem behavior.

## Type Mapping Patterns

| TypeScript Pattern | Recommended Rust | Notes |
|---|---|---|
| `Map<K, V>` | `HashMap<K, V>` wrapped in Arc<RwLock> | Thread-safe shared state. |
| `EventEmitter` | tokio::sync::Semaphore + VecDeque | FIFO queueing + concurrency control. |
| `Promise<T>` | `impl Future<Output = Result<T, StorageError>>` | Result-based error handling. |
| `NodeJS.ReadableStream` | `impl tokio::io::AsyncRead + Send` | Pin-based polling. |
| `Writable` | `impl tokio::io::AsyncWrite + Send` | Async write sink. |
| `Callback` | `FnOnce() + Send + 'static` or oneshot channel | Deferred execution. |
| `static class` | `lazy_static!` + Arc<Mutex<...>> | Global shared state. |
| `(Buffer \| Stream)` | `enum ExtentDataInput { Bytes(..), Stream(..) }` | Union type preservation. |
| `Date` | `chrono::DateTime<Utc>` | Timestamp snapshots. |

## Recommended Rust Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"
uuid = { version = "1", features = ["v4"] }
bytes = "1"
chrono = "0.4"
serde_json = "1"
sled = "0.34"  # Loki replacement
lazy_static = "1.4"
parking_lot = "0.12"  # Optional, for lock-free variants
nix = "0.27"  # For fdatasync, fsync
sysinfo = "0.28"  # For os::total_memory equivalent
```

## Integration Notes

### Order of Porting
1. **Phase 2.1 (OperationQueue)**: Foundational; required by FSExtentStore.
2. **Phase 2.6 (ZeroBytesStream)**: Utility; no external dependencies except stream traits.
3. **Phase 2.7 (Mutex)**: Independent utility; consider early if needed by other modules.
4. **Phase 2.2 (MemoryExtentStore)**: Depends on IExtentStore interface and ZeroBytesStream.
5. **Phase 2.4 (LokiExtentMetadataStore)**: Depends on IExtentMetadataStore interface.
6. **Phase 2.5 (AllExtentsAsyncIterator)**: Depends on LokiExtentMetadataStore for delegation.
7. **Phase 2.3 (FSExtentStore)**: Depends on all others; hot-path complexity; port last.

### Shared Testing Strategy
- **Unit tests for OperationQueue**: Mock concurrency; verify FIFO queueing, max concurrency limits.
- **Integration tests for FSExtentStore**: Real file I/O, concurrent append/read/delete, error recovery.
- **Metadata store tests**: Pagination correctness, snapshot-time consistency, GC filtering.
- **Stream tests**: ZeroBytesStream EOF handling, MultiStream composition, error propagation.

### Documentation Requirements
- Preserve contextId semantics in logging (lowercase 'd' in persistence subsystem).
- Document callback contracts (Mutex unlock→lock re-entry, OperationQueue result handling).
- Update error codes and propagation strategy (TS Error → Rust StorageError/ResultError).
- Clarify maxConcurrency semantics: per-store instance? global? configurable?

---

**Generated**: 2025-03-13  
**Phase**: 2 (porting-db analysis)  
**Status**: All 7 files analyzed and documented
