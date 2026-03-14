# Azurite Blob Phase 11 & 12 Porting Synthesis for Faramir

## PHASE 11: BLOB HANDLERS (~12 files, ~4,300 LOC)

### Overview
Phase 11 implements all Azure Blob Storage API operations. Handlers are the core operation dispatch layer, each implements a generated interface (IBlobHandler, IContainerHandler, etc.) and calls into the persistence layer (IBlobMetadataStore, IExtentStore).

---

## Phase 11 Files

### 11.1 BaseHandler.ts (~30 LOC, L)
**Purpose**: Singleton persistence layer gateway for all handler subclasses.

**Key Members**:
- Protected fields: `metadataStore: IBlobMetadataStore`, `extentStore: IExtentStore`, `logger: ILogger`, `loose: boolean`

**Design Pattern**: Dependency injection via constructor; all handlers inherit from this base.

**Dependencies**: 
- `IExtentStore`, `IBlobMetadataStore`, `ILogger` (Phase 6, 10)

**Fidelity Notes**:
- Must maintain as pure DI base with no business logic
- The `loose` flag is used throughout to control strict API compliance
- Logger is injected for all tracing

---

### 11.2 ServiceHandler.ts (~416 LOC, H)
**Purpose**: Implements Azure Blob Service-level operations (account-wide).

**Major Methods**:
- `getServiceProperties()` - Service metadata (CORS, metrics, retention)
- `setServiceProperties()` - Configure service defaults
- `getAccountInfo()` / `getAccountInfoWithHead()` - Account metadata
- `listContainers()` / `listContainersSegment()` - Container enumeration
- `getUserDelegationKey()` - OAuth/SAS delegation
- `submitBatch()` - Batch operation dispatcher (delegates to BlobBatchHandler)

**Main Dependencies**:
- `BaseHandler` (11.1)
- `BlobBatchHandler` (11.10) - Called from submitBatch()
- `IAccountDataStore` (Phase 4.4) - For account metadata
- `OAuthLevel` - OAuth configuration

**Fidelity-Sensitive Quirks**:
- Default service properties hardcoded (~lines 57-70): CORS=[], metrics settings, versions
- `disableProductStyle` flag controls response formatting
- Delegation key parsing uses JWT decoding (jsonwebtoken library)
- listContainers uses cursor-based pagination with marker token

---

### 11.3 ContainerHandler.ts (~865 LOC, H)
**Purpose**: Azure Blob Container-level operations (create, properties, metadata, ACL, batch submission).

**Major Methods**:
- `create()` - Create container with access level
- `getProperties()` / `getPropertiesWithHead()` - Container metadata
- `delete()` - Delete container and all blobs
- `setMetadata()` / `getAccessPolicy()` / `setAccessPolicy()` - Container metadata/ACL management
- `acquireLease()` / `releaseLease()` / `renewLease()` / `breakLease()` / `changeLease()` - Container-level leases
- `listBlobFlatSegment()` / `listBlobHierarchySegment()` - Blob enumeration (flat/hierarchical)
- `filterBlobs()` - Tag-based filtering
- `submitBatch()` - Batch operation dispatcher

**Main Dependencies**:
- `BaseHandler` (11.1)
- `IAccountDataStore` (Phase 4.4) - OAuth context
- `OAuthLevel` - OAuth configuration
- `BlobBatchHandler` (11.10) - Batch processing
- Metadata preservation from raw HTTP headers (line 66-67, 206-209)

**Fidelity-Sensitive Quirks**:
- Metadata key case preservation via `convertRawHeadersToMetadata()` (lines 66-67, 206-209)
- Lease operations integrate with metadataStore lease validators
- Container delete is synchronous but TODO notes async cleanup needed (line 165-168)
- ACL serialization has known XML formatting issues (lines 274-278) - generator bug
- `disableProductStyle` affects response headers
- Batch submission creates new BlobBatchHandler instance inline (line 343-344)

---

### 11.4 BlobHandler.ts (~1350 LOC, H)  **[CRITICAL - DEEP DIVE REQUIRED]**
**Purpose**: Generic blob operations (works for all 3 blob types). Download, properties, metadata, leases, copy, tagging.

**Major Methods** (~27 async operations):
- `download()` - Blob data retrieval with range support
- `getProperties()` - Blob metadata/properties
- `setHTTPHeaders()` / `setMetadata()` - Header/metadata updates
- `delete()` / `undelete()` - Deletion (undelete throws NotImplemented)
- `acquireLease()` / `releaseLease()` / `renewLease()` / `breakLease()` / `changeLease()` - Blob leases
- `createSnapshot()` - Snapshot creation
- `startCopyFromURL()` / `abortCopyFromURL()` / `copyFromURL()` - Copy operations
- `setTier()` - Archive/Hot/Cool tier management
- `getTags()` / `setTags()` - Tag operations
- `query()` - SQL query support (throws NotImplemented)
- Private: `downloadBlockBlobOrAppendBlob()`, `downloadPageBlob()`, `validateCopySource()`

**Key Dependencies**:
- `BaseHandler` (11.1)
- `IPageBlobRangesManager` (11.8, 11.9) - Injected in constructor for range management
- `BlobStorageContext` - Request context parsing

**Streaming/Body Handling** (Critical for Rust):
- **download()** with ranges: Lines 998-1095
  - Parses range headers via `deserializeRangeHeader()` (line 1010-1012)
  - For block blobs: reads multiple extents via `extentStore.readExtents()` with block list (line 1067-1073)
  - For page blobs: reads single extent with offset/count (line 1056-1063)
  - MD5 computation for small downloads (<4MB): lines 1084-1094
  - Returns stream via `Models.BlobDownloadResponse.body`

**Lease & Conditions Integration** (Critical):
- Lines 79-80, 120-121, 264-265, 342-344: Passes `leaseAccessConditions` and `modifiedAccessConditions` to metadataStore
- Metadata store validates lease state and modification times
- Copy operations check snapshot constraints (line 378-382)

**Fidelity-Sensitive Quirks**:
- Archive tier access throws error (line 83-85)
- Metadata key case preservation (line 333-335)
- setHTTPHeaders workaround for sequence number action redirect (lines 245-266) - maps to updateSequenceNumber
- Snapshot constraints prevent lease acquisition (lines 378-382)
- Copy source validation via axios (lines 701+) - fetches real URL to validate
- URI parsing uses @azure/ms-rest-js URLBuilder (line 1)
- No implementation for undelete, setExpiry, setImmutabilityPolicy, deleteImmutabilityPolicy, setLegalHold, query

---

### 11.5 BlockBlobHandler.ts (~507 LOC, H)
**Purpose**: Block blob specific operations (upload, put block, put block list).

**Major Methods**:
- `upload()` - Direct blob upload
- `uploadFromUrl()` - Upload from external URL
- `putBlock()` - Stage block for block blob
- `putBlockList()` - Commit staged blocks as blob
- `getBlockList()` - List staged/committed blocks

**Key Dependencies**:
- `BaseHandler` (11.1)
- `IExtentStore` - Append blob data to extents

**Streaming/Body Handling**:
- `upload()`: Accepts body stream, computes MD5, stores via extentStore.appendExtent()
- `putBlock()`: Stages block with ID, stores separately
- `putBlockList()`: Commits staged blocks in specified order

**Fidelity-Sensitive Quirks**:
- Content-Type defaults to "application/octet-stream"
- MD5 validation from headers: content-md5 or x-ms-blob-content-md5 (lines 48-52)
- Tags parsing via `getTagsFromString()`

---

### 11.6 PageBlobHandler.ts (~495 LOC, H)
**Purpose**: Page blob specific operations (create, upload pages, clear pages, resize, page ranges).

**Major Methods**:
- `create()` - Create empty page blob with size
- `uploadPages()` - Write data to page range
- `clearPages()` - Zero out page range
- `resize()` - Expand/shrink blob
- `getPageRanges()` / `getPageRangesDiff()` - Page range queries
- `uploadPagesFromURL()` - Copy pages from URL (NotImplemented)

**Key Dependencies**:
- `BaseHandler` (11.1)
- `IPageBlobRangesManager` (11.8, 11.9) - Injected, manages range merging
- `BlobLeaseAdapter` / `BlobWriteLeaseValidator` - Lease validation

**Fidelity-Sensitive Quirks**:
- Pages must be 512-byte aligned
- Size must be multiple of 512 bytes
- Range merging delegates to IPageBlobRangesManager (uses split-first strategy, see 11.9)
- Lease validation via BlobWriteLeaseValidator for write operations

---

### 11.7 AppendBlobHandler.ts (~263 LOC, M)
**Purpose**: Append blob specific operations (create, append, seal).

**Major Methods**:
- `create()` - Create empty append blob
- `appendBlock()` - Append data block
- `seal()` - Prevent further appends

**Key Dependencies**:
- `BaseHandler` (11.1)

**Fidelity-Sensitive Quirks**:
- Create requires contentLength=0 in strict mode (line 33-38)
- Max block count: MAX_APPEND_BLOB_BLOCK_COUNT (constants)
- Max block size: MAX_APPEND_BLOB_BLOCK_SIZE (constants)
- Tracks committed block count incrementally

---

### 11.8 IPageBlobRangesManager.ts (~15 LOC, L)
**Purpose**: Interface defining page range management operations.

**Key Methods**:
- `mergeRange()` - Insert/merge new range into ranges array
- `clearRange()` - Remove range from ranges array (zero-fill)
- `cutRanges()` - Extract sub-ranges
- `fillZeroRanges()` - Identify zero-filled ranges

**Design Pattern**: Pure interface, allows pluggable range management strategies.

---

### 11.9 PageBlobRangesManager.ts (~481 LOC, H)  **[CRITICAL - DEEP DIVE REQUIRED]**
**Purpose**: Core algorithm for merging/managing page blob ranges (extents).

**Major Methods**:
- `mergeRange()` - Split-first strategy: splits first/last impacted ranges, inserts new range in middle
- `clearRange()` - Similar split logic for clearing
- `cutRanges()` - Extract ranges within bounds
- `fillZeroRanges()` - Identify zero ranges
- `selectImpactedRanges()` - Binary search for affected ranges
- `locateFirstImpactedRange()` / `locateLastImpactedRange()` - Range boundary finding
- `positionInRange()` - Check if byte position is in range

**Architectural Pattern** (Critical):
- Uses split-first strategy (NOT merge-all) to preserve GC opportunities (lines 8-49)
- Rationale: Future GC can merge small ranges; full merge prevents effective GC
- Each page range tracks persistency: `{id, offset, count}` (pointer to extent storage)
- Ranges are stored as sorted array in memory (blob.pageRanges)

**Data Structures**:
```typescript
PersistencyPageRange: {
  start: number,        // Inclusive byte position
  end: number,          // Inclusive byte position
  persistency: {
    id: string,         // Extent ID
    offset: number,     // Offset within extent
    count: number       // Byte count
  }
}
```

**Algorithm Deep Dive**:
- `selectImpactedRanges()` (lines 328-362): Binary search returns [firstIndex, lastIndex] of ranges overlapping [start, end]
- `mergeRange()` (lines 51-115):
  1. Find impacted ranges
  2. If no impact, splice insert new range
  3. If impact:
     - If first range extends before start: keep prefix (lines 84-93)
     - Insert new range
     - If last range extends after end: keep suffix (lines 99-110)
     - Splice all impacted ranges, insert preserved pieces + new range

**Fidelity-Sensitive Quirks**:
- Assumes all ranges are non-overlapping in input (precondition)
- Persistency references must be valid extent IDs
- No validation that extents actually exist
- Range boundaries are inclusive on both ends (start, end)
- Byte counts must match (end - start + 1)

---

### 11.10 BlobBatchHandler.ts (~576 LOC, H)  **[CRITICAL - DEEP DIVE REQUIRED]**
**Purpose**: Batch operation parsing and execution (multipart/mixed message handling).

**Major Methods**:
- `submitBatch()` - Parse multipart body, execute subrequests, serialize responses
- `parseSubRequests()` - Parse multipart/mixed boundary-delimited requests (lines 325-417)
- `serializeSubResponse()` - Format multipart/mixed response (lines 420-449)
- `requestBodyToString()` - Stream → string conversion with 4MB buffer
- `getSubRequestOperation()` - Determine operation type for subrequest
- `HandleOneSubRequest()` - Execute single subrequest through middleware pipeline
- `HandleOneFailedRequest()` - Error handling for failed batch

**Middleware Pipeline Architecture** (Critical):
```
handlePipeline = [
  subRequestContextMiddleware,           // Extract account/container/blob from URL
  subRequestDispatchMiddleware,          // Dispatch to operation
  subRequestAuthenticationMiddleware,    // AuthN (multiple authenticators)
  subRequestDeserializeMiddleware,       // Deserialize body/headers
  subRequestHandlerMiddleware,           // Call handler (via HandlerMiddlewareFactory)
  subRequestSerializeMiddleWare,         // Serialize response
  subRequestEndMiddleWare                // Finalize
]
```

**Constructor Setup** (lines 53-244):
- Instantiates all 6 blob handlers inline (AppendBlobHandler, BlobHandler, BlockBlobHandler, ContainerHandler, PageBlobHandler, ServiceHandler)
- Passes PageBlobRangesManager instance to BlobHandler and PageBlobHandler (lines 154, 175)
- Creates authenticator instances: PublicAccess, SharedKey, AccountSAS, BlobSAS, [optional] Token
- Builds middleware pipeline for each subrequest
- Separate operationFinder pipeline (2 stages) to detect operation type without auth/execution

**Multipart Parsing** (lines 325-417):
- Splits body by batch boundary: `--${boundary}` → per-request prefix
- Splits by per-request prefix to extract individual requests
- For each request:
  1. Parse headers (Content-ID, Content-Type)
  2. Parse HTTP request line (METHOD /path HTTP/1.1)
  3. Validate path starts with container prefix
  4. Parse request headers
  5. Determine operation via `getSubRequestOperation()`
  6. Validate all subrequests are same operation (line 394-407)
  7. Only supports Blob_Delete and Blob_SetTier (line 394)

**Response Serialization** (lines 420-449):
- Per subrequest:
  1. Write boundary
  2. Write Content-Type: application/http
  3. Write Content-ID if present
  4. Write blank line
  5. Write HTTP response (protocol, status, message)
  6. Write response headers
  7. If body present: blank line, body, blank line
- Final boundary with -- suffix

**Streaming/Body Handling**:
- `requestBodyToString()` (lines 281-293): Reads stream into 4MB Buffer
- Converts Buffer to string for parsing
- Error if stream exceeds 4MB (line 265)

**Error Handling**:
- Per-request parsing errors caught (line 465-488)
- If any error: return single error response
- If >256 subrequests: return error (line 491-497)
- Individual subrequest errors handled via error middleware (line 208-216)

**Fidelity-Sensitive Quirks**:
- Only supports Delete and SetTier operations in current implementation (line 394)
- Validates all subrequests use same operation (line 401-407)
- Content-ID must be present in each subrequest (line 361-365)
- Batch boundary extracted from main request Content-Type header (ContainerHandler:341)
- Subrequest context uses DEFAULT_CONTEXT_PATH constant
- Authenticators checked in order: PublicAccess → SharedKey → AccountSAS → BlobSAS → [Token if OAuth enabled]

---

### 11.11 BlobBatchSubRequest.ts (~80 LOC, M)
**Purpose**: Wrapper implementing IRequest interface for multipart subrequests.

**Key Members**:
- `content_id: number` - Batch subrequest ID
- `url: string`, `method: HttpMethod`, `protocolWithVersion: string`
- `headers: {[header: string]: string | string[] | undefined}`

**Methods**:
- `getMethod()`, `getUrl()`, `getEndpoint()`, `getPath()` - URL parsing
- `getHeader()`, `setHeader()` - Header management
- `getBodyStream()` → empty Readable (no body in batch delete/settier)
- `getBody()` → undefined

**Design Pattern**: Adapter pattern - wraps string data into IRequest contract.

---

### 11.12 BlobBatchSubResponse.ts (~50 LOC, L)
**Purpose**: Wrapper implementing IResponse interface for multipart subrequests.

**Key Members**:
- `content_id: number | undefined`
- `statusCode`, `statusMessage`, `headers`, `bodyStream: SubResponseTextBodyStream`

**Methods**:
- `setStatusCode()`, `getStatusCode()`
- `setStatusMessage()`, `getStatusMessage()`
- `setHeader()`, `getHeader()`, `getHeaders()`
- `getBodyStream()` → SubResponseTextBodyStream instance
- `end()` - Finalize response

**Design Pattern**: Adapter pattern - wraps response data into IResponse contract.

---

### 11.13 SubResponseTextBodyStream.ts (~40 LOC, L)
**Purpose**: Writable stream collecting batch subrequest response body.

**Key Members**:
- `bodyText: string` - Accumulated response body
- Extends Node.js `Writable`

**Methods**:
- `_write()` - Accumulate chunks into string (line 13-15)
- `end()` - Override to handle final chunk (lines 18-23)
- `getBodyContent()` - Return accumulated body (line 26-28)

**Design Pattern**: Stream adaptor - collects text output for serialization.

---

## PHASE 12: MIDDLEWARE, SERVER, CONFIG, ENVIRONMENT (~10 files, ~1,200 LOC)

### Overview
Phase 12 wires handlers together via middleware, creates the HTTP server, and manages configuration/environment.

---

## Phase 12 Files

### 12.1 src/blob/utils/constants.ts (~50 LOC, L)
**Purpose**: Blob service constants and configuration values.

**Key Constants**:
- `BLOB_API_VERSION` - Service version string
- `HeaderConstants.*` - HTTP header names (x-ms-*, etc.)
- `MethodConstants.*` - HTTP method names
- `DEFAULT_CONTEXT_PATH` - Express locals key
- `DEFAULT_LIST_BLOBS_MAX_RESULTS` - Pagination default
- `DEFAULT_LIST_CONTAINERS_MAX_RESULTS` - Pagination default
- `HTTP_HEADER_DELIMITER`, `HTTP_LINE_ENDING` - Batch parsing
- `EMULATOR_ACCOUNT_KIND`, `EMULATOR_ACCOUNT_SKUNAME` - Account defaults
- Port/host defaults

**Fidelity Notes**:
- Many of these are referenced in Phase 11 handlers
- Used for header/footer serialization in batch operations

---

### 12.2 src/blob/utils/utils.ts (~100 LOC, M)
**Purpose**: Utility functions for ranges, tags, headers.

**Key Functions**:
- `deserializeRangeHeader()` - Parse range bytes header
- `deserializePageBlobRangeHeader()` - Page blob range parsing
- `getBlobTagsCount()` - Count blob tags
- `validateBlobTag()` - Validate tag format
- `getTagsFromString()` - Parse tag string
- `removeQuotationFromListBlobEtag()` - ETag formatting
- `getUserDelegationKeyValue()` - Extract delegation key

**Dependencies**: None, pure utility functions.

---

### 12.3 src/blob/middlewares/blobStorageContext.middleware.ts (~80 LOC, M)  **[CRITICAL]**
**Purpose**: Extract blob service context from HTTP request (account, container, blob names, API version).

**Key Functions**:
- `createStorageBlobContextMiddleware()` - Express middleware factory
- `internalBlobStorageContextMiddleware()` - Core context extraction
- Populates `BlobStorageContext` attached to request

**Context Extracted** (lines 44-54):
- Server header set to `Azurite-Blob/${VERSION}`
- Generates request UUID
- Validates API version (if not skipApiVersionCheck)
- Extracts account/container/blob from URL (two styles: product-style via host, path-style via first segment)
- Validates container name
- Sets context into res.locals

**Key Dependencies**:
- `BlobStorageContext` (Phase 6.5)
- API version validation via `ValidAPIVersions` and `checkApiVersion()`

**Fidelity-Sensitive Quirks**:
- Product-style URL: account from hostname (line ~80+)
- Path-style URL: account from first path segment
- `disableProductStyleUrl` flag switches between modes
- `loose` mode allows invalid container names for testing
- NoAccountHostNames list for special cases (localhost, 127.0.0.1)

---

### 12.4 src/blob/middlewares/AuthenticationMiddlewareFactory.ts (~100 LOC, M)
**Purpose**: Authentication middleware factory wiring multiple authenticators.

**Key Methods**:
- `authenticate()` - Apply authenticators in order until one passes

**Design Pattern**:
- Factory creates middleware that chains authenticators
- Authenticators checked in order: PublicAccess → SharedKey → AccountSAS → BlobSAS → [Token if OAuth]
- First passing authenticator wins
- Failure returns false

**Dependencies**:
- Phase 7 authenticators (BlobSharedKeyAuthenticator, BlobSASAuthenticator, BlobTokenAuthenticator, AccountSASAuthenticator, PublicAccessAuthenticator)
- ILogger

---

### 12.5 src/blob/middlewares/PreflightMiddlewareFactory.ts (~465 LOC, H)
**Purpose**: CORS preflight (OPTIONS) request handling and model validation.

**Key Methods**:
- `createOptionsHandlerMiddleware()` - Handle OPTIONS requests
- CORS header validation
- Blob metadata and specification validation

**Design Pattern**:
- Error handler middleware (ErrorRequestHandler signature)
- Validates CORS Origin header
- Returns CORS response headers

**Dependencies**:
- `BlobStorageContext`, `StorageErrorFactory`
- Generated Specifications and Mappers (Phase 5)

---

### 12.6 src/blob/middlewares/StrictModelMiddlewareFactory.ts (~80 LOC, M)
**Purpose**: Strict mode validation - blocks unsupported headers/parameters.

**Key Exports**:
- `UnsupportedHeadersBlocker` - Rejects unsupported headers
- `UnsupportedParametersBlocker` - Rejects unsupported query params

**Design Pattern**:
- Blockers are middleware functions composed into middleware factory

**Dependencies**:
- Unsupported feature lists (probably auto-generated)

---

### 12.7 src/blob/middlewares/telemetry.middleware.ts (~50 LOC, L)
**Purpose**: Telemetry data collection middleware.

**Design Pattern**:
- Thin wrapper around telemetry library
- Hooks request/response events

---

### 12.8 src/blob/IBlobEnvironment.ts (~50 LOC, L)
**Purpose**: Interface defining blob service environment configuration.

**Key Properties**:
- `blobHost()`, `blobPort()`, `blobKeepAliveTimeout()`
- `location()`, `silent()`, `loose()`, `skipApiVersionCheck()`
- `cert()`, `key()`, `pwd()` - HTTPS/mTLS config
- `debug()`, `oauth()`, `disableProductStyleUrl()`
- `inMemoryPersistence()`, `extentMemoryLimit()`
- `disableTelemetry()`

**Design Pattern**: Pure interface, allows multiple implementations (e.g., arg-based, config-file-based).

---

### 12.9 src/blob/BlobEnvironment.ts (~100 LOC, M)
**Purpose**: Default implementation of IBlobEnvironment - parses CLI arguments.

**Architecture**:
- Uses `args` library to parse process.argv
- Options defined inline with defaults
- Lazy evaluation via methods

**Key Features**:
- Host/port override
- Loose mode (ignores unsupported headers)
- Skip API version check
- OAuth support
- Cert/key for HTTPS
- In-memory persistence with extent memory limit
- Debug logging path

**Dependencies**:
- `args` library for CLI parsing
- IBlobEnvironment interface

---

### 12.10 src/blob/BlobConfiguration.ts (~50 LOC, L)  **[CRITICAL]**
**Purpose**: Configuration class for blob server initialization.

**Key Properties**:
- From ConfigurationBase: host, port, keepAliveTimeout, enableAccessLog, enableDebugLog, loose, skipApiVersionCheck, cert, key, pwd, oauth, disableProductStyleUrl
- Blob-specific: metadataDBPath, extentDBPath, persistencePathArray, isMemoryPersistence, memoryStore

**Design Pattern**: Extends ConfigurationBase (Phase 4.7).

**Constructor Parameters** (lines 28-48):
- Database paths for LokiJS (metadata store, extent metadata store)
- Persistence array (file/memory storage destinations)
- Memory extent store config (for in-memory persistence)

**Fidelity Notes**:
- MetadataDBPath defaults to DEFAULT_BLOB_LOKI_DB_PATH
- ExtentDBPath defaults to DEFAULT_BLOB_EXTENT_LOKI_DB_PATH
- PersistencePathArray configures where extent data goes
- IsMemoryPersistence flag: true = skip disk, use SharedChunkStore
- Must coordinate with BlobServer initialization (Phase 12.12)

---

### 12.11 src/blob/BlobRequestListenerFactory.ts (~200 LOC, H)  **[CRITICAL - MIDDLEWARE INTEGRATION]**
**Purpose**: Express app factory - assembles all middleware and handlers.

**Constructor Parameters** (lines 50-59):
- metadataStore, extentStore, accountDataStore (persistence layer - Phase 6, 10, 4.4)
- enableAccessLog, accessLogWriteStream
- loose, skipApiVersionCheck, disableProductStyleUrl
- oauth

**`createRequestListener()`** (lines 62-150+):
1. Create Express app with `x-powered-by` disabled
2. Create ExpressMiddlewareFactory (auto-generated middleware wrapper)
3. Create PageBlobRangesManager singleton (shared between BlobHandler, PageBlobHandler)
4. Instantiate all 6 blob handlers with persistence stores (lines 75-120)
5. Create middleware factories:
   - PreflightMiddlewareFactory (CORS)
   - TelemetryMiddlewareFactory
   - StrictModelMiddlewareFactory
6. Register middleware in **strict order** (lines 140-150+):
   - morgan (access logging) if enabled
   - blobStorageContextMiddleware (context extraction)
   - dispatchMiddleware (auto-generated, routes to operation)
   - [other middleware]
   - error handlers

**Middleware Composition** (lines 135-150):
```
1. morgan (if enabled) - Access log
2. createStorageBlobContextMiddleware() - Extract context (Phase 12.3)
3. createDispatchMiddleware() - Route to operation (generated)
4. [Authentication middleware] (Phase 12.4)
5. [Preflight/Options handler] (Phase 12.5)
6. [Strict model middleware] (Phase 12.6) - if strict mode
7. [Other middleware]
8. [Error handlers]
```

**Critical Architectural Pattern**:
- PageBlobRangesManager created once, shared between page blob handlers (line 72)
- Same instance ensures consistent range state across requests
- All handlers instantiated fresh on each request listener creation (not truly singleton, created per app)

**Key Dependencies**:
- Phase 5: ExpressMiddlewareFactory, auto-generated MiddlewareFactory
- Phase 11: All handler classes (BaseHandler, ServiceHandler, ContainerHandler, BlobHandler, BlockBlobHandler, PageBlobHandler, AppendBlobHandler, PageBlobRangesManager, BlobBatchHandler)
- Phase 12: All middleware factories
- IAuthenticator implementations (Phase 7)

---

### 12.12 src/blob/BlobServer.ts (~245 LOC, H)  **[CRITICAL - SERVER ASSEMBLY]**
**Purpose**: HTTP(S) server initialization and lifecycle management.

**Key Members** (lines 42-46):
- `metadataStore: IBlobMetadataStore` (Phase 10.1)
- `extentMetadataStore: IExtentMetadataStore` (Phase 4.6)
- `extentStore: IExtentStore` (Phase 4.5)
- `accountDataStore: IAccountDataStore` (Phase 4.4)
- `gcManager: IGCManager` - Garbage collection manager

**Constructor** (lines 54-???):
- Takes optional BlobConfiguration (Phase 12.10)
- If no config provided, creates default BlobConfiguration()
- Initializes HTTP(S) server based on config
- Sets up persistence stores (LokiJS or in-memory)
- Sets up request listener factory (Phase 12.11)
- Starts garbage collection manager

**Extends ServerBase**:
- Abstract HTTP server implementation from common layer
- Handles server lifecycle (start, close, status)
- Implements ICleaner interface

**Persistence Initialization**:
- If isMemoryPersistence: create MemoryExtentStore with memoryStore
- Else: create FSExtentStore with persistencePathArray
- Create LokiBlobMetadataStore or LokiExtentMetadataStore as needed
- Create AccountDataStore

**Garbage Collection**:
- BlobGCManager handles cleanup of expired data
- Runs asynchronously
- Must handle errors during close (line 25-26)

**Key Dependencies**:
- Phase 4.4: IAccountDataStore, AccountDataStore
- Phase 4.5: IExtentStore, FSExtentStore, MemoryExtentStore
- Phase 4.6: IExtentMetadataStore, LokiExtentMetadataStore
- Phase 10.1: IBlobMetadataStore, LokiBlobMetadataStore
- Phase 12.10: BlobConfiguration
- Phase 12.11: BlobRequestListenerFactory
- Blob GC: BlobGCManager

---

### 12.13 src/blob/BlobServerFactory.ts (~60 LOC, M)
**Purpose**: Factory for creating BlobServer instances.

**Key Method**:
- `create()` - Instantiate BlobServer with configuration

**Design Pattern**: Simple factory, wraps server instantiation logic.

---

### 12.14 src/blob/main.ts (~80 LOC, M)
**Purpose**: Entry point for azurite-blob CLI application.

**Key Functions**:
- Parses CLI arguments via BlobEnvironment (Phase 12.9)
- Creates BlobConfiguration from environment
- Instantiates BlobServer via factory
- Starts server
- Registers graceful shutdown handlers

**Lifecycle**:
1. BlobEnvironment.parse() from process.argv
2. BlobConfiguration construction with environment values
3. BlobServer creation
4. server.start()
5. Process signal handlers (SIGINT, SIGTERM) → server.close()

**Dependencies**:
- Phase 12.8: IBlobEnvironment
- Phase 12.9: BlobEnvironment
- Phase 12.10: BlobConfiguration
- Phase 12.12: BlobServer
- Phase 12.13: BlobServerFactory

---

## CROSS-FILE ARCHITECTURAL PATTERNS FOR RUST

### 1. **Inheritance/Composition (Handler Hierarchy)**

**TypeScript Pattern**:
```typescript
class BaseHandler {
  protected metadataStore, extentStore, logger, loose
}
class BlobHandler extends BaseHandler implements IBlobHandler { }
class ServiceHandler extends BaseHandler implements IServiceHandler { }
class ContainerHandler extends BaseHandler implements IContainerHandler { }
```

**Rust Equivalent**:
- Use composition over inheritance: each handler struct contains BaseHandlerDeps
- Trait implementations for IBlobHandler, IServiceHandler, etc.
- Shared dependency struct:
  ```rust
  struct BaseHandlerDeps {
    metadata_store: Arc<IBlobMetadataStore>,
    extent_store: Arc<IExtentStore>,
    logger: Arc<ILogger>,
    loose: bool,
  }
  
  struct BlobHandler {
    base: BaseHandlerDeps,
    ranges_manager: Arc<IPageBlobRangesManager>,
  }
  ```

---

### 2. **Generated Middleware Interactions**

**TypeScript Flow**:
- Auto-generated middleware from Swagger (Phase 5): dispatch, serializer, deserializer, handler
- BlobRequestListenerFactory composes these into Express app
- BlobBatchHandler recreates pipeline for each subrequest

**Rust Equivalent**:
- Port generated middleware as functions/traits
- Middleware pipeline as array of function pointers or trait objects
- Context flow via locals/request/response objects
- Same pattern in BlobBatchHandler (build pipeline, execute sequentially)

---

### 3. **Persistence Dependencies (Dual-Store Pattern)**

**Core Pattern**:
```
┌─────────────────────────────────────┐
│  Handler Layer (11.*)               │
│  - Calls metadataStore for metadata │
│  - Calls extentStore for blob data  │
└──────────┬──────────────────────────┘
           │
     ┌─────┴────────┬──────────────────┐
     │              │                  │
     ▼              ▼                  ▼
MetadataStore  ExtentStore    ExtentMetadataStore
(LokiBlobMS)   (FSExtentS)    (LokiExtentMS)
(Phase 10.1)   (Phase 4.5)    (Phase 4.6)
```

**Fidelity Requirement**: Must preserve dual-store pattern:
- metadataStore: holds blob metadata (properties, tags, blocks, ranges)
- extentStore: holds actual blob data chunks
- Extent pointers stored in metadata, resolved at read time

---

### 4. **Lease & Conditions Integration**

**Pattern**:
- Every operation receives `leaseAccessConditions` and `modifiedAccessConditions` options
- Passed directly to metadataStore methods
- metadataStore validates:
  - Lease ID matches active lease
  - Lease not expired
  - Modification times match (If-Modified-Since, If-Unmodified-Since, ETag)

**Key Files**:
- BlobHandler: lines 79-80, 120-121, 264-265, 342-344
- ContainerHandler: lines 117, 219, 254
- Lease validators in Phase 9 (lease/)

**Rust Implementation**:
- Pass conditions as enum/struct variants
- metadataStore.validateConditions() before operation
- Use traits for condition validation

---

### 5. **Streaming/Body Handling**

**Pattern**:
- Input: NodeJS.ReadableStream (req.body)
- Output: NodeJS.ReadableStream (res.body)
- Download operations: stream from extentStore through optional MD5 computation
- Upload operations: stream to extentStore, compute MD5 in parallel

**Key Files**:
- BlobHandler.download(): lines 998-1095 (range parsing, extent reading, MD5 computation)
- BlockBlobHandler.upload(): stream handling
- BlobBatchHandler.requestBodyToString(): buffer accumulation (4MB limit)
- SubResponseTextBodyStream: Writable stream accumulating batch response

**Rust Equivalent**:
- Use tokio streams or similar async iterators
- Implement AsyncRead/AsyncWrite traits
- MD5 computation via streaming MD5 hasher
- Buffer accumulation for batch responses

---

### 6. **Multipart Batch Parsing**

**Pattern**:
```
Input: multipart/mixed body with boundary
├─ Parse boundary from Content-Type header
├─ Split by boundary
├─ For each part:
│  ├─ Parse headers (Content-ID, Content-Type)
│  ├─ Parse HTTP request line (METHOD /path HTTP/1.1)
│  ├─ Parse request headers
│  └─ Create BlobBatchSubRequest wrapper
└─ Validate all subrequests same operation
└─ Execute each via middleware pipeline
└─ Serialize responses as multipart/mixed
```

**Key File**: BlobBatchHandler.ts lines 325-417 (parseSubRequests), lines 420-449 (serializeSubResponse)

**Parsing Details**:
- HTTP_LINE_ENDING constant: `\r\n`
- HTTP_HEADER_DELIMITER: `:`
- Split phases: boundary → per-request → headers/body
- Content-ID mandatory, must be numeric

**Rust Implementation**:
- nom parser for multipart/mixed format
- Recreate BlobBatchSubRequest/SubResponse wrappers
- Execute pipeline on each subrequest

---

### 7. **Config/Server Assembly**

**Initialization Chain**:
```
main.ts (12.14)
  └─ BlobEnvironment (12.9) - parse CLI args
  └─ BlobConfiguration (12.10) - config struct
  └─ BlobServer (12.12) - server creation
       └─ Initialize persistence stores (Phase 4, 10)
       └─ BlobRequestListenerFactory (12.11) - create Express app
            └─ Instantiate all 6 handlers (11.1-11.7)
            └─ Register middleware in order (12.3-12.7)
            └─ PageBlobRangesManager singleton (11.9)
```

**Rust Equivalent**:
- Builder pattern for configuration
- Server struct holds all persistence layers + listeners
- Handler factory function creates handlers with injected dependencies
- Middleware chain built during server initialization

---

## IMPLEMENTATION ORDERING RECOMMENDATION FOR PHASE 11/12

### Phase 11 Dependency Graph:
```
11.1 BaseHandler (foundation)
  ├─ 11.2 ServiceHandler
  ├─ 11.3 ContainerHandler
  ├─ 11.4 BlobHandler
  ├─ 11.5 BlockBlobHandler
  ├─ 11.6 PageBlobHandler
  └─ 11.7 AppendBlobHandler

11.8 IPageBlobRangesManager (interface)
11.9 PageBlobRangesManager (impl)
  ↑ (used by 11.4, 11.6)

11.10 BlobBatchHandler
  ├─ needs all of 11.1-11.7 (handler instantiation)
  └─ needs 11.11, 11.12, 11.13 (subrequest wrappers)

11.11 BlobBatchSubRequest
11.12 BlobBatchSubResponse
11.13 SubResponseTextBodyStream
  ↑ (used by 11.10)
```

### Recommended Order:
1. **11.1** BaseHandler (pure foundation)
2. **11.8** IPageBlobRangesManager (interface, zero deps)
3. **11.9** PageBlobRangesManager (core algorithm, critical)
4. **11.5** BlockBlobHandler (simple, no ranges)
5. **11.7** AppendBlobHandler (simple, no ranges)
6. **11.6** PageBlobHandler (uses ranges manager, moderate)
7. **11.4** BlobHandler (complex, uses ranges, many operations)
8. **11.2** ServiceHandler (medium complexity)
9. **11.3** ContainerHandler (high complexity, batch submission)
10. **11.11** BlobBatchSubRequest (wrapper, simple)
11. **11.12** BlobBatchSubResponse (wrapper, simple)
12. **11.13** SubResponseTextBodyStream (stream wrapper, simple)
13. **11.10** BlobBatchHandler (complex, ties everything together)

### Phase 12 Dependency Graph:
```
12.1 constants (no deps)
12.2 utils (no deps)

12.8 IBlobEnvironment (interface)
12.9 BlobEnvironment (impl)
  ↑ used by main.ts

12.10 BlobConfiguration
  ↑ used by BlobServer

12.3 blobStorageContext.middleware
12.4 AuthenticationMiddlewareFactory
12.5 PreflightMiddlewareFactory
12.6 StrictModelMiddlewareFactory
12.7 telemetry.middleware
  ↑ all used by BlobRequestListenerFactory

12.11 BlobRequestListenerFactory
  ├─ needs Phase 11 handlers
  ├─ needs 12.3-12.7 middleware
  └─ creates Express app

12.12 BlobServer
  ├─ needs 12.10 BlobConfiguration
  ├─ needs 12.11 BlobRequestListenerFactory
  └─ manages persistence layers

12.13 BlobServerFactory
  ↑ wraps 12.12

12.14 main.ts
  ├─ needs 12.9 BlobEnvironment
  ├─ needs 12.10 BlobConfiguration
  ├─ needs 12.13 BlobServerFactory
  └─ entry point
```

### Recommended Order:
1. **12.1** constants (no deps)
2. **12.2** utils (no deps)
3. **12.8** IBlobEnvironment (interface)
4. **12.9** BlobEnvironment (impl)
5. **12.3** blobStorageContext.middleware (context extraction)
6. **12.4** AuthenticationMiddlewareFactory (auth composition)
7. **12.5** PreflightMiddlewareFactory (CORS)
8. **12.6** StrictModelMiddlewareFactory (validation)
9. **12.7** telemetry.middleware (telemetry)
10. **12.10** BlobConfiguration (config struct)
11. **12.11** BlobRequestListenerFactory (middleware assembly + Phase 11 handlers)
12. **12.12** BlobServer (server lifecycle + persistence)
13. **12.13** BlobServerFactory (factory wrapper)
14. **12.14** main.ts (entry point)

---

## CRITICAL IMPLEMENTATION NOTES FOR ARAGORN

### High-Fidelity Focus Areas:

1. **PageBlobRangesManager** (11.9)
   - Range merging algorithm is precise: split-first, not merge-all
   - Maintains invariant: ranges are non-overlapping, sorted, persistent refs valid
   - Binary search for impact detection is performance-critical
   - Must preserve extent pointer integrity

2. **BlobBatchHandler** (11.10)
   - Multipart parsing is brittle: boundary detection, header parsing
   - Middleware pipeline MUST execute in exact order
   - All subrequests must be same operation (validation critical)
   - Content-ID tracking per subrequest for response mapping

3. **Streaming/Body Handling**
   - MD5 computation on range requests (lines 1084-1094 in BlobHandler)
   - Extent composition for block blob downloads (readExtents with multiple blocks)
   - 4MB buffer limit in batch request parsing
   - Stream error handling and cleanup

4. **Lease & Conditions**
   - Every operation passes conditions through
   - metadataStore must validate before mutation
   - Snapshot cannot have lease (BlobHandler line 378-382)

5. **Context Extraction** (12.3)
   - Product-style vs path-style URL parsing
   - Account/container/blob name extraction
   - API version validation
   - Server header generation

6. **Middleware Order** (12.11)
   - CRITICAL: middleware registration order affects behavior
   - Context middleware must run first
   - Auth before handlers
   - Serialization after handlers
   - Error handlers last

### Testing Strategy:
- Unit tests for PageBlobRangesManager (edge cases: splits, no impact, partial overlap)
- Integration tests for BlobBatchHandler (multipart parsing, pipeline execution)
- Streaming tests (MD5 computation, extent reading, response body assembly)
- Configuration tests (environment parsing, server initialization)

