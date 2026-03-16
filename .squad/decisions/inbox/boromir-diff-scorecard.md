# Boromir Differential Scorecard

**Date:** 2026-03-16T22:03:51Z  
**Requested by:** Quetzal Bradley

## Summary
I verified the Rust release binaries, rebuilt the x86_64 release target, re-ran the differential harness, fixed the Rust-only parity bugs that were clearly actionable, and then revalidated the port with clippy, fmt, the harness, the integration runner, and the SDK suite.

## Binary Verification
- Verified `rust/target/x86_64-unknown-linux-gnu/release/azurite` exists.
- Rebuilt with `cargo build --release --target x86_64-unknown-linux-gnu` to ensure the release artifacts matched current HEAD.
- Re-ran the harness after the rebuild; the score stayed at **5 pass / 11 fail**, confirming Aragorn's referenced changes do not change the current per-service differential failure set.

## What Aragorn's XML Fix Changed
**Observed answer:** none of the currently failing harness scenarios flipped from fail to pass after the rebuild/rerun.

The XML declaration work is still valid, but in this harness profile the XML-sensitive cases were already not part of the failing set. The current remaining failures were instead dominated by ETag differences, one Rust-only snapshot header, copy-blob error-path handling, and table create body leakage.

## Boromir Fixes Applied
1. **Blob snapshot parity:** removed the Rust-only `x-ms-request-server-encrypted` response header from `createSnapshot`.
2. **NotImplemented wire parity:** updated generated error middleware to unwrap `NotImplementedError` / `NotImplementedinSQLError` wrappers instead of downgrading them to generic 500 responses.
3. **Table create payload parity:** made create-table emit an explicit JSON body so header-only response metadata no longer appears in the payload.
4. **Follow-through:** applied the same NotImplemented middleware handling to queue/table generated middleware so the same regression does not recur there.

## Differential Test Scorecard
```
Differential Test Scorecard
============================
Before (first run):        5 pass / 11 fail
After Aragorn's XML fix:   5 pass / 11 fail
After Boromir's fixes:     6 pass / 10 fail
```

## Failure Breakdown After Boromir's Fixes

### Fixed
- **Create table** — now passes; Rust no longer leaks `preferenceApplied` and `version` into the JSON body.
- **Create snapshot header drift** — fixed; Rust no longer emits the extra `x-ms-request-server-encrypted` header.
- **Copy blob status/header drift** — fixed; Rust now returns the TS-style `501 APINotImplemented` XML error instead of a blank 500.

### Remaining gaps
1. **Create container**  
   - Category: header difference  
   - Gap: `etag` value differs.

2. **Upload block blob**  
   - Category: header difference  
   - Gap: `etag` value differs.

3. **Download block blob**  
   - Category: header difference  
   - Gap: `etag` value differs.

4. **Get blob properties**  
   - Category: header difference  
   - Gap: `etag` value differs.

5. **Create page blob + page ranges**  
   - Category: header difference  
   - Gap: `etag` value differs.

6. **Lease operations**  
   - Category: header difference  
   - Gap: `etag` value differs.

7. **Create snapshot**  
   - Category: header difference  
   - Gap: `etag` value differs.

8. **Set/get blob metadata**  
   - Category: header difference  
   - Gap: `etag` value differs.

9. **Insert entity**  
   - Category: header difference  
   - Gap: `etag` value differs (`W/"datetime'...Z'"` timestamp differs by a few milliseconds).

10. **Copy blob**  
    - Category: body difference  
    - Gap: the XML error envelope now matches, but the `<Message>` text still embeds dynamic `RequestId:` and `Time:` lines that differ between the two live servers.

## Analysis of Remaining Gaps
### Header differences
- The remaining header mismatches are all **ETag values**.
- Blob/container ETags are generated dynamically by `newEtag()` in both stacks.
- Table entity ETags are derived from request-local high-precision timestamps in both stacks.
- In side-by-side differential runs, those values naturally diverge because each server generates its own clock/random-derived identity.

### Body differences
- Only **Copy blob** still differs in the body.
- The XML structure now matches; the remaining mismatch is the dynamic `RequestId` / `Time` suffix embedded inside the error `<Message>` text.

### Status code differences
- None remain after the middleware fix.

## Validation Results
- `cargo clippy --all-targets` ✅
- `cargo fmt --all` ✅
- `cargo build --release --target x86_64-unknown-linux-gnu` ✅
- `bash scripts/run-integration-tests.sh` ✅
- `cargo test -p azurite-integration-tests -- --test-threads=1` ✅ (33 SDK tests passing)

## Recommendation
The remaining differential failures are now dominated by **dynamic identity fields**, not clear deterministic Rust logic bugs. If the team wants the harness score to continue improving, the next QA step should be to normalize dynamic ETags and embedded error `RequestId` / `Time` text in the harness rather than forcing Rust to emit values that are inherently generated independently at runtime.
