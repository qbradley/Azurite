# PHASE 11 & 12: KEY FILE:LINE REFERENCES FOR DEEP DIVES

## CRITICAL SECTIONS TO PRESERVE (HIGH FIDELITY)

### BlobHandler.ts (11.4) - DOWNLOAD & RANGE HANDLING

**Download with Range Support** (BlobHandler.ts:1000-1095)
- Line 1010-1012: Range header parsing via `deserializeRangeHeader()`
- Line 1021-1022: Range start/end extraction
- Line 1025-1038: Range validation (start > length, shift to blob size)
- Line 1040-1041: Content length and partial read flag calculation
- Line 1052-1064: Block blob extent reading (single extent with offset/count)
- Line 1067-1073: Block blob with multiple blocks (readExtents with block list)
- Line 1076-1080: Content-Range header formatting
- Line 1084-1094: MD5 computation for small <4MB downloads
- Line 1098: Response status code 206 (Partial) vs 200 (Full)

**Lease & Conditions Integration** (BlobHandler.ts:79-80, 120-121, 264-265, 342-344)
- Line 79-80: download() passes leaseAccessConditions + modifiedAccessConditions
- Line 120-121: getProperties() same pattern
- Line 264-265: setHTTPHeaders() same pattern
- Line 342-344: acquireLease() same pattern

**Lease Snapshot Constraint** (BlobHandler.ts:378-382)
- Line 378: Query for snapshot parameter
- Line 380-384: Throws error if snapshot exists ("A lease cannot be granted for a blob snapshot")

**Metadata Key Case Preservation** (BlobHandler.ts:333-335)
- convertRawHeadersToMetadata() to preserve case from headers

**setHTTPHeaders Redirect** (BlobHandler.ts:245-266)
- Line 246: Get sequenceNumberAction header
- Line 249: Get blobSequenceNumber header
- Line 252-266: If action present, redirect to updateSequenceNumber (not setHTTPHeaders)

### ContainerHandler.ts (11.3) - BATCH & LEASE

**Batch Submission** (ContainerHandler.ts:334-367)
- Line 341: Extract boundary from Content-Type header
- Line 343-344: Create NEW BlobBatchHandler instance (NOT singleton)
- Line 346-350: Call submitBatch() on handler
- Line 352-354: Wrap response in Readable stream
- Line 359-362: Return 202 status with multipart/mixed content type

**Metadata Key Case Preservation** (ContainerHandler.ts:66-67, 206-209)
- Line 66-67: create() preserves case
- Line 206-209: setMetadata() preserves case

**Container Delete TODO** (ContainerHandler.ts:165-168)
- Line 165-168: TODO comment about async blob cleanup needed

**ACL Serialization Issue** (ContainerHandler.ts:274-278)
- Line 274-278: Known XML formatting bug in generated code

### PageBlobRangesManager.ts (11.9) - RANGE ALGORITHM

**mergeRange Algorithm** (PageBlobRangesManager.ts:51-115)
- Line 55-57: Extract start, end, persistency from new range
- Line 59: Find impacted ranges
- Line 61-67: Calculate impacted count
- Line 72-74: If no impact: simple splice insert
- Line 77-78: Get first/last impacted ranges
- Line 84-93: Split first range if overlaps before start
- Line 96: Insert new range
- Line 99-110: Split last range if overlaps after end
- Line 113: Splice old ranges, insert new set

**clearRange Algorithm** (PageBlobRangesManager.ts:117-179)
- Line 122-130: Similar impact detection
- Line 141-170: Split first/last ranges to preserve non-cleared portions

**selectImpactedRanges (Binary Search)** (PageBlobRangesManager.ts:328-362)
- Binary search for ranges overlapping [start, end]
- Returns [firstIndex, lastIndex] or [-1, -1] if no impact

**locateFirstImpactedRange** (PageBlobRangesManager.ts:372-419)
- Binary search for first range where end >= start

**locateLastImpactedRange** (PageBlobRangesManager.ts:429-473)
- Binary search for last range where start <= end

### BlobBatchHandler.ts (11.10) - MULTIPART & PIPELINE

**Middleware Pipeline Setup** (BlobBatchHandler.ts:62-244)
- Line 62-75: subRequestContextMiddleware setup
- Line 77-84: subRequestDispatchMiddleware setup
- Line 87-110: Authenticators array construction
- Line 112-131: subRequestAuthenticationMiddleware setup
- Line 133-140: subRequestDeserializeMiddleware setup
- Line 142-185: Handler instantiation (all 6 handlers)
- Line 187-197: Handler middleware factory setup
- Line 199-226: Serialization, error, end middleware
- Line 228-236: handlePipeline array in ORDER (CRITICAL)
- Line 238-241: operationFinder pipeline (2 stages)
- Line 243: errorHandler assignment

**Middleware Order** (BlobBatchHandler.ts:228-236)
```
1. subRequestContextMiddleware
2. subRequestDispatchMiddleware
3. subRequestAuthenticationMiddleware
4. subRequestDeserializeMiddleware
5. subRequestHandlerMiddleware
6. subRequestSerializeMiddleWare
7. subRequestEndMiddleWare
```

**Stream to Buffer Conversion** (BlobBatchHandler.ts:246-293)
- Line 282: Allocate 4MB buffer
- Line 264-265: Error if stream exceeds buffer size

**Sub Request Parsing** (BlobBatchHandler.ts:325-417)
- Line 332-334: Split by batchRequestEnding, then perRequestPrefix
- Line 342-343: Split request by HTTP_LINE_ENDING
- Line 355-365: Parse headers (extract Content-ID)
- Line 371-382: Parse HTTP request line and path
- Line 385-392: Parse request headers
- Line 393: Determine operation via getSubRequestOperation()
- Line 394-407: Validate operation support + all same type

**Response Serialization** (BlobBatchHandler.ts:420-449)
- Line 425: For each subResponse
- Line 426: Write boundary
- Line 427: Content-Type header
- Line 429: Content-ID header
- Line 431-434: HTTP response line (protocol, status, message)
- Line 436-439: Response headers
- Line 441-444: Response body if present
- Line 448: Final boundary with -- suffix

**Handler Execution** (BlobBatchHandler.ts:529-564)
- Line 531: Get pipeline from this.handlePipeline
- Line 532: Get error handler from this.errorHandler
- Line 537-556: next() callback handling
- Line 538-540: Guard against double completion
- Line 543-546: Error path → error handler
- Line 547-563: Success path → next middleware
- Line 558-563: Execute first middleware

### BlobRequestListenerFactory.ts (12.11) - MIDDLEWARE ASSEMBLY

**Handler Instantiation** (BlobRequestListenerFactory.ts:72-120)
- Line 72: Create PageBlobRangesManager singleton (shared instance)
- Line 76-80: AppendBlobHandler
- Line 82-88: BlobHandler (with rangesManager)
- Line 89-93: BlockBlobHandler
- Line 95-102: ContainerHandler (with accountDataStore, oauth)
- Line 104-109: PageBlobHandler (with rangesManager)
- Line 111-119: ServiceHandler (with accountDataStore, oauth)

**Middleware Composition** (BlobRequestListenerFactory.ts:140-150+)
- Line 141-143: morgan access logging (if enabled)
- Line 146: blobStorageContextMiddleware (context extraction)
- Line 149: dispatchMiddleware (auto-generated)
- [Additional middleware registered after]

### BlobServer.ts (12.12) - SERVER INITIALIZATION

**Constructor** (BlobServer.ts:54-???)
- Sets up HTTP/HTTPS server
- Initializes persistence stores based on config
- Creates request listener via BlobRequestListenerFactory
- Starts garbage collection

### blobStorageContext.middleware.ts (12.3) - CONTEXT EXTRACTION

**Context Extraction** (blobStorageContext.middleware.ts:44-54)
- Line 56: Set Server header
- Line 60+: Validate API version (if not skipApiVersionCheck)
- [Product vs path-style URL parsing]
- [Account/container/blob extraction]

---

## UNIT DEPENDENCIES BY FILE

### BlobHandler.ts (11.4)
- Imports: URLBuilder, axios, URL, IExtentStore, convertRawHeadersToMetadata, getMD5FromStream
- Imports: BlobStorageContext, NotImplementedError, StorageErrorFactory, Models, Context
- Imports: IBlobHandler, ILogger, parseXML, extractStoragePartsFromPath
- Imports: IBlobMetadataStore, BlobModel, BLOB_API_VERSION, EMULATOR_ACCOUNT_KIND, etc.
- Imports: deserializePageBlobRangeHeader, deserializeRangeHeader, getBlobTagsCount, validateBlobTag
- Imports: BaseHandler, IPageBlobRangesManager
- Constructor: metadataStore, extentStore, logger, loose, rangesManager

### ContainerHandler.ts (11.3)
- Constructor: accountDataStore, oauth, metadataStore, extentStore, logger, loose, disableProductStyle
- Creates BlobBatchHandler inline (line 343)

### BlobBatchHandler.ts (11.10)
- Constructor: accountDataStore, oauth, metadataStore, extentStore, logger, loose, disableProductStyle
- Instantiates: AppendBlobHandler, BlobHandler, BlockBlobHandler, ContainerHandler, PageBlobHandler, ServiceHandler
- Creates: PageBlobRangesManager (line 154), all authenticators, middleware factories

### PageBlobRangesManager.ts (11.9)
- Imports: PageRange, PersistencyPageRange, ZERO_EXTENT_ID
- No constructor dependencies (stateless)

### BlobRequestListenerFactory.ts (12.11)
- Constructor: metadataStore, extentStore, accountDataStore, logger, config flags
- Instantiates: all Phase 11 handlers (same 6 as BlobBatchHandler)
- Creates: PageBlobRangesManager singleton (line 72)

### BlobServer.ts (12.12)
- Extends: ServerBase
- Stores: metadataStore, extentMetadataStore, extentStore, accountDataStore, gcManager
- Instantiates: persistence stores based on BlobConfiguration
- Creates: BlobRequestListenerFactory

---

## STRUCTURAL PATTERNS TO PRESERVE

1. **Handler Constructor Consistency**
   - BaseHandler base (metadataStore, extentStore, logger, loose)
   - Handlers extend BaseHandler + implement interface
   - PageBlobHandler, BlobHandler: inject IPageBlobRangesManager
   - ContainerHandler, ServiceHandler: inject IAccountDataStore, OAuthLevel

2. **PageBlobRangesManager Singleton**
   - Created once per request listener
   - Shared between BlobHandler and PageBlobHandler
   - Stateless (no per-blob state)

3. **Middleware Pipeline**
   - Strict order in handlePipeline array
   - Each middleware (req, res, locals, next) callback
   - Locals dictionary accumulates context

4. **Batch Subrequest Lifecycle**
   - Parse → Determine operation → Execute pipeline → Serialize
   - BlobBatchSubRequest/Response wrappers implement IRequest/IResponse
   - SubResponseTextBodyStream accumulates body

5. **Persistence Layer Pattern**
   - metadataStore: all blob metadata operations
   - extentStore: blob data I/O
   - Both injected into handlers

6. **Streaming**
   - Readable for downloads, request bodies
   - Writable for batch responses
   - MD5 computation on streaming data

