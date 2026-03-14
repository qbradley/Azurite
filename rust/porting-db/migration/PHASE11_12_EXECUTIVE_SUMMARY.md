# Azurite Blob Storage Phase 11 & 12 - Executive Summary for Faramir

**Date**: March 13, 2025  
**Scope**: TypeScript → Rust port for Blob Storage handlers and middleware  
**Status**: Analysis complete, ready for Rust implementation  

---

## OVERVIEW

**Phase 11** (~4,300 LOC): All blob operation handlers (create, read, update, delete, lease, batch)  
**Phase 12** (~1,200 LOC): Middleware, server assembly, configuration, environment setup  

Total blob service scope: ~5,500 LOC across 22 TypeScript files.

---

## PHASE 11: HANDLER LAYER (API Operations)

### Structure
- **BaseHandler**: Dependency injection base (metadataStore, extentStore, logger)
- **6 Main Handlers**: ServiceHandler, ContainerHandler, BlobHandler, BlockBlobHandler, PageBlobHandler, AppendBlobHandler
- **3 Batch Support**: BlobBatchHandler, BlobBatchSubRequest/SubResponse wrappers, SubResponseTextBodyStream
- **1 Range Manager**: PageBlobRangesManager (page blob extent management)

### Key Metrics
| File | LOC | Complexity | Status |
|------|-----|-----------|--------|
| BlobHandler.ts | 1350 | H | Core blob operations, range download, copy |
| ContainerHandler.ts | 865 | H | Container operations, batch submission |
| BlobBatchHandler.ts | 576 | H | Multipart parsing, pipeline execution |
| ServiceHandler.ts | 416 | H | Account-level operations |
| PageBlobRangesManager.ts | 481 | H | Page blob range merging algorithm |
| PageBlobHandler.ts | 495 | H | Page blob create/update/ranges |
| BlockBlobHandler.ts | 507 | H | Block blob staging/commit |
| AppendBlobHandler.ts | 263 | M | Append blob create/append |
| BaseHandler.ts | 30 | L | DI base |
| IPageBlobRangesManager.ts | 15 | L | Interface |
| Batch support (3 files) | 170 | M | Wrappers + streaming |

---

## PHASE 12: MIDDLEWARE & SERVER (Wire Everything)

### Structure
- **Constants & Utils**: Blob service constants, range/tag utilities
- **5 Middleware Factories**: Context extraction, authentication, CORS preflight, validation, telemetry
- **Config & Environment**: BlobConfiguration, BlobEnvironment (CLI parsing), IBlobEnvironment interface
- **Server & Factory**: BlobServer (HTTP/HTTPS, persistence init), BlobRequestListenerFactory (middleware composition), BlobServerFactory, main.ts entry point

### Key Metrics
| File | LOC | Purpose |
|------|-----|---------|
| BlobRequestListenerFactory.ts | 200 | Middleware assembly + handler instantiation |
| PreflightMiddlewareFactory.ts | 465 | CORS/OPTIONS handling |
| BlobServer.ts | 245 | HTTP server + persistence initialization |
| BlobEnvironment.ts | 100 | CLI argument parsing |
| blobStorageContext.middleware.ts | 80 | Context extraction (account/container/blob) |
| BlobConfiguration.ts | 50 | Configuration class |
| BlobRequestListenerFactory constants | 150 | Utilities, constants, auth factories |

---

## CRITICAL ARCHITECTURAL PATTERNS

### 1. **Inheritance → Composition (Rust Refactor)**
```
TypeScript:
class BlobHandler extends BaseHandler implements IBlobHandler {}

Rust:
struct BlobHandler {
  base_deps: Arc<BaseHandlerDeps>,
  ranges_manager: Arc<dyn IPageBlobRangesManager>,
}
impl IBlobHandler for BlobHandler { ... }
```

### 2. **Dependency Injection (DI) Container**
All handlers receive via constructor:
- `metadataStore: IBlobMetadataStore` (blob metadata, blocks, ranges, leases)
- `extentStore: IExtentStore` (blob data chunks)
- `extentMetadataStore: IExtentMetadataStore` (extent indexes)
- `accountDataStore: IAccountDataStore` (accounts, auth)
- `logger: ILogger` (tracing)

**Rust**: Use Arc<>-wrapped traits, factory pattern for construction.

### 3. **Dual-Store Pattern (CRITICAL)**
- **Metadata Store** (Phase 10.1): Persistent blob metadata
  - Properties (size, type, etag, timestamps)
  - Committed blocks (for block blobs)
  - Page ranges (for page blobs)
  - Leases (state, expiration)
  - Tags, snapshots, copy state
  
- **Extent Store** (Phase 4.5): Blob data chunks
  - Raw bytes stored separately
  - Referred to by extent ID + offset + count
  - Shared across snapshots/ranges

### 4. **Lease & Conditions Flow**
Every operation receives:
```typescript
leaseAccessConditions?: LeaseAccessConditions  // Lease ID, duration
modifiedAccessConditions?: ModifiedAccessConditions  // ETag, timestamps
```
→ Passed to metadataStore validation methods  
→ metadataStore validates lease state + modification times  
→ Operation proceeds or throws error

**Critical**: Snapshot cannot be leased (BlobHandler:378-382).

### 5. **Streaming Data**
- **Download**: `BlobHandler:download()` → `downloadBlockBlobOrAppendBlob()` → extent reading with range support
  - Parse range headers (bytes=0-1023)
  - Validate range bounds
  - For block blobs: compose multiple extents
  - For page blobs: read single extent
  - MD5 computation for <4MB downloads
  - Return 206 (Partial) or 200 (Full) + body stream
  
- **Upload**: Body stream → extentStore.appendExtent() → returns extent ID  
  
- **Batch**: Accumulate in 4MB buffer → parse → execute → serialize

### 6. **Multipart Batch Processing** (BlobBatchHandler)
Pipeline order CRITICAL:
```
1. Context extraction (account/container/blob from URL)
2. Operation dispatch (determine Delete vs SetTier)
3. Authentication (SharedKey/SAS/Token)
4. Deserialization (request body parsing)
5. Handler execution (call actual operation)
6. Serialization (format response)
7. End (finalization)
```

Parsing:
- Split by multipart boundary (from Content-Type)
- For each subrequest: parse HTTP request line + headers
- Validate all subrequests same operation
- Execute pipeline per subrequest
- Collect responses → serialize as multipart/mixed

**Constraint**: Currently only supports Delete and SetTier operations.

### 7. **PageBlobRangesManager Algorithm** (11.9) - SPLIT-FIRST STRATEGY
When merging new range [start, end]:
1. Find impacted existing ranges (binary search)
2. If no impact: splice insert new range
3. If impact: split first/last ranges to preserve non-overlapped portions
4. Insert new range between preserved portions
5. Splice old ranges, insert preserved + new

**Rationale**: Enables future garbage collection to merge small ranges. Full merge would prevent effective GC.

**Invariants**:
- Ranges non-overlapping, sorted by start position
- Each range has persistency ref (extent ID, offset, count)
- Byte offsets inclusive on both ends

### 8. **Middleware Order** (BlobRequestListenerFactory:12.11)
```
1. morgan (access logging, if enabled)
2. blobStorageContextMiddleware (extract context)
3. dispatchMiddleware (route to operation, auto-generated)
4. [Additional middleware]
5. Error handlers (last)
```

**CRITICAL**: Context middleware MUST run first; error handlers last.

### 9. **Handler Constructor Dependency Injection**
```typescript
// All handlers receive these:
BaseHandler(
  metadataStore: IBlobMetadataStore,
  extentStore: IExtentStore,
  logger: ILogger,
  loose: boolean  // Strict/loose API compliance
)

// Subclasses add specific deps:
BlobHandler(...base, rangesManager: IPageBlobRangesManager)
ContainerHandler(...base, accountDataStore, oauth)
ServiceHandler(...base, accountDataStore, oauth)
```

---

## HIGH-FIDELITY FOCUS AREAS FOR RUST PORT

### ⚠️ FIDELITY-CRITICAL (Must preserve exactly)

1. **Range Download Logic** (BlobHandler:1000-1095)
   - Range validation (start > length → error, shift to blob size)
   - Block blob extent composition
   - MD5 computation on read
   - Response status code (206 vs 200)

2. **PageBlobRangesManager Algorithm** (PageBlobRangesManager:51-115)
   - Split-first strategy (NOT merge-all)
   - Binary search for impact detection
   - Extent pointer validity

3. **Batch Multipart Parsing** (BlobBatchHandler:325-417)
   - Boundary detection (from Content-Type)
   - HTTP request line parsing
   - Content-ID extraction + validation
   - Operation determination
   - Subrequest homogeneity check (all same operation)

4. **Middleware Pipeline Order** (BlobBatchHandler:228-236)
   - Exact sequence: context → dispatch → auth → deserialize → handler → serialize → end
   - Cannot reorder

5. **Lease & Conditions** (Throughout)
   - Snapshot lease constraint (error)
   - ETag/timestamp validation
   - Lease expiration check

6. **Metadata Key Case Preservation** (convertRawHeadersToMetadata)
   - HTTP headers are case-insensitive but metadata keys must preserve case
   - Used in create/setMetadata operations

7. **setHTTPHeaders Redirect** (BlobHandler:245-266)
   - If sequenceNumberAction header present → redirect to updateSequenceNumber
   - This is a quirk workaround for Swagger issue

---

## IMPLEMENTATION ORDER RECOMMENDATION

### Phase 11 (Handlers)
1. **11.1** BaseHandler - Foundation
2. **11.8** IPageBlobRangesManager - Interface
3. **11.9** PageBlobRangesManager - **Core algorithm** (test thoroughly)
4. **11.5** BlockBlobHandler - Simple, no ranges
5. **11.7** AppendBlobHandler - Simple, no ranges
6. **11.6** PageBlobHandler - Uses ranges
7. **11.4** BlobHandler - **Large, complex** (many operations, ranges)
8. **11.2** ServiceHandler - Medium complexity
9. **11.3** ContainerHandler - High complexity (batch submission)
10. **11.11-11.13** Batch wrappers - Simple
11. **11.10** BlobBatchHandler - **Complex pipeline** (ties everything)

### Phase 12 (Middleware/Server)
1. **12.1-12.2** Constants, utils
2. **12.8-12.9** IBlobEnvironment, BlobEnvironment
3. **12.3-12.7** Middleware factories (5 files)
4. **12.10** BlobConfiguration
5. **12.11** BlobRequestListenerFactory - **Middleware composition**
6. **12.12** BlobServer - **Server initialization**
7. **12.13-12.14** Factory, main entry point

---

## KEY FILES FOR REFERENCE

Full analysis: `/home/azureuser/Azurite/PHASE11_12_SYNTHESIS.md` (38 KB)  
Key lines: `/home/azureuser/Azurite/PHASE11_12_KEY_REFERENCES.md` (10 KB)  

---

## STRUCTURAL DEPENDENCIES TO PRESERVE

```
┌──────────────────────────────────────────────────┐
│ HTTP Server (BlobServer:12.12, main.ts:12.14)    │
└────────────────┬─────────────────────────────────┘
                 │
         ┌───────┴─────────────┐
         │                     │
    ┌────▼─────────────┐    ┌──▼──────────────────┐
    │ BlobRequest      │    │ Persistence Stores  │
    │ ListenerFactory  │    │ (Phase 4, 10)       │
    │ (12.11)          │    │                     │
    └────┬─────────────┘    └─────────────────────┘
         │
    ┌────┴──────────────────────────────────┐
    │ Middleware Composition (12.3-12.7)    │
    └────┬──────────────────────────────────┘
         │
    ┌────┴──────────────────────────────────┐
    │ 6 Handlers (11.1-11.7)                │
    │ + Batch Handler (11.10)               │
    │ + PageBlobRangesManager (11.9)        │
    └────────────────────────────────────────┘
         │
    ┌────┴──────────────────────────────────┐
    │ metadataStore, extentStore, logger    │
    │ (Phase 6, 10, 4)                      │
    └──────────────────────────────────────┘
```

---

## TESTING PRIORITIES

1. **PageBlobRangesManager** - Unit test range merging (splits, no impact, partial overlap)
2. **BlobHandler.download()** - Integration test range requests, MD5, extent composition
3. **BlobBatchHandler** - Multipart parsing, pipeline execution, response serialization
4. **ContainerHandler.submitBatch()** - End-to-end batch workflow
5. **Middleware ordering** - Functional tests for context/auth/serialization
6. **Streaming** - Body stream reading, MD5 computation, error handling

---

## KNOWN ISSUES / TECHNICAL DEBT

1. **ContainerHandler:165-168** - TODO: Async cleanup of blobs on container delete
2. **ContainerHandler:274-278** - ACL XML formatting bug (generator issue)
3. **BlobBatchHandler** - Only supports Delete & SetTier (not all operations)
4. **BlobHandler.query()** - Throws NotImplemented (SQL query not supported)
5. **BlobHandler.undelete()** - Throws NotImplemented (soft delete not supported)
6. **BlobHandler.copyFromURL()** - Uses synchronous axios call (performance concern)

These are acceptable for Phase 11/12 scope but should be documented.

---

## NEXT STEPS FOR ARAGORN (Rust Port Lead)

1. **Deep Dive Review**
   - Read PHASE11_12_SYNTHESIS.md for full context
   - Review PHASE11_12_KEY_REFERENCES.md for exact file:line locations
   - Study PageBlobRangesManager algorithm (lines 51-115 in PageBlobRangesManager.ts)

2. **Architecture Design**
   - Map Rust trait-based DI pattern
   - Design Arc<>-wrapped dependency injection
   - Plan middleware pipeline as trait objects or function pointers
   - Streaming strategy (tokio::io traits)

3. **Parallel Implementation**
   - Handlers 1-7 can be implemented in parallel (6 blob types)
   - BlobBatchHandler depends on all handlers (implement last)
   - Phase 12 mostly independent (can start once Phase 11 core is stubbed)

4. **Testing**
   - PageBlobRangesManager: 50+ unit tests (splits, overlaps, GC scenarios)
   - BlobHandler: 30+ integration tests (all operations)
   - BlobBatchHandler: 20+ tests (parsing, pipeline, response)

---

## DOCUMENT LOCATIONS

- **Full Synthesis**: `/home/azureuser/Azurite/PHASE11_12_SYNTHESIS.md`
- **Key References**: `/home/azureuser/Azurite/PHASE11_12_KEY_REFERENCES.md`
- **TypeScript Source**: `/home/azureuser/Azurite/src/blob/handlers/`, `/home/azureuser/Azurite/src/blob/middlewares/`

---

*Ready for Rust implementation. All TypeScript source analyzed, dependencies mapped, critical patterns documented.*
