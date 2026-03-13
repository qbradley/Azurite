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
