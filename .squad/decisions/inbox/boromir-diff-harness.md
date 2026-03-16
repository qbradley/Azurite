# Boromir Decision Inbox — Differential Harness

## 2026-03-16: Side-by-side TS vs Rust REST diff harness

**By:** Boromir (QA Expert)

**What:** Added a standalone differential harness at `rust/scripts/differential-test.sh` backed by `rust/scripts/differential_test.py`.

The harness:
- starts TypeScript blob/queue/table services on `10000/10001/10002`
- starts Rust blob/queue/table binaries on `11000/11001/11002`
- sends identical authenticated HTTP requests to both implementations
- compares status, normalized headers, and normalized XML/JSON/binary bodies
- prints PASS/FAIL results per scenario and preserves logs/state when requested

**Why:** Existing test coverage proves each stack against its own expectations, but not against each other on the same wire inputs. Differential testing gives QA a direct parity oracle: if TS returns X for input Y, Rust must return the same X.

**Implementation decisions:**
1. Use the per-service Rust binaries instead of the unified Rust binary because `rust/scripts/run-integration-tests.sh` already documents unresolved unified startup failures.
2. Start TypeScript directly from source with `ts-node` rather than relying on a globally installed `azurite` package.
3. Normalize HTTP framing noise (`Connection`, `Keep-Alive`, chunked/content-length differences) so the harness focuses on service-contract differences.
4. Normalize XML/JSON dynamic fields (`requestId`, queue pop receipts, body timestamps, etc.) while still keeping meaningful parity failures such as ETag mismatches, extra headers, status drift, and unexpected response fields.

**First run findings:**
- Queue scenarios reached parity for create queue, put message, and get messages.
- List containers and query entities matched semantically.
- Blob responses still diverge on ETag values across multiple operations.
- Snapshot response adds Rust-only `x-ms-request-server-encrypted`.
- Copy blob differs materially: TS returned `501 APINotImplemented`, Rust returned `500` with an empty body.
- Table create response differs because Rust returns extra `preferenceApplied` and `version` fields; insert entity still diverges on ETag.

**Impact:** This harness should become the default QA check for cross-stack parity regressions and for verifying future Rust fixes against the TypeScript behavior.