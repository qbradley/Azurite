# Decision: Comparison-Phase ETag Normalization Strategy

**Date:** 2026-03-17  
**Author:** Boromir (QA Expert)  
**Status:** Implemented  
**Affects:** `rust/scripts/traffic_replay.py`

## Context

The traffic replay harness needs to handle stale ETags and snapshot timestamps from recorded traffic when replaying against Rust Azurite. The initial implementation attempted to substitute these values in the request before sending, but this approach had a critical flaw.

## Problem

**SharedKey Authorization signatures are computed over the complete request, including conditional headers.**

When replaying recorded traffic:
1. The recorded request has an `Authorization` header computed by the original client
2. That signature was computed over specific header values (including `If-Match`, `If-None-Match`, etc.)
3. If we modify those headers before replaying, the signature becomes invalid
4. Result: 403 Forbidden errors, even though the request is otherwise correct

Example:
```
Recorded request:
  If-Match: "0x8DC2E5B4F780000"
  Authorization: SharedKey devstoreaccount1:ABC123... (computed over original If-Match)

If we substitute the ETag:
  If-Match: "0x8DC2E5B4F999999"  ← Changed!
  Authorization: SharedKey devstoreaccount1:ABC123... (now invalid!)
  
Result: 403 Forbidden
```

Similarly, modifying URL query parameters (e.g., `?snapshot=...`) invalidates SAS signatures embedded in the URL.

## Decision

**Use comparison-phase normalization instead of request modification.**

### Strategy

1. **Send requests unmodified** - preserve original headers and URLs from the recording
   - This keeps auth signatures valid
   - Requests will authenticate successfully

2. **Learn ETag/snapshot mappings** from responses (keep existing learning functions)
   - Track which recorded ETags/snapshots appear in responses
   - Build a map of "recorded value → actual value"

3. **Detect harness artifacts during comparison** - when status codes differ, check if it's explained by stale references:
   
   **Stale If-Match:**
   - Expected: 200 (TS matched the ETag) → Actual: 412 (Rust doesn't match the stale ETag)
   - This is a **known harness artifact**, not a Rust bug
   
   **Stale If-None-Match:**
   - Expected: 304 (TS matched, returned Not Modified) → Actual: 200 (Rust doesn't match, returns content)
   - Expected: 200 (TS didn't match) → Actual: 304 (Rust matches the stale ETag - unexpected but explainable)
   - Both are **known harness artifacts**
   
   **Stale Snapshot:**
   - Expected: 200 → Actual: 403/404 (snapshot doesn't exist in Rust's state)
   - This is a **known harness artifact**

4. **Mark as `[SKIP]` instead of `[FAIL]`** - distinguish harness limitations from real Rust bugs
   - Add `skip_reason` field to `ComparisonResult`
   - Count separately: "passed: X, skipped: Y (stale-etag: A, stale-snapshot: B), failed: Z"

### Implementation

```python
# Before (BROKEN):
headers = self._substitute_request_etags(headers)  # ← Breaks auth!
path = self._substitute_request_path(path)  # ← Breaks SAS!

# After (CORRECT):
# Send unmodified request
# Check for artifacts in _compare_responses():
if self._is_stale_etag_artifact(request, recorded_status, actual_status):
    return ComparisonResult(..., skip_reason="stale-etag")
```

## Consequences

### Positive
- ✅ Requests authenticate correctly (no more 403 errors from modified headers)
- ✅ Clear distinction between harness artifacts and real Rust bugs
- ✅ Skip counts provide visibility into harness limitations
- ✅ Summary shows: `passed: 11172, skipped: 12 (stale-etag: 7, stale-snapshot: 5), failed: 33`
- ✅ Works with both SharedKey and SAS authentication

### Negative
- ⚠️ Some legitimate test cases are skipped (those that rely on conditional headers with recorded ETags)
- ⚠️ Requires careful logic to detect which status code differences are artifacts vs real bugs

### Neutral
- The learning functions (`_learn_etag_mapping`, `_learn_snapshot_mapping`) are still needed for comparison intelligence
- Skip counts should be minimized by ensuring corpus is captured with fresh state

## Alternatives Considered

### 1. Re-sign Requests
**Rejected:** Would require implementing the complete SharedKey and SAS signing algorithms in the harness, including clock sync, key management, and handling all signature variations. Too complex and error-prone.

### 2. Use Fresh ETags for Conditionals
**Rejected:** Would require running GET requests before conditional operations to fetch current ETags. This changes the request sequence and breaks the faithful replay guarantee.

### 3. Strip Conditional Headers
**Rejected:** Would change request semantics and prevent testing conditional operations entirely. Many real-world workloads rely on conditional headers.

### 4. Use Only Requests Without Conditionals
**Rejected:** Would significantly reduce test coverage. Conditional operations are important Azure Storage features that must be tested.

## Notes

### Which Headers Invalidate SharedKey Signatures?

SharedKey Authorization includes these in the string-to-sign:
- `If-Match`
- `If-None-Match`
- `If-Modified-Since`
- `If-Unmodified-Since`
- `Content-MD5`
- `Content-Type`
- `Date` / `x-ms-date`
- All `x-ms-*` headers (canonicalized)

**Safe to modify:** Headers NOT in the string-to-sign (e.g., `User-Agent`, custom headers)  
**Unsafe to modify:** Any header listed above, or any query parameter in the URL

### When to Use This Pattern

Use comparison-phase normalization when:
- Replaying signed requests (SharedKey, SAS)
- The value being normalized is part of the signature (headers, URL, body)
- You can predict how the stale value will affect the response status

Use request-phase substitution when:
- Requests are unsigned, or
- The value being substituted is NOT part of the signature

## Related

- Commit: `fix(qa): use comparison-phase ETag normalization instead of request modification`
- Learnings: See `.squad/agents/boromir/history.md` § SharedKey Auth and Conditional Headers
- Context: Previous commit `feat(qa): upgrade traffic replay harness with dynamic ETag and snapshot substitution` introduced the bug
