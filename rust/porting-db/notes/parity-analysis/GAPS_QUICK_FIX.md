# Quick Reference: 9 TS-vs-Rust Parity Gaps & Fixes

## All ETags Mismatch (Issues 1-4, 7, 9) → Root Cause: RNG

**Status:** Create container, upload blob, page blob, lease, snapshot ETag all differ + table entity timestamp differs

**Root Causes:**
1. **Issues 1-4, 7:** Blob ETag random number generation uses wrong randomness source
2. **Issue 9:** Table entity ETag uses `SystemTime::now().subsec_nanos()` instead of process.hrtime()

**Fix Locations:**

### A. Blob ETag Randomness (Issues 1-4, 7)
**File:** `/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:84-92`

**Current (Rust):**
```rust
pub fn newEtag() -> String {
    let now = Utc::now().timestamp_millis() as u64;
    let random = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let multiplier = random % 30001 + 70000;  // Problem: subsec_nanos() ≠ Math.random()
    format!("\"0x{:X}\"", now.saturating_mul(multiplier))
}
```

**Reference (TS):**
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

**Expected Fix:**
- Import a proper RNG crate (e.g., `rand`)
- Replace `subsec_nanos()` with `random() * 30000 + 70000` equivalent
- Ensure multiplier range is exactly 70000-100000 (not 70000-100001)

---

### B. Table Entity Timestamp Precision (Issue 9)
**File:** `/home/azureuser/Azurite/rust/crates/azurite-common/src/utils/utils.rs:108-130`

**Current (Rust):**
```rust
pub fn truncatedISO8061Date(
    date: DateTime<Utc>,
    withMilliseconds: bool,
    hrtimePrecision: bool,
) -> String {
    let dateString = date.to_rfc3339_opts(SecondsFormat::Millis, true);
    if hrtimePrecision {
        let hrtime = SystemTime::now()  // ← Problem: samples NOW, not from context.startTime()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let padded = format!("{:0>4}", hrtime);
        return format!("{}{}Z", &dateString[..dateString.len() - 1], &padded[..4]);
    }
    // ...
}
```

**Reference (TS):**
```typescript
if (hrtimePrecision) {
    return (
      dateString.substring(0, dateString.length - 1) +
      process.hrtime()[1].toString().padStart(4, "0").slice(0, 4) +
      "Z"
    );
}
```

**Expected Fix:**
- Don't call `SystemTime::now()` inside this function
- Accept high-precision nanosecond value as parameter (or use context.startTime())
- Treat input `date` as the source of truth, extract subsec_nanos() from it once
- This ensures the same timestamp is used consistently throughout entity creation

---

## Non-Randomness Issues

### Issue 5: Remove `isServerEncrypted` from Snapshot Response
**File:** `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/blob_handler.rs:752`

**Action:** Delete this line:
```rust
response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
```

**Reason:** TS snapshot response (line 618) does NOT include isServerEncrypted, so Rust shouldn't either.

---

### Issue 6: Fix Copy Blob Error Response (501 → 500)
**Files:** 
- `/home/azureuser/Azurite/rust/crates/azurite-blob/src/handlers/block_blob_handler.rs:470-480` (handler returns correct NotImplementedError)
- Middleware error serialization (check generated error handlers)

**Problem:** NotImplementedError is created correctly with 501 status and "APINotImplemented" code, but HTTP response comes back as 500 with empty body.

**Investigation Steps:**
1. Check `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/not_implemented_error.rs` - it sets 501 correctly
2. Trace how `GeneratedResult<T>` errors are serialized to HTTP responses
3. Verify that `content-type: application/xml` and `x-ms-error-code` headers are being added
4. This is likely a middleware/serialization issue, not the handler

---

### Issue 8: Remove Extra Fields from Create Table Response Body
**File:** `/home/azureuser/Azurite/rust/crates/azurite-table/src/handlers/table_handler.rs:97-103`

**Problem:** Rust adds `preferenceApplied` and `version` to response body; TS does not.

**Current (Rust):**
```rust
let mut response = GeneratedResponse::new(201);
self.base
    .add_response_metadata(&mut response, &options, &context, true);  // Adds extra fields?
response
    .fields
    .extend(self.table_response_value(&account, &table_name, &accept, &context));
self.base.update_response_prefer(&mut response, &context);  // Adds preferenceApplied?
```

**Expected (TS):**
```typescript
const response: Models.TableCreateResponse = {
  clientRequestId: options.requestId,
  requestId: tableContext.contextID,
  version: TABLE_API_VERSION,  // ← In response object, but not serialized to body
  date: context.startTime,
  statusCode: 201
};
response.tableName = table;
```

**Action:** Audit `add_response_metadata()` and `update_response_prefer()` in base_handler.rs to ensure they:
- Only add headers, not body fields
- Don't add `version` to body (it's a response header)
- Don't add `preferenceApplied` to body (might be header only or omitted)

---

## Implementation Priority

1. **High:** Fix Issues 1-4 (blob ETag RNG) - affects 4 test failures
2. **High:** Fix Issue 9 (table entity timestamp) - affects 1 test failure
3. **Medium:** Fix Issue 5 (remove serverEncrypted) - affects 1 test failure
4. **Medium:** Fix Issue 8 (remove extra table fields) - affects 1 test failure
5. **Low:** Fix Issue 6 (error response serialization) - affects 1 test failure

---

## Testing

Re-run differential test to verify:
```bash
cd /home/azureuser/Azurite/rust
python scripts/differential_test.py
```

Expected result after fixes: All 16 scenarios should pass (5 passing + 11 fixed = 16 total).
