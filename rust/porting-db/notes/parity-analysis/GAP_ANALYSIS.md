# TS-vs-Rust Parity Gap Analysis for Azurite

## Issue 1: Create Container ETag Differs

**Failure:** TS=`"0x2617CE0B8607660"` vs Rust=`"0x1BC2B675F878C4D"`

**Root Cause:** Random ETag generation uses different randomness sources:
- **TS** (/home/azureuser/Azurite/src/common/utils/utils.ts:55-66):
  - Uses `new Date().getTime() * Math.round(Math.random() * 30000 + 70000)`
  - Multiplier range: 70000-100000
  
- **Rust** (/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:84-92):
  - Uses `SystemTime::now().duration_since(UNIX_EPOCH).subsec_nanos() as u64` for randomness
  - Multiplier range: 70000-100001 (off-by-one)
  - Uses `Utc::now().timestamp_millis()` for time (should be consistent)

**TS Code:**
```typescript
export function newEtag(): string {
  return (
    '"0x' +
    (new Date().getTime() * Math.round(Math.random() * 30000 + 70000))
      .toString(16)
      .toUpperCase() +
    '"'
  );
}
```

**Rust Code:**
```rust
pub fn newEtag() -> String {
    let now = Utc::now().timestamp_millis() as u64;
    let random = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let multiplier = random % 30001 + 70000;
    format!("\"0x{:X}\"", now.saturating_mul(multiplier))
}
```

**Files Affected:**
- `/home/azureuser/Azurite/src/blob/handlers/ContainerHandler.ts:74-75` (generates ETag at container creation)
- `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/container_handler.rs:90-91` (generates ETag at container creation)

**Minimal Fix:** In Rust, replace randomness source with `Math.random()`-like behavior using a deterministic seed or ensure it matches TS exactly. The issue is that `subsec_nanos()` produces different values than JS `Math.random()`.

---

## Issue 2-4: Upload/Download/Get-Properties/Lease/Page-Blob/Share Same Blob ETag Mismatch

**Failures:**
- Upload block blob: TS=`"0x235FD2D13591220"` vs Rust=`"0x2719B5CD8F0CF50"`
- Download block blob: Same mismatch
- Get blob properties: Same mismatch
- Create page blob + page ranges: TS=`"0x25BE4625DC4A2A0"` vs Rust=`"0x1DDFA0D6F81D6FC"`
- Lease operations: TS=`"0x1E140E66B154ED0"` vs Rust=`"0x1DFA727C0FA28CA"`

**Root Cause:** All blob creation operations use `newEtag()` which has the randomness mismatch from Issue #1. Once a blob is created with a mismatched ETag, all subsequent operations on that blob return the same mismatched ETag.

**Key Files:**
- **TS blob metadata store:** `/home/azureuser/Azurite/src/blob/persistence/LokiBlobMetadataStore.ts`
  - Line 1460, 1512, 1957, 2146, 2334, 2466, 2783, 2848, 2970, 3071, 3560: `newEtag()` calls
  
- **Rust blob metadata store:** `/home/azureuser/Azurite/rust/crates/azurite-blob/src/persistence/loki_blob_metadata_store.rs`
  - Line 187, 1769, 1824, 2774, 2818, 3024, 3454, 3536, 3667, 3770: `new_etag()` calls

- **TS blob handlers:** `/home/azureuser/Azurite/src/blob/handlers/BlockBlobHandler.ts`, `PageBlobHandler.ts`
- **Rust blob handlers:** `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/block_blob_handler.rs`, `page_blob_handler.rs`

**Minimal Fix:** Fix the ETag randomness source in `newEtag()` (see Issue #1).

---

## Issue 5: Snapshot Adds Rust-Only `x-ms-request-server-encrypted` and ETag Mismatch

**Failure:** 
- ETag mismatch (inherits from Issue #2)
- Rust adds header: `x-ms-request-server-encrypted: true` (TS returns empty list)

**Root Cause:** Two separate issues:
1. **ETag mismatch:** Inherits from general blob ETag randomness issue
2. **Extra header in Rust:** The Rust snapshot handler explicitly adds `isServerEncrypted` which maps to response header, while TS does not

**TS Code** (/home/azureuser/Azurite/src/blob/handlers/BlobHandler.ts:618):
```typescript
const response: Models.BlobCreateSnapshotResponse = {
  statusCode: 201,
  eTag: res.properties.etag,
  lastModified: res.properties.lastModified,
  requestId: context.contextId,
  date: context.startTime!,
  version: BLOB_API_VERSION,
  snapshot: res.snapshot,
  clientRequestId: options.requestId
};
```

**Rust Code** (/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/blob_handler.rs:752):
```rust
response.insert_field("snapshot", string_value(&res.snapshot));
response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
Ok(response)
```

**Files Affected:**
- TS: `/home/azureuser/Azurite/src/blob/handlers/BlobHandler.ts:576-627` (createSnapshot)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/blob_handler.rs:720-754` (createSnapshot)

**Minimal Fix:** Remove the `response.insert_field("isServerEncrypted", ...)` line from Rust createSnapshot handler (line 752).

---

## Issue 6: Copy Blob Returns TS 501 APINotImplemented vs Rust 500 Empty Body

**Failure:** 
- Status: 501 (TS) vs 500 (Rust)
- TS headers: `content-type: application/xml`, `x-ms-error-code: APINotImplemented`
- Rust: no headers, empty body

**Root Cause:** Copy blob (`putBlobFromUrl`) is not implemented in either version, but:
- **TS** correctly raises `NotImplementedError` with proper 501 status and error code
- **Rust** raises `NotImplementedError` but it's somehow being converted to 500 with empty body

**TS Code** (/home/azureuser/Azurite/src/blob/handlers/BlockBlobHandler.ts:164-167):
```typescript
public async putBlobFromUrl(contentLength: number, copySource: string, ...): Promise<...> {
  throw new NotImplementedError(context.contextId);
}
```

**Rust Code** (/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/block_blob_handler.rs:470-480):
```rust
async fn putBlobFromUrl(
    &self,
    _contentLength: f64,
    _copySource: String,
    _options: models::BlockBlobPutBlobFromUrlOptionalParams,
    context: Context,
) -> crate::generated::GeneratedResult<models::BlockBlobPutBlobFromUrlResponse> {
    Err(Box::new(NotImplementedError::new(
        context.contextId().as_deref(),
    )))
}
```

**NotImplementedError Details** (/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/not_implemented_error.rs:11-20):
```rust
impl NotImplementedError {
    pub fn new(requestID: Option<&str>) -> Self {
        Self(StorageError::new(
            501,  // Status code is correct
            "APINotImplemented",  // Error code is correct
            "Current API is not implemented yet...",
            requestID.unwrap_or(""),
            StorageError::empty_extra(),
        ))
    }
}
```

**Files Affected:**
- TS: `/home/azureuser/Azurite/src/blob/handlers/BlockBlobHandler.ts:164-167`
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/block_blob_handler.rs:470-480`
- Rust Error: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/not_implemented_error.rs`

**Investigation Needed:** Check the middleware/error handling that converts the Rust error response. The 500 status suggests the error is being mishandled during serialization or HTTP response generation. Check the error handler in the generated middleware or the `GeneratedResult` error handler.

**Minimal Fix:** Ensure Rust error responses properly serialize the StorageError with 501 status and include the error code in response headers/body. This may be a middleware issue rather than a handler issue.

---

## Issue 7: Set/Get Blob Metadata ETag Mismatch

**Failure:** TS=`"0x1FC2DF8044F4D40"` vs Rust=`"0x1F3CAD774951A80"`

**Root Cause:** When metadata is set, a new ETag is generated using `newEtag()`. Since `newEtag()` uses the randomness mismatch, the ETags differ.

**TS Code** (/home/azureuser/Azurite/src/blob/handlers/BlobHandler.ts:1312-1330):
```typescript
const res = await this.metadataStore.setBlobMetadata(
  context,
  account,
  container,
  blob,
  options.leaseAccessConditions,
  metadata,
  options.modifiedAccessConditions
);

const response: Models.BlobSetMetadataResponse = {
  statusCode: 200,
  eTag: res.etag,  // Uses ETag from setBlobMetadata
  lastModified: res.lastModified,
  isServerEncrypted: true,
  ...
};
```

**Rust Code** (/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/blob_handler.rs:364-380):
```rust
let res = self
    .base
    .metadataStore
    .setBlobMetadata(
        &context,
        &account,
        &container,
        &blob,
        lease_conds.as_ref(),
        metadata,
        mod_conds.as_ref(),
    )
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

let mut response = GeneratedResponse::new(200);
set_common_fields(&mut response, &context, &options, "requestId");
copy_prop_field(&res, &mut response, "etag", "eTag");
```

**Files Affected:**
- TS: `/home/azureuser/Azurite/src/blob/persistence/LokiBlobMetadataStore.ts` (setBlobMetadata calls `newEtag()`)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/persistence/loki_blob_metadata_store.rs` (setBlobMetadata calls `new_etag()`)

**Minimal Fix:** Fix the `newEtag()` randomness source (see Issue #1).

---

## Issue 8: Create Table Body Includes Rust-Only `preferenceApplied` and `version`

**Failure:** 
TS response body:
```json
{
  "TableName": "DiffTablefa7404ec05"
}
```

Rust response body:
```json
{
  "TableName": "DiffTablefa7404ec05",
  "preferenceApplied": "return-content",
  "version": "2025-11-05"
}
```

**Root Cause:** The Rust table handler adds extra fields to the response body that TS does not include.

**TS Code** (/home/azureuser/Azurite/src/table/handlers/TableHandler.ts:58-100):
```typescript
const response: Models.TableCreateResponse = {
  clientRequestId: options.requestId,
  requestId: tableContext.contextID,
  version: TABLE_API_VERSION,
  date: context.startTime,
  statusCode: 201
};

response.tableName = table;
updateTableOptionalOdataAnnotationsForResponse(
  response,
  account,
  table,
  this.getOdataAnnotationUrlPrefix(tableContext, account),
  accept
);

this.updateResponsePrefer(response, tableContext);
this.updateResponseAccept(tableContext, accept);

return response;
```

**Rust Code** (/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs):
```rust
async fn create(
    &self,
    table: TableProperties,
    options: TableCreateOptionalParams,
    context: Context,
) -> Result<TableCreateResponse, crate::errors::StorageError> {
    // ... validation ...
    
    let mut response = GeneratedResponse::new(201);
    self.base
        .add_response_metadata(&mut response, &options, &context, true);  // Adds extra fields
    response
        .fields
        .extend(self.table_response_value(&account, &table_name, &accept, &context));
    self.base.update_response_prefer(&mut response, &context);  // Adds preferenceApplied
    self.base
        .set_response_content_type(&mut response, Some(&accept));
    Ok(response)
}
```

**Helper Function** (/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs):
```rust
fn table_response_value(
    &self,
    account: &str,
    table: &str,
    accept: &str,
    context: &Context,
) -> GeneratedObject {
    let annotations = updateTableOptionalOdataAnnotationsForResponse(...);
    let mut response = GeneratedObject::new();
    response.insert(String::from("TableName"), string_value(table.to_string()));
    add_optional(&mut response, "odata.metadata", annotations.odatametadata);
    add_optional(&mut response, "odata.type", annotations.odatatype);
    add_optional(&mut response, "odata.id", annotations.odataid);
    add_optional(&mut response, "odata.editLink", annotations.odataeditLink);
    response
}
```

The `add_response_metadata()` and `update_response_prefer()` functions are adding extra fields.

**Files Affected:**
- TS: `/home/azureuser/Azurite/src/table/handlers/TableHandler.ts:58-100` (create method)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs:85-109` (create method)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/base_handler.rs` (add_response_metadata, update_response_prefer)

**Minimal Fix:** 
1. Find where `preferenceApplied` and `version` are being added in Rust response
2. For `preferenceApplied`: Don't add it to the body if it's being added via `update_response_prefer()` - check if TS puts it in headers instead
3. For `version`: Don't add it to the response body in create table (it should only be in headers)
4. Investigate whether these belong in response headers vs. body

---

## Issue 9: Insert Entity ETag Differs by Timestamp

**Failure:** 
- TS: `W/"datetime'2026-03-16T21%3A44%3A17.0175310Z'"`
- Rust: `W/"datetime'2026-03-16T21%3A44%3A17.0212155Z'"`

**Root Cause:** Different high-precision timestamp implementations cause different subsecond values in the ETag.

**TS Code** (/home/azureuser/Azurite/src/common/utils/utils.ts:175-195):
```typescript
export function truncatedISO8061Date(
  date: Date,
  withMilliseconds: boolean = true,
  hrtimePrecision: boolean = false
): string {
  const dateString = date.toISOString();

  if (hrtimePrecision) {
    return (
      dateString.substring(0, dateString.length - 1) +
      process.hrtime()[1].toString().padStart(4, "0").slice(0, 4) +  // Uses process.hrtime()[1]
      "Z"
    );
  }
  return withMilliseconds
    ? dateString.substring(0, dateString.length - 1) + "0000" + "Z"
    : dateString.substring(0, dateString.length - 5) + "Z";
}
```

**Rust Code** (/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:108-130):
```rust
pub fn truncatedISO8061Date(
    date: DateTime<Utc>,
    withMilliseconds: bool,
    hrtimePrecision: bool,
) -> String {
    let dateString = date.to_rfc3339_opts(SecondsFormat::Millis, true);
    if hrtimePrecision {
        let hrtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();  // Uses subsec_nanos() instead of process.hrtime()[1]
        let padded = format!("{:0>4}", hrtime);
        return format!("{}{}Z", &dateString[..dateString.len() - 1], &padded[..4]);
    }
    // ...
}
```

**Key Difference:** 
- TS uses `process.hrtime()[1]` (nanosecond component of high-resolution timer)
- Rust uses `SystemTime::now().subsec_nanos()` (nanosecond component of current time)
- Both extract 4 digits, but the timing of when they sample differs, causing microsecond differences

**TS Code for Entity Creation** (/home/azureuser/Azurite/src/table/handlers/TableHandler.ts:298-310):
```typescript
private createPersistedEntity(
    context: Context,
    options: Models.TableInsertEntityOptionalParams | ...,
    partitionKey: string,
    rowKey: string
  ) {
    const modTime = truncatedISO8061Date(context.startTime!, true, true);  // Uses hrtimePrecision=true
    const eTag = newTableEntityEtag(modTime);  // ETag includes the high-precision timestamp
    // ...
  }
```

**Rust Code for Entity Creation** (/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs:87-100):
```rust
fn create_persisted_entity(
    &self,
    context: &Context,
    properties: Option<GeneratedObject>,
    partition_key: String,
    row_key: String,
) -> Entity {
    let mod_time = newHighPrecisionTimeStamp(BaseHandler::start_time(context));  // Calls the Rust function
    let e_tag = newTableEntityEtag(&mod_time);
    // ...
}
```

**Table ETag Format** (/home/azureuser/Azurite/rust/crates/azurite-table/src/utils/utils.rs:308-310):
```rust
pub fn new_table_entity_etag(high_pres_mod_time: &str) -> String {
    format!("W/\"datetime'{}'\"", high_pres_mod_time.replace(':', "%3A"))
}
```

**Files Affected:**
- TS: `/home/azureuser/Azurite/src/common/utils/utils.ts:175-195` (truncatedISO8061Date)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:108-130` (truncatedISO8061Date)
- TS: `/home/azureuser/Azurite/src/table/handlers/TableHandler.ts:298-310` (createPersistedEntity)
- Rust: `/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs:87-100` (create_persisted_entity)

**Minimal Fix:** Replace Rust's `SystemTime::now().subsec_nanos()` with equivalent of `process.hrtime()[1]`. This requires either:
1. Adding a Node.js FFI call to `process.hrtime()` (not feasible for pure Rust)
2. Using a static counter or clock to approximate the nanosecond value consistently
3. Accepting that timestamps will differ slightly and only comparing the main part (milliseconds)

The most TS-faithful approach would be to match the sampling time exactly by using `context.startTime()` for both implementations and not calling `SystemTime::now()` separately.

---

## Summary of Minimal Rust Fixes

| Issue | File | Fix |
|-------|------|-----|
| 1, 2-4, 7 | `/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:84-92` | Replace `subsec_nanos()` randomness with JS-compatible `Math.random()`-like behavior. Use a static RNG or seed-based approach to match TS behavior exactly. |
| 5 | `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/blob_handler.rs:752` | Remove line: `response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));` from createSnapshot. |
| 6 | `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/block_blob_handler.rs` (middleware) | Verify error response serialization. Check that NotImplementedError with 501 status is properly converted to HTTP 501 response with `application/xml` content-type and `x-ms-error-code` header. |
| 8 | `/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/base_handler.rs` | Audit `add_response_metadata()` and `update_response_prefer()` to not add `preferenceApplied` and `version` to response body for table creation. These should be headers only or omitted entirely. |
| 9 | `/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:116-120` | Replace `SystemTime::now().subsec_nanos()` with a value sampled at context creation time (use `context.startTime()` consistently). This ensures deterministic nanosecond values. |

