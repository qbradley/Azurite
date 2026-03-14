# Team Decisions

## D-001: Account-SAS Compatibility Structure (Aragorn — Phase 3)
**Status:** ACTIVE

Keep account-SAS `SasIPRange` as a distinct compatibility struct in `rust/crates/azurite-common/src/authentication/i_account_sas_signature_values.rs` and only convert it into local `IIPRange` at serialization time.

**Rationale:** Preserves Faramir's D-010 constraint that account-SAS intentionally names the external SDK-shaped type while the shared formatter lives on the local `IIPRange` helper. Making the boundary explicit keeps future TS changes visible instead of silently collapsing the two contracts.

**Follow-up:** Phase 3 carries local `computeHMACSHA256` and `truncatedISO8061Date` helpers inside the same module so account-SAS signing can compile before Phase 4 `utils.rs` is fully ported. When Phase 4 lands, consolidate those helpers without changing signature bytes or the 2015/2020 string-to-sign layouts.

---

## D-002: Preserve Observable Phase 4 Quirks (Faramir — Phase 4)
**Status:** PENDING_APPROVAL

Preserve observable Phase 4 quirks unless explicitly approved otherwise during Rust translation.

**Quirks Identified:**
1. **Telemetry.ts persistence/privacy quirks**
   - `GetInstanceID()` persists and reads the misspelled JSON key `instaceID`.
   - `GetRequestUri()` uses `hostname in knownHosts`, so local-host redaction never triggers because the JS `in` operator checks array indices, not values.

2. **Environment.ts CLI-registration quirk**
   - `--disableProductStyleUrl` is registered twice in the `args` option chain.

3. **WinstonLoggerStrategy.ts formatting quirk**
   - Missing `contextID` defaults to a tab character (`"\t"`), which affects literal log output.

**Rationale:** These are exactly the kinds of details that future TS changes can touch. If the Rust port quietly normalizes them now, future change propagation becomes ambiguous: maintainers will not know whether a later TS fix is genuinely new behavior or something Rust already diverged on.

**Handling:** Treat these as compatibility-sensitive. Aragorn should preserve them in structure/notes or add an explicit compatibility layer, not silently "fix" them while porting.

**Awaiting:** Gandalf/Samwise approval on whether to preserve these behaviors or normalize them during Phase 4 implementation.

---

## D-003: Phase 8 Lease Subsystem Translation (Aragorn — Phase 8)
**Status:** ACTIVE

### D-ILeaseState-ObjectSafety
`ILeaseState` is exposed as a `Box<dyn ILeaseState>` trait object.  The TypeScript
interface's generic `sync<T>(syncer: ILeaseSyncer<T>): T` method is omitted from the
Rust trait because generic methods are not object-safe.  Instead, a `lease() -> &ILease`
method is added to the trait; callers call `syncer.sync(state.lease())` directly, which
preserves identical semantics.  `ILeaseSyncer<T>` and `ILeaseValidator` remain as regular
(non-object-safe) traits used at concrete call sites.

**Rationale:** Maintaining `Box<dyn ILeaseState>` as the return type of all state-machine
transitions is the closest structural equivalent to TypeScript's polymorphic interface
return.  Using an enum instead would require a large match statement in the factory and
obscure the per-state dispatch pattern.

### D-BlobModel-ContainerModel-Minimal
`BlobModel` and `ContainerModel` are defined in `persistence/i_blob_metadata_store.rs`
with only the fields the lease subsystem needs today (`properties`, `leaseId`,
`leaseDurationSeconds`, `leaseExpireTime`, `leaseBreakTime`, `accountName`, etc.).
Remaining `BlobItemInternal` fields (page ranges, committed blocks, metadata) are deferred
to Phase 10/11 when the handlers that consume them are ported.

**Rationale:** Avoids premature over-specification of the persistence model before the
handlers are translated; remaining fields can be added surgically in Phase 10.

### D-LeaseStateConstants
`LeaseStateType`, `LeaseStatusType`, and `LeaseDurationType` are implemented as Rust
sub-modules with `const &str` members (e.g. `LeaseStateType::Available = "available"`)
rather than enums, matching the TypeScript string-alias types `pub type LeaseStateType = String`.
Comparisons use `.as_deref()` against the const values.

**Rationale:** The generated `models.rs` already defines these as `pub type X = String`.
Introducing Rust enums would require conversion layers everywhere and make future TS
change propagation harder.

### D-ContainerDeleteLeaseValidator-NullCollapse
`ContainerDeleteLeaseValidator` in TypeScript checks `=== null` separately from
`=== undefined` for the incoming `leaseId` field.  In Rust, `GeneratedObject.get("leaseId")`
returns `None` for both absent keys and JSON-null values (via `GeneratedValue::as_string()`).
Both are collapsed to `None`, which has no observable behavioural difference at the Azure
REST wire level since both cases result in a missing/null lease ID.

**Rationale:** The Rust `GeneratedObject` accessor cannot distinguish null-JSON from absent
key without adding a `Null` variant to `GeneratedValue`, which would require a broader
change.  Document as intentional collapse.

---

## D-004: Phase 11/12 Linked Translation Unit Strategy (Faramir — Phase 11+12)
**Status:** ACTIVE

Treat Phase 11 (blob handlers) and Phase 12 (blob config/server assembly) as three linked translation units instead of a flat file list to preserve observable behaviors and avoid accidental normalization.

### Unit 1: Page Range Core (Prerequisite)
Port `IPageBlobRangesManager` + `PageBlobRangesManager` before `BlobHandler` / `PageBlobHandler`.

**Preserve:**
- Split-first/last range strategy
- `ZERO_EXTENT_ID` hole filling
- Inclusive offset arithmetic
- Documented latent quirks

**Rationale:** Blob download behavior depends on exact arithmetic. If this unit is incomplete, handlers cannot be tested in isolation.

### Unit 2: Batch Pipeline (Isolated Subsystem)
Port `BlobBatchSubRequest`, `BlobBatchSubResponse`, `SubResponseTextBodyStream`, and `BlobBatchHandler` together.

**Preserve:**
- Separate reduced middleware pipeline (context → dispatch → auth → deserialize → handler → serialize → end)
- No strict-mode, CORS, or telemetry in batch path
- Request context isolation

**Anti-pattern:** Do **not** collapse batch handling into the normal request listener. Batch intentionally rebuilds a reduced pipeline.

### Unit 3: Server Assembly (Bootstrap Order Sensitive)
Port Phase 12 request assembly with the exact middleware order documented in `BlobRequestListenerFactory.md`.

**Preserve:**
- `BlobEnvironment.blobKeepAliveTimeout()` reading `keepAliveTimeout` (typo preserved)
- `BlobServerFactory` mutating `DEFAULT_BLOB_PERSISTENCE_ARRAY`
- `main.ts` configuring logger/telemetry after server creation

**Rationale:** Bootstrap sequence is sensitive to field initialization order. Literal order preservation prevents silent behavior divergence.

**Rationale:** These are handwritten execution layers sitting on generated middleware. Flat file order porting risks accidental normalization of routing, batch, range, or bootstrap behavior.
