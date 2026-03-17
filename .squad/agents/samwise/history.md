# Samwise — History

## Project Context
- **Project:** Azurite — Azure Storage Emulator (TypeScript to Rust port)
- **User:** Quetzal Bradley
- **Stack:** Node.js/TypeScript (source) → Rust (target)
- **Goal:** Faithful TS→Rust translation prioritizing change propagation over idiomatic Rust

## Learnings

### Workspace Ready; Phase 1 Complete; Test Infrastructure Operational (2026-03-13)
**Aragorn Status:** Phase 1 Rust translation complete. All 15 common interfaces translated to `rust/crates/azurite-common/src/`. Porting-db records updated. IEnvironment flattened to local trait pattern (crate-graph safe). Workspace compiles.

**Faramir Status:** Phase 1 & 2 TS analysis complete. Phase 2 fidelity risks flagged:
- `ZERO_EXTENT_ID` circular dependency (must move constant from `src/blob/` to `src/common/`)
- `LastModifyInMS` vs `lastModifiedInMS` field casing mismatch (Loki query depends on exact spelling)
- File/class name asymmetries (preserve in Rust)

**Boromir Status:** Test infrastructure deployed. 9 active Phase 1 parity tests passing. 9 ignored placeholders ready for Phase 2 modules. Per-crate test structure mirrors TS suite. `cargo test` green.

**Next Phase:** All systems ready for Phase 2 interface translation. Aragorn will use Faramir's fidelity risks to guide implementation. Boromir will expand tests incrementally.

### Phase 2 Complete; Phase 3 Analysis Ready (2026-03-13 → 21:30)
**Aragorn Status:** Phase 2 persistence translation COMPLETE. 7 modules ported to `rust/crates/azurite-common/src/`:
- OperationQueue, MemoryExtentStore, FSExtentStore, LokiExtentMetadata, AllExtentsAsyncIterator, ZeroBytesStream, Mutex
- Validation: `cargo check` ✅, `cargo test -p azurite-common` ✅
- Decision recorded: D-008 (ZERO_EXTENT_ID placement)

**Faramir Status:** Phase 3 authentication analysis COMPLETE. 5 files analyzed. 3 critical fidelity constraints:
1. Account SAS sentinel enum members (Any permissions/resource types) are validation-only — exclude from serialization
2. Serialization order is contract-sensitive (rwdxlacuptfiy, btqf, sco) — must replicate exactly
3. IP range type asymmetry (SasIPRange vs IIPRange) — preserve with explicit compatibility layer

**Boromir Status:** Phase 1 parity coverage now 26 active tests + 4 ignored placeholders. All workspace compilation clean. Phase 3 placeholders ready to unignore incrementally.

**Ready for Next Round:** Phase 3 implementation can begin immediately with all fidelity constraints documented. Samwise reviews API governance. Boromir expands test coverage.


### Phase 3 Translation + Phase 4 Analysis Complete (2026-03-13 → 22:10)
- **Aragorn:** Phase 3 translation COMPLETE. 5 auth files (IIPRange, AccountSASPermissions, AccountSASServices, AccountSASResourceTypes, IAccountSASSignatureValues) ported to `rust/crates/azurite-common/src/authentication/`. Tests passing. Account-SAS signing ready for Phase 4 utils consolidation.
- **Faramir:** Phase 4 utilities/config analysis COMPLETE. 11 modules analyzed. Fidelity hazards documented: Telemetry `instaceID` misspelling, knownHosts redaction quirk, WinstonLoggerStrategy tab default, Environment CLI arg duplication. Decision D-002 (preserve Phase 4 quirks) recorded; awaiting Gandalf/Samwise approval.
- **Boromir:** Phase 2 parity tests ACTIVATED. 7 modules all passing. Old placeholders removed. Test suite clean at 36 active + 8 ignored.
- **Metrics:** 44 total tests passing, 8 ignored, 38 porting-db records, 108 Rust source files.
- **Ready:** Phase 4 implementation can proceed post-D-002 approval. Phase 2 integration tests ready for Phase 3 coordination.

### Protocol Parity Gap Analysis Complete (2026-03-16)
**Full report:** `.squad/decisions/inbox/samwise-protocol-parity.md`

**Critical findings:**
1. **XML Declaration Missing (P-001):** Every XML response body in the Rust port lacks `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` that the TS version (via xml2js) includes. Affects all blob and queue service responses. One-day fix: prepend declaration in `stringifyXML()`.
2. **Cross-Account SAS Unimplemented (P-002):** `validateCopySource()` in blob_handler.rs ignores the source account parameter entirely. No SAS signature validation, no public access fallback, no archive tier check. This causes 8 test failures. 1-2 week fix.

**Moderate findings:**
3. Error body whitespace differs — TS uses pretty-printed XML (via default xml2js Builder), Rust uses compact hand-built XML.
4. `maxresults` boundary values (≤0) unvalidated in both TS and Rust.
5. No version-conditional handler behavior in either implementation (all 45 API versions treated identically).

**Confirmed parity areas:**
- SharedKey signature computation (string-to-sign, HMAC-SHA256, canonical resource)
- Account SAS permission/service/resource-type serialization orders
- Blob/Queue/Table SAS canonical name formats and version-specific signing
- OAuth/JWT validation (claims, audiences, HTTPS enforcement)
- RFC 1123 date formatting, ETag format, boolean serialization, null handling
- OData annotation levels, EDM type system, batch multipart formatting
- Error XML/JSON structure (modulo declaration and whitespace)
- Response header names, values, and pipeline behavior

**Recommended verification approach:** Proxy-based differential testing — mirror SDK requests to both TS and Rust, compare responses semantically (parsed XML/JSON, not byte-level). Priority SDKs: JS, .NET, Python, Java, Go.

### Swagger-Based Differential Test Module Complete (2026-03-17)
**Module:** `rust/scripts/swagger_diff/swagger_parser.py`

Built Part 1 of the swagger-based differential test generator. Module provides:
1. **SwaggerSpec class** — Parses `swagger/blob-storage-2021-10-04.json`, resolves all $ref references (103 parameters, 56 definitions), loads 72 operations from x-ms-paths
2. **Operation dependency DAG** — 5-tier system (Tier 0: no deps, Tier 1: needs container, Tier 2: needs blob, Tier 3: needs lease, Tier 4: needs snapshot). DependencyDAG.get_setup_chain() returns ordered prerequisites
3. **RequestBuilder class** — Generates complete HTTP requests with SharedKey auth (reuses auth logic from differential_test.py). Handles parameter defaults, path substitution, header/query/body construction
4. **SharedKey authentication** — Full HMAC-SHA256 signature computation with canonicalized headers and resources (account: devstoreaccount1, standard Azurite test key)

**Key architectural decisions:**
- **x-ms-paths discriminators** — Path templates like `/{containerName}/{blob}?BlockBlob` use query params as operation discriminators in swagger. These are NOT real query parameters and are filtered out (discriminators: BlockBlob, PageBlob, AppendBlob)
- **Parameter generation** — Only required parameters included by default. Optional parameters skipped unless in overrides dict. Smart defaults: containerName/blob get random suffixes, x-ms-version=2021-10-04, x-ms-date=RFC1123(now), x-ms-blob-type inferred from operation_id
- **Body handling** — BlockBlob_Upload → b"hello world", PageBlob_UploadPages → 512-byte aligned zeros, AppendBlob_AppendBlock → b"appended content"
- **Content-Length** — Auto-computed from body and injected into headers after body is built

**File paths:**
- Parser: `rust/scripts/swagger_diff/swagger_parser.py` (650 lines)
- Package: `rust/scripts/swagger_diff/__init__.py` (existing)
- Swagger spec: `swagger/blob-storage-2021-10-04.json` (12,617 lines)
- Auth reference: `rust/scripts/differential_test.py` (lines 1022-1120)

**Usage pattern for Boromir:**
```python
from swagger_diff.swagger_parser import SwaggerSpec, RequestBuilder

spec = SwaggerSpec('swagger/blob-storage-2021-10-04.json')
builder = RequestBuilder('devstoreaccount1', account_key, '127.0.0.1', 10000)

# Get operation and its prerequisites
op = spec.get_operation('Blob_SetMetadata')
chain = spec.get_dependency_chain('Blob_SetMetadata')  # [Container_Create, BlockBlob_Upload]

# Build request with overrides
request = builder.build_request(op, overrides={'_containerName': 'mycontainer'})
```

**Next phase:** Boromir will build the runner module that executes these requests against both TS and Rust servers, compares responses, and generates differential test reports.

### Azure Batch Request Format Implementation (2026-03-17)
**Module:** `rust/scripts/swagger_diff/swagger_parser.py` (added batch support)

Fixed the swagger differential test harness to properly generate Azure Storage batch request bodies for `Service_SubmitBatch` and `Container_SubmitBatch` operations.

**Azure Batch Request Format Requirements:**
1. **Content-Type:** Must be `multipart/mixed; boundary=batch_<uuid>` (not `application/octet-stream`)
2. **Body Structure:** Multipart MIME with embedded HTTP sub-requests
3. **Sub-request Format:** Each part contains:
   - `Content-Type: application/http`
   - `Content-Transfer-Encoding: binary`
   - `Content-ID: <sequence-number>`
   - Full HTTP request message (method, path, headers, optional body)
4. **Sub-request Authentication:** Each embedded sub-request MUST have its own SharedKey Authorization header computed independently

**Implementation Details:**
- Added `_build_batch_body()` method to generate proper multipart batch bodies
- Modified `_build_body()` to delegate batch operations to the specialized handler
- Modified `_generate_param_values()` to skip setting Content-Type for batch operations (set by batch body builder)
- Each batch contains one DELETE sub-request targeting the blob created by the setup chain
- Sub-request uses full canonicalized path (`/devstoreaccount1/container/blob`) with independent SharedKey signature
- Boundary uses UUID format: `batch_<uuid>`
- Body uses CRLF line endings (`\r\n`) per HTTP multipart spec

**Key Learning:** Batch operations are fundamentally different from normal requests because they contain complete HTTP requests as payload. The outer request needs one SharedKey signature, and each inner sub-request needs its own independent SharedKey signature computed over its own headers and path. The test harness now generates valid batch requests that both TS and Rust implementations can process.
