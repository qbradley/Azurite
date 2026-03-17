# Squad Decisions

## 2026-03-13: Porting Philosophy Established
**By:** Quetzal Bradley (via Copilot)
**What:** The Rust port of Azurite must prioritize fidelity with the TypeScript source over idiomatic Rust. No performance optimization. The primary goal is easy propagation of future TS changes to the Rust port.
**Why:** User directive — foundational project constraint.

## 2026-03-13: Porting Database Required
**By:** Quetzal Bradley (via Copilot)
**What:** A `porting-db/` directory will maintain per-file records of all decisions, transformations, substitutions, and notes for every source file in the TypeScript codebase. This is essential for future change propagation.
**Why:** User directive — core infrastructure for the port.

## 2026-03-13: Rust Code Location
**By:** Quetzal Bradley (via Copilot)
**What:** All Rust port code lives in the `rust/` subdirectory at the repository root.
**Why:** User directive — project structure decision.

## 2026-03-13: Fidelity Over Idiom
**By:** Quetzal Bradley (via Copilot)
**What:** Favor keeping fidelity with the TypeScript source rather than idiomatic Rust. Do not prioritize performance or use Rust tricks. The code should be a near-direct translation.
**Why:** User directive — ensures future TS changes can be mechanically propagated to Rust.

## 2026-03-13: Porting Strategy Established
**By:** Gandalf (Lead Architect)
**What:** Comprehensive porting strategy for Azurite TypeScript → Rust defined in `porting-db/STRATEGY.md` (1100 lines) and `porting-db/PORTING-ORDER.md` (424 lines). Covers type mappings, async strategy (Tokio + axum), dependency mapping, module organization, API translation, middleware patterns, error handling, and per-file porting database format.
**Decisions:** 8 critical architectural decisions documented:
- D-004: Tokio as async runtime (required by axum)
- D-005: axum as Express replacement (Tower-based, async-native)
- D-006: No autorest re-generation (manual translation preserves 1:1 structure)
- D-007: Custom in-memory store (HashMap+RwLock, NOT LokiJS port)
- D-010: Flatten intersection types (TS `A & B & C` → single Rust struct)
- D-013: VS Code extension out of scope (JS-only APIs)
- D-015: Blob first, queue second, table third (blob establishes patterns)
- D-016: Composition over inheritance (embed base struct as field)
**Why:** Required to guide implementation. All team members must review STRATEGY.md before starting work.
**Governance:** 17-phase porting schedule. Per-file YAML records in `porting-db/src/` mirror TS source tree.

## 2026-03-13: Rust Workspace Scaffold Rooted in `rust/`
**By:** Aragorn (Rust Expert)
**What:** Scaffolded the Rust port as a Cargo workspace rooted at `rust/`, with 5 service/library crates under `rust/crates/` and porting database under `rust/porting-db/`. Workspace members: azurite, azurite-common, azurite-blob, azurite-queue, azurite-table. Shared dependency versions centralized in `[workspace.dependencies]` at workspace root. Workspace compiles with `cargo check` from `rust/` directory. Phase 0 scaffolding complete.
**Why:** Keeps every Rust artifact under single root, matches Gandalf's planned crate layout, facilitates future TS→Rust propagation. Service crates expose both lib.rs and service binary in main.rs to stay structurally parallel to TypeScript entry points.

## 2026-03-13: Move porting-db/ to rust/porting-db/
**By:** Gandalf (Lead Architect)
**What:** Moved porting database from repository root to `rust/porting-db/`. All path references in STRATEGY.md and PORTING-ORDER.md updated (`porting-db/` → `rust/porting-db/`). Template examples in §16 (Record Format) now reference correct paths. New structure: rust/porting-db/ contains STRATEGY.md, PORTING-ORDER.md, README.md, and per-file YAML records under src/ mirroring TS structure.
**Why:** All Rust artifacts (including documentation and porting records) live under single `rust/` root except `.squad/` team metadata. Improves directory hygiene, clarifies change propagation boundaries, makes porting database first-class artifact of Rust port rather than separate root-level artifact. When Aragorn implements Rust module, he can immediately reference strategy docs and per-file records without crossing TypeScript/Rust boundary.

## 2026-03-13: Phase 1 Analysis Preserves Naming and Model Inconsistencies
**By:** Faramir (TypeScript Expert)
**What:** Decided to preserve TS source naming and model inconsistencies in Rust porting plan rather than normalizing them. Keep `contextID`/`contextId` differences documented. Treat `src/common/persistence/IExtentMetadata.ts` and `src/common/persistence/IExtentMetadataStore.ts` as distinct contracts with separate extent models (`persistencyId`/`LastModifyInMS` vs `locationId`/`lastModifiedInMS`). Do not collapse them without explicit compatibility layer.
**Why:** These differences are real TypeScript source behavior, not noise. Normalizing in port would make future TS change propagation harder and could hide compatibility-sensitive behavior like Loki metadata field bridging. Preserves exact TS semantics for accurate long-term propagation.
**Key Concerns Flagged:** (1) `IOperationQueue.operate<T>()` is generic and not object-safe as trait object in Rust — may need concrete implementation or non-object-safe trait pattern. (2) `IEnvironment` aggregates three service traits with overlapping method names, may push Rust port toward flattened config type for ergonomics while still preserving TS semantics. (3) `IServerFactory` abstraction narrower than current TS implementations — translation should preserve abstraction without assuming every factory directly implements it.

## 2026-03-13: Phase 1 IEnvironment Translation Pattern
**By:** Aragorn (Rust Expert)
**What:** Phase 1 ports `src/common/IEnvironment.ts` as a flattened local `IEnvironment` trait inside `azurite-common` instead of a Rust supertrait over `IBlobEnvironment`, `IQueueEnvironment`, and `ITableEnvironment`.
**Why:** The Cargo workspace dependency direction is `azurite-{blob,queue,table} -> azurite-common`, so `azurite-common` cannot reference traits that will live in service crates without creating a cycle. Flattening the aggregate interface preserves the TypeScript-visible method surface while keeping the common crate buildable.
**Follow-up:** When service-specific environment traits are ported, they should align their method signatures and either implement the flattened common trait directly or introduce adapter wrappers if a more literal hierarchy becomes practical.

## 2026-03-13: Per-Crate Test Infrastructure With Ignored Placeholders
**By:** Boromir (QA Expert)
**What:** Rust parity tests organized in per-crate integration test trees mirroring the TypeScript suite:
- `azurite-common/tests/common/`
- `azurite-blob/tests/blob/`
- `azurite-queue/tests/queue/`
- `azurite-table/tests/table/`

For modules not yet translated, placeholder tests are marked `#[ignore]` instead of documented only in prose.
**Why:** Keeps Rust port aligned with TS suite structure, makes parity work discoverable, lets `cargo test` compile and verify pending suites as codebase evolves. Allows incremental replacement of ignored placeholders without workspace reorganization.
**Impact:** Aragorn and future QA passes can expand parity test coverage incrementally as translations complete.

## 2026-03-13: Copy ZERO_EXTENT_ID into azurite-common (D-008)
**By:** Aragorn (Rust Expert)
**What:** Copy `ZERO_EXTENT_ID = "*ZERO*"` into `azurite-common::persistence::ZERO_EXTENT_ID` instead of importing it from the future blob crate.
**Why:** `MemoryExtentStore` and `FSExtentStore` are Phase 2 common-layer ports, while the original TypeScript constant lives under blob persistence. Keeping the exact value in `azurite-common` preserves the TS blob-layer leakage Faramir flagged without creating a Rust crate cycle.
**Impact:** Future blob-layer ports should reuse this common constant or explicitly bridge back to the blob contract so the same sentinel string remains visible at both layers.

## 2026-03-13: Preserve Account SAS Sentinel Enum Members as Validation-Only (D-009)
**By:** Faramir (TypeScript Expert)
**What:** Keep `AccountSASPermission.Any = "AnyPermission"` and `AccountSASResourceType.Any = "AnyResourceType"` as explicit sentinel variants/constants in the Rust port. Do **not** allow the normal Phase 3 parser/serializer helpers to accept or emit those values.
**Why:** In TS, `AccountSASPermissions.parse()/toString()` and `AccountSASResourceTypes.parse()/toString()` only handle canonical one-character account-SAS values. The `Any*` members are consumed later by blob batch authorization checks in `src/blob/authentication/OperationAccountSASPermission.ts`. Preserving them as validation-only sentinels maintains TS fidelity and prevents future serialization bugs.
**Impact:** Aragorn's Phase 3 port must map these as enum variants/constants but exclude them from canonical serialization. Serialization order sensitivity (`rwdxlacuptfiy`, `btqf`, `sco`) is contract-critical and must be replicated exactly.

## 2026-03-13: Preserve Account-SAS IP Range Type Asymmetry (D-010)
**By:** Faramir (TypeScript Expert)
**What:** Keep an explicit compatibility layer between account-SAS `SasIPRange | string` and the local `IIPRange` formatter logic instead of silently collapsing them into one undocumented Rust type.
**Why:** `src/common/authentication/IAccountSASSignatureValues.ts` intentionally imports `SasIPRange` from `@azure/storage-blob`, while blob/queue/table service SAS interfaces import local `IIPRange`. The current TS code relies on structural compatibility plus shared formatting; future TS changes could split those shapes. Preserving the asymmetry with an explicit layer makes the coupling visible and tractable.
**Impact:** Aragorn's Phase 3 port should avoid auto-collapsing these types. Document the adapter explicitly so future maintainers see the boundary.

## 2026-03-13: Execute Phase 1 Parity Coverage with Fixtures Even for Incomplete Implementations (D-011)
**By:** Boromir (QA Expert)
**What:** Activate Phase 1 parity coverage with executable Rust fixture-based contract tests for translated interfaces even when the matching concrete runtime implementation is not fully ported yet. Keep `#[ignore]` placeholders only for concrete TypeScript behaviors that truly have no Rust equivalent yet.
**Why:** This gives immediate protection against interface drift and casing/type mismatches without pretending stub structs are already behaviorally complete. It also keeps the remaining parity backlog explicit: `AccountDataStore` env parsing/refresh, `ConfigurationBase` helper methods, service-specific request listener/server factories, and Phase 2 extent roundtrip behavior.
**Impact:** QA can unignore concrete parity tests incrementally as Aragorn lands implementations, while translated Phase 1 contracts are already covered by runnable tests. This keeps Boromir's parity rule enforceable earlier in the port instead of waiting for every concrete service implementation.

## 2026-03-13: Phase 4 Translation — IEnvironment Rust Contract Refinement
**By:** Aragorn (Rust Expert)
**What:** Updated common `IEnvironment` Rust trait to precisely match TypeScript Phase 4 surface. `debug()` now returns `Option<String>` (preserves undefined → None mapping), and `extentMemoryLimit()` returns `Option<f64>` (preserves parseFloat/NaN behavior from `Environment.ts` flowing into `ConfigurationBase::setExtentMemoryLimit()`).
**Implementation Detail:** Represented duplicate `disableProductStyleUrl` registration in clap builder by registering argument once, then mutating with alternate help text. Keeps duplicate registration visible in source without asking clap to accept same long flag twice.
**Why:** Exact TypeScript semantics preservation; Phase 4 utilities depend on these nullable returns for configuration logic.

## 2026-03-13: Continuous Pipeline Execution Directive
**By:** Quetzal Bradley (via Copilot)
**What:** Each batch completion automatically triggers the next phase. No pauses between scheduled phases. Continuous execution until all phases are complete.
**Why:** User directive (2026-03-13T23:08) — maximize throughput and maintain momentum on the port. Auto-launch next phase immediately upon batch completion.

## 2026-03-13: Phase 5 Blob Generated Framework Translation (D-012)
**By:** Aragorn (Rust Expert)
**What:** Represent autorest-generated blob framework metadata (`parameters`, `mappers`, `specifications`) as generated JSON snapshots loaded by Rust modules. Use generic carrier types (`GeneratedObject`, `GeneratedResponse`, `GeneratedValue`) to preserve model name fidelity while deferring concrete handwritten struct expansion to business logic phases.
**Constraints:** Six-stage middleware pipeline explicit and ordered exactly as TypeScript. `Operation`, `specifications`, `handlerMappers` coupled by enum order. Concrete scalar inputs (`string`, `number`, `boolean`, stream) narrowed at boundaries; large object families stay generic.
**Why:** Snapshot-backed metadata keeps translation mechanical and auditable. Easy to refresh after future autorest regenerations. Allows middleware/handler wiring to compile before downstream business logic phases.
**Impact:** Phase 5 generated framework complete. 34 files under `rust/azurite-blob/src/generated/`. Supports Phase 6-7 error/context/auth analysis.

## Governance

- All porting decisions must be recorded in `rust/porting-db/`
- Architecture decisions require Gandalf's approval
- API-facing changes require Samwise's approval
- TS fidelity is reviewed by Faramir

## 2026-03-14: Phase 6-7 Blob Errors/Auth Preservation
**By:** Aragorn (Rust Expert)
**What:** Preserved existing blob-authentication quirks in Rust instead of normalizing them during Phase 6-7 port. Kept `StorageError`/`StorageErrorFactory` wire behavior mechanically close to TypeScript, including eager XML body construction, permissive SAS authorization rules (ANY-character matching), `BlobSnapshot` routing through container permission table, and loose BASIC-token path in `BlobTokenAuthenticator`.
**Why:** These quirks are part of the observable Azurite contract today. Normalizing in Rust would make future TypeScript change propagation harder and could silently diverge authentication/error behavior.

## 2026-03-14: Phase 5 Generated Framework Parity Strategy
**By:** Boromir (QA Expert)
**What:** Generated blob framework parity protected by metadata-driven contract tests rather than hand-picked spot checks. Use generated metadata snapshots (`operations.generated.json`, `handler_mappers.generated.json`, `handler_interfaces.generated.json`) as executable parity fixtures for routing and handler-surface coverage. Keep direct behavioral tests for middleware/context/serializer wire format, but let snapshot metadata verify full autogenerated surface area so future autorest regenerations are caught mechanically.
**When isolating from unrelated local edits:** Run final `cargo test --workspace` in clean temporary worktree based on HEAD and copy only Boromir-owned files, keeping verdicts isolated from shared environment edits.
**Why:** Phase 5 is largely framework glue generated from swagger metadata. Snapshot-backed tests give broader coverage with less manual drift.

## 2026-03-14: Phase 8-10 Analysis Scope Clarification
**By:** Faramir (TypeScript Expert)
**What:** Analyzed Phase 8 (blob lease subsystem) + Phase 10 (blob persistence) together to satisfy Quetzal's requested "Phase 8 and Phase 9" pairing while keeping Aragorn ahead on files Quetzal explicitly called out (lease/persistence).
**Fidelity flags for Phase 8 implementation:** (1) Lease timing is lazy and factory-driven; no background timer should be introduced. (2) `LeaseExpiredState.renew()` ignores the caller-supplied lease ID. (3) Refer to persistence as **Phase 10** in future coordination unless phase numbering is re-baselined.
**Fidelity flags for Phase 10 implementation:** (1) `QueryParser` documents unary `not` but does not implement it. (2) `LokiBlobMetadataStore` normalizes snapshot lease state/status to `Available/Unlocked` despite TODOs. (3) `setBlobTag()` ignores `modifiedAccessConditions`.
**Why:** Preserves exact TS semantics and keeps team coordination clear on phase boundaries.

## 2026-03-14: Phase 9-10 Translation Decisions
**By:** Aragorn (Rust Expert)
**What:**

1. **D-FilterBlobModel-Concrete:** Made `FilterBlobModel` a concrete struct with typed fields (`name: String`, `containerName: String`, `tags: Option<BlobTags>`) rather than aliasing to `FilterBlobItem` (GeneratedObject). Conditions validators and query interpreter need typed access to these fields. The TS type `FilterBlobModel = FilterBlobItem` is actually just `{ name, containerName, tags }` in practice.

2. **D-IQueryNode-TraitObject:** Query AST nodes use `Box<dyn IQueryNode>` trait-object dispatch, consistent with Phase 8's `ILeaseState` pattern. `BinaryOperatorNode` is a concrete struct (not a trait) that holds `left`/`right` children; each comparison node (Equals, GreaterThan, etc.) wraps it via composition.

3. **D-IBlobMetadataStore-Expanded:** Expanded the `IBlobMetadataStore` trait from Phase 8's minimal 2-method surface to the full ~40 method interface matching TypeScript. Added `leaseAccessConditions` parameter to `getContainerACL()`, breaking 2 existing callers (fixed in same changeset). This is a backward-incompatible trait change but necessary for fidelity.

4. **D-QueryParser-NotGrammar-Preserved:** The TS `QueryParser` documents a grammar production for unary `not` but the `visitUnary()` method never consumes the `not` keyword. Preserved this exact behavior — the Rust parser skips `visitUnary` to `visitExpressionGroup` without checking for `not`.

5. **D-PageWithDelimiter-InsertionOrder:** Used `Vec<String>` + `BTreeSet<String>` to track prefix insertion order, preserving TS `Set` iteration semantics (insertion order). A plain `BTreeSet` alone would give sorted order, which happens to be equivalent for sorted input but the dual tracking makes the fidelity explicit.

**Why:** These decisions preserve TS fidelity while adapting to Rust's type system. Each was chosen to minimize behavioral divergence from the TypeScript source per project directive.

**Status:** ACTIVE
**Scope:** Phase 9 conditions + Phase 10 persistence (excluding 10.7 LokiBlobMetadataStore)

## 2026-03-14: Clippy + Fmt Mandatory Pre-Commit Directive
**By:** Quetzal Bradley (via Copilot)
**What:** Before committing any Rust changes, all agents must run `cargo clippy --all-targets` and fix all clippy issues, then run `cargo fmt`. This is a mandatory pre-commit hygiene step for all agents.
**Why:** User request — maintains code quality and consistent formatting across the entire Rust port.
**Status:** ACTIVE
**Scope:** All Rust commits

## 2026-03-16: TS→Rust Parity Analysis Complete — 34+ Bugs Identified Across 5 Categories
**By:** Gandalf (Lead Architect)
**Date:** 2026-03-17
**Status:** COMPLETE
**What:** Deep-dive analysis of Azurite TS→Rust port revealed structural completion (174 Rust files, 1031 tests passing, 49 blob handlers) but identified five systemic categories of bugs during integration testing and customer validation.

**Bug Categories Found:**
1. **Category A: Concurrency/Atomicity (6 bugs)** — TypeScript's single-threaded event loop makes read-then-modify-then-write sequences inherently atomic. Rust with Arc<RwLock> requires explicit atomicity. Examples: `uploadPages` (31% data loss), `clearRange`, `appendBlock`, `resizePageBlob`, `updateSequenceNumber`, `commitBlockList`. Fix pattern: single write lock across entire operation.

2. **Category B: Serialization/Wire Format (8+ bugs)** — Divergences between ms-rest-js/xml2js (TS) and serde_json/quick_xml (Rust). Examples: RFC 1123 dates, xmlIsWrapped arrays, self-closing XML elements, field name casing, sequence body serialization. Root cause: behavioral contracts embedded in runtime libraries, not REST API spec.

3. **Category C: Semantic/Business Logic (12+ bugs)** — Subtle interaction ordering and side effects. Examples: condition check ordering (412 before 404), lease preservation on overwrite, ETag/lastModified updates, snapshot lease clearing, source conditional header remapping. Found by handler-by-handler audit; zero caught by existing tests.

4. **Category D: Infrastructure/Initialization (5+ bugs)** — Node.js ambient capabilities (lenient base64, process lifecycle, env var auto-initialization) require explicit Rust implementation. Examples: AccountDataStore.init() not called, auth middleware loop short-circuit, base64 decoding leniency, batch sub-request context, SIGHUP crash, getBlobType() stub.

5. **Category E: Missing Feature Stubs (3+ bugs)** — Autorest-generated metadata requires runtime patching. Table MERGE verb, isXML overrides, empty specifications.

**Root Cause Analysis:** Impedance mismatches between TypeScript single-threaded semantics and Rust explicit concurrency. Line-by-line translation produces mechanically correct code but fails to recognize implicit atomicity requirements in TS become explicit in Rust.

**Test Coverage Gap:** Existing TS test suite is entirely sequential (zero concurrent operations). All 34+ bugs escaped detection because race conditions, serialization format details, semantic interaction ordering, and infrastructure initialization are untested in the TS suite.

**Strategic Recommendation — Differential Testing Harness as #1 Priority:** Build test runner that (1) starts both TS and Rust servers, (2) replays 824 TS test requests against both, (3) captures raw HTTP responses, (4) reports differences in status/headers/body. Would have caught Categories B and C automatically. Estimated 2-3 weeks effort.

**Additional Priorities:**
- Concurrency stress tests for all metadata store methods (would catch all Category A)
- Fix table store TOCTOU races (insertOrUpdateTableEntity, insertOrMergeTableEntity)
- Fix 8 SAS cross-account test failures (cross-account copy validation)
- Multi-SDK testing (Python + .NET SDKs against Rust server)

**Why This Matters:** Port achieved structural completion and happy-path functionality, but correctness in Rust requires approaches beyond line-by-line translation. Every bug category maps to testing strategies that can be systematized to prevent future issues.

**Documentation:** Full analysis with code examples, root cause deep-dives, and 8-phase remediation roadmap written to `decisions/inbox/gandalf-parity-analysis.md` (5800+ lines).

## 2026-03-16: Azure Storage REST API Protocol Parity Gap Analysis
**By:** Samwise (Azure Storage REST Expert)
**Date:** 2026-03-16
**Status:** COMPLETE
**What:** Wire-format, authentication, error format, and protocol completeness analysis identifying divergences between TS and Rust implementations.

**Critical Issues (2):**
1. **XML Declaration Missing** — Every XML response differs at the start. xml2js Builder emits `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` by default; quick_xml does not. Impact: byte-level differential tests fail on every XML response. Fix: prepend declaration in stringifyXML() — 1 day effort, high impact.

2. **Cross-Account SAS Copy Validation Unimplemented** — 8 failing tests in blob/sas.test.ts (lines 1841–2200). validateCopySource() accepts _sourceAccount but never validates SAS tokens, looks up source account keys, or checks public access fallback. Requires: multi-account key lookup, SAS signature validation, public access check, archive tier check. Fix: 1-2 weeks, only functional gap causing real failures.

**Moderate Issues (5):**
1. Error body whitespace (pretty vs compact XML)
2. Blob error responses missing x-ms-version header (matches TS, diverges from Azure)
3. maxresults ≤ 0 not validated
4. No version-conditional behavior (shared with TS, design choice)
5. Content-ID typing differs across blob/table

**Minor Issues (4):**
1. serde_json Map ordering may differ for additionalProperties
2. Rust accepts LF, TS requires CRLF in batch operations
3. ETag entropy source differs (nanos vs Math.random) — cosmetic
4. Copy from external non-Azure URLs untested

**Authentication Parity — CONFIRMED:**
- SharedKey validation identical (13-line canonical format, header sorting, signature computation)
- Account SAS serialization order confirmed (`rwdxlacuptfiy`, `btqf`, `sco`)
- Blob/Queue/Table SAS canonical names and signing fields match
- OAuth/Bearer token validation matches

**Protocol Areas at Risk (8+):**
- Version-specific response behavior (2009-04-14 through 2025-11-05 all produce identical responses)
- Copy operations with complex sources
- Pagination edge cases (maxresults=0, very large result sets, concurrent modifications)
- Batch operation edge cases (Content-ID typing, LF vs CRLF, malformed sub-requests)
- Conditional headers interaction (If-Match with stale ETag, multiple conditions combined)
- Large blob operations (50k blocks, 195GB append blobs, sparse page blobs)
- CORS preflight with complex rules
- Service properties (logging, metrics, static website, default version)

**Recommended Fixes (Priority Order):**
1. XML Declaration — add to stringifyXML() across blob, queue, table crates (1 day, highest impact)
2. Error Whitespace — either use jsonToXML or document deviation (1 day)
3. Cross-Account SAS — implement validateCopySource() properly (1-2 weeks, functional)
4. Differential Testing — proxy-based dual-server testing comparing responses semantically (eliminates false positives from formatting, catches real behavioral differences)

**Recommended Testing Strategy:**
- Proxy-based differential testing with both servers running
- SDK compatibility matrix: @azure/storage-blob (P0), Azure.Storage.Blobs/.NET (P0), Python SDK (P1), Java SDK (P1), Go SDK (P2)
- OpenAPI spec compliance scanning from azure-rest-api-specs
- Protocol fuzzing (headers, XML bodies, SAS tokens, pagination tokens)

**Documentation:** Full analysis with tables, authentication deep-dives, untested protocol catalog, and differential testing strategy written to `decisions/inbox/samwise-protocol-parity.md` (2500+ lines).

**Key Insight:** Rust port achieves strong structural fidelity (serialization pipeline, auth machinery, handler architecture faithfully translated). Wire-format divergences are cosmetic to SDKs (which parse semantically) but matter for strict protocol compliance. The #1 fix (XML declaration) is mechanically simple and eliminates most visible difference across every XML response.

## 2026-03-16: XML Declaration Parity Added (D-XML-Declaration)
**By:** Aragorn (Rust Implementation Expert)  
**What:** Rust serializers now emit `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` as the first line of every XML response, matching TS `xml2js.Builder` default behavior. Added to blob, queue, and table `stringifyXML()`/`jsonToXML()` implementations.  
**Why:** TS xml2js emits this declaration by default on every XML response (blob GetBlobProperties, ListBlobs, SnapshotBlob, GetBlockList; queue GetQueueAcl, PeekMessages, GetMessages; table ListTables, QueryEntities, BatchTransaction). Rust quick_xml does not include it by default. Without this declaration, every XML response differs at the byte level from TS, breaking naive differential tests even when semantics match.  
**Status:** ✅ IMPLEMENTED & TESTED (1031 tests passing)  
**Impact:** Eliminates XML response header mismatch across all three services. Highest-impact Rust parity fix (addresses Samwise's #1 recommended priority).

## 2026-03-16: Cross-Account SAS Copy-Source Validation Preserves Raw Query String (D-Copy-Source-Raw)
**By:** Aragorn (Rust Implementation Expert)  
**What:** `copyBlob` handler now preserves the raw SAS query string when appending `comp=metadata` for cross-account source validation. Instead of reconstructing the entire source URL and risking query normalization, the handler appends the metadata comparison flag directly to the raw query retained from the original source header.  
**Why:** SAS tokens are part of externally supplied auth material and must be validated without modification. TS `URLBuilder` preserves the query string exactly as provided; reconstructing it could normalize or lose details (encoding, parameter order, special characters). Preserves fidelity with TS validation flow and avoids validation-specific URL drift.  
**Status:** ✅ IMPLEMENTED & TESTED (1031 tests passing)  
**Impact:** Cross-account copy now validates source SAS tokens correctly without reshaping the auth material.

## 2026-03-16: Table Upsert/Merge TOCTOU Race Fixed With Atomic Lock (D-Table-Upsert-Race)
**By:** Aragorn (Rust Implementation Expert)  
**What:** `insertOrUpdateTableEntity` and `insertOrMergeTableEntity` handlers now use a single atomic write-lock scope to read the current entity snapshot and perform the mutation/insert, eliminating the TOCTOU (Time-of-Check-Time-of-Use) race window between separate existence queries and updates.  
**Why:** Separate query-then-mutate paths allow concurrent operations to interleave: Thread A queries entity (doesn't exist) → Thread B inserts → Thread A inserts over Thread B's result. The fix applies the same atomic pattern used in blob `commitBlockList` race: all collection reads and mutations happen within one write lock scope. Preserves exact semantics (read-snapshot-and-mutate is atomic from the map's perspective) while preventing races.  
**Status:** ✅ IMPLEMENTED & TESTED (1031 tests passing)  
**Impact:** Table upsert/merge operations now race-free and deterministic under concurrent load.

## 2026-03-16: Differential Testing Harness Deployed (D-Differential-Test-Harness)
**By:** Boromir (QA Expert)  
**What:** Added standalone differential test harness at `rust/scripts/differential-test.sh` (shell orchestrator) and `rust/scripts/differential_test.py` (HTTP client + comparison engine). Harness simultaneously:
- Starts TS blob/queue/table services on ports 10000/10001/10002
- Starts Rust service binaries on ports 11000/11001/11002
- Sends identical authenticated HTTP requests to both stacks
- Compares status codes, normalized headers, normalized bodies (XML/JSON)
- Reports PASS/FAIL with diff details per scenario

Normalization removes HTTP framing noise (Connection, Keep-Alive, chunked vs content-length) while preserving meaningful parity failures (ETag mismatches, extra headers, status drift, unexpected response fields).  
**Why:** Existing test suites validate each stack against its own expectations, but not against each other on identical wire inputs. Differential testing provides a direct parity oracle: if TS returns X for input Y, Rust must return the same X. This harness decouples implementation testing from parity validation, enabling rapid iteration on remaining divergences.  
**Status:** ✅ HARNESS DEPLOYED & OPERATIONAL (first run: 5 pass, 11 fail)  
**First-Run Results:**
- PASS: Queue create, put message, get messages; generic list containers, query entities
- FAIL: Blob ETag divergences, Rust adds extra headers (x-ms-request-server-encrypted), copy blob status codes differ (TS 501 vs Rust 500), table create adds preferenceApplied/version fields, table insert entity ETag mismatch

**Impact:** Established QA validation layer for cross-stack parity regression detection. Harness enables fast iteration: fix (Aragorn) → re-run (Boromir) → next fix cycle.

**Next:** Re-run harness against Aragorn's XML/copy/upsert fixes to validate improvements.


## 2026-03-16: Boromir Differential Scorecard (D-Differential-Scorecard-Validated)
**By:** Boromir (QA Expert)  
**Date:** 2026-03-16T22:03:51Z  
**Status:** COMPLETE

### Summary
Verified Rust release binaries, rebuilt x86_64 release target, re-ran differential harness, fixed Rust-only parity bugs, and revalidated port with clippy, fmt, harness, integration runner, and SDK suite.

### Binary Verification
- Verified `rust/target/x86_64-unknown-linux-gnu/release/azurite` exists
- Rebuilt with `cargo build --release --target x86_64-unknown-linux-gnu`
- Re-ran harness; score stayed at **5 pass / 11 fail** after Aragorn's XML declaration fix

### Boromir Fixes Applied
1. **Blob snapshot parity:** Removed Rust-only `x-ms-request-server-encrypted` response header from `createSnapshot`
2. **NotImplemented wire parity:** Updated generated error middleware to unwrap `NotImplementedError` / `NotImplementedinSQLError` wrappers instead of downgrading to generic 500 responses
3. **Table create payload parity:** Made create-table emit explicit JSON body so header-only response metadata no longer appears in payload
4. **Follow-through:** Applied same NotImplemented middleware handling to queue/table generated middleware

### Differential Test Scorecard
```
Before Fixes:           5 pass / 11 fail
After Aragorn's Fix:    5 pass / 11 fail  
After Boromir's Fixes:  6 pass / 10 fail
```

### Remaining Gaps (Post-Fix)
- **ETag mismatches (7 scenarios):** Header differences on container create, block blob operations, page blob, leases, snapshot, blob metadata, table insert entity
- **Dynamic message suffixes (1 scenario):** Copy blob error message embeds request ID and timestamp
- **No status code gaps** after middleware fix

### Analysis
Remaining differential failures are dominated by **dynamic identity fields** (ETags, timestamps, request IDs), not deterministic Rust logic bugs. Next QA step should normalize dynamic fields in harness rather than forcing Rust to emit independently-generated values.

### Validation Results
- `cargo clippy --all-targets` ✅
- `cargo fmt --all` ✅
- `cargo build --release --target x86_64-unknown-linux-gnu` ✅
- `bash scripts/run-integration-tests.sh` ✅
- `cargo test -p azurite-integration-tests -- --test-threads=1` ✅ (33 SDK tests passing)

### Recommendation
Normalize dynamic response fields in harness before comparison. This keeps the harness aligned with parity rules: ignore only values that cannot be equal across independent servers, but do not hide genuine wire-contract differences.

---

## 2026-03-16: Harness Normalization & Dynamic Field Elimination (D-Harness-Normalization)
**By:** Boromir (QA Expert)  
**Date:** 2026-03-16T22:39:00Z  
**Status:** COMPLETE

### Decision
Normalize dynamic response fields in differential harness before comparison, keeping true payload-shape differences visible.

### Normalization Rules Adopted
- Normalize `etag`, `x-ms-request-id`, `Date` response headers before value comparison
- Normalize XML `<RequestId>`, `<Time>`, ETag text, and dynamic `RequestId:` / `Time:` message suffixes
- Normalize JSON/XML server-generated timestamp and queue-ID fields where expected to vary per server instance
- Compare `Content-Length` semantically: pass if normalized bodies match even if raw byte count differs due to dynamic content
- **Preserve:** `List Blobs` root-metadata shape mismatches as real bugs (not normalized away)

### Why
Keeps harness aligned with parity rule: ignore only values that cannot be equal across independent servers, but do not hide genuine wire-contract differences. The remaining `List Blobs` failure is therefore actionable and should be fixed in Rust serializer.

### Outcome
After normalization pass, scorecard improved from **6 pass / 10 fail** to **23 pass / 1 fail** (8 new scenarios added, 18 fixed by normalization).

### Identified Real Bug
**List Blobs XML Attributes:** Rust emits `EnumerationResults` root metadata (`ContainerName`, `ServiceEndpoint`, etc.) as child XML elements instead of XML attributes on the root element. This is a genuine wire-protocol divergence that the harness correctly surfaces and does not normalize away.

### Impact
Established repeatable, maintainable harness validation layer. Scorecard now reflects true parity gaps (List Blobs serializer bug) rather than cosmetic differences. Harness ready for rapid iteration: fix → re-run → next fix.

---

## 2026-03-16: Aragorn Markdown File Organization (D-Markdown-Org)
**By:** Aragorn (TS-to-Rust Migration Lead)  
**Date:** 2026-03-17T01:04:00Z  
**Status:** COMPLETE

### Decision
Centralize all team markdown note files into `rust/porting-db/notes/{topic-slug}/NOTE.md` pattern.

### Context
19 markdown note files were scattered in `rust/` root directory and project home directory, reducing discoverability and violating repository organization standards.

### Action
Relocated all notes into organized `rust/porting-db/notes/` directory structure with descriptive topic slugs.

### Why
- Maintains coherent repository structure
- Improves discoverability for team members
- Keeps home and rust root directories clean
- Establishes clear convention for future documentation

### Scope
- File cleanup completed in commit 3a5bd0b1
- Standard now applies to all future note creation

---

## 2026-03-17: Team Directive on Note Organization (D-Note-Directive)
**By:** Quetzal Bradley (Project Owner)  
**Date:** 2026-03-17T00:33:00Z  
**Status:** ACTIVE

### Directive
Notes created by the team MUST go in `rust/porting-db/notes/{topic-slug}/NOTE.md` where `{topic-slug}` is a descriptive topic identifier and NOTE is a descriptive filename.

### Prohibition
Do NOT put loose markdown files in `rust/` root or the home directory.

### Why
- User request for repository organization
- Enables easy team navigation
- Prevents clutter in primary directory structures
- Captured for team memory

---
