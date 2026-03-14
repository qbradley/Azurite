# Rust Parity Tests: Phase 6 & 7 Documentation Index

## Overview

This document set provides a complete guide for adding Rust parity tests for Blob Phase 6 (errors/context) and Phase 7 (authentication). All 14 ported modules are fully implemented and ready for testing.

## Documents

### 1. **PHASE6_7_PARITY_SUMMARY.md** (Primary Reference)
- **Purpose**: Quick executive summary
- **Length**: ~200 lines
- **Contents**:
  - What's ported (5 Phase 6 modules + 12 Phase 7 modules)
  - Test structure and how to wire new tests
  - 14 known fidelity risks (high + medium confidence)
  - 30 key TS parity requirements
  - File references and test execution commands
  - Success criteria

**Start here** for a 5-minute overview of what needs testing and why.

### 2. **PHASE6_7_PARITY_MAP.md** (Comprehensive Reference)
- **Purpose**: Detailed technical reference
- **Length**: 906 lines
- **Contents**:
  1. Existing Rust test structure & how to wire new ones (file paths, module setup)
  2. Behavior coverage matrix (what's implemented, known limitations)
  3. Exact TS fidelity requirements by module (30 tables with TS vs Rust comparison)
  4. 14 likely parity bugs & API mismatches with risk levels
  5. Complete test coverage checklist (100+ test cases)
  6. Test data & fixtures (account keys, container/blob names, SAS versions)
  7. Integration points with other phases
  8. Source code file references (TS + RS, line numbers)

**Consult this** when implementing specific tests or investigating fidelity issues.

### 3. **PHASE6_7_TEST_SCAFFOLD.rs** (Code Template)
- **Purpose**: Ready-to-use test code skeleton
- **Length**: 449 lines
- **Contents**:
  - Phase 6 tests (14 written tests for StorageError, StorageErrorFactory, NotImplementedError, StrictModelNotSupportedError, BlobStorageContext)
  - Phase 7 test scaffolds (28 test shells for all authenticators, SAS modules)
  - Import statements and module organization
  - Comments explaining each test
  - Ignored tests with #[ignore] and implementation notes

**Use this** as a starting template for test files.

### 4. **PHASE6_7_PARITY_INDEX.md** (This File)
- **Purpose**: Navigation and documentation structure
- **Contents**: File descriptions, quick links, key numbers

---

## Key Numbers

| Aspect | Count | Notes |
|---|---|---|
| **Ported Modules** | 14 | 5 Phase 6 + 9 Phase 7 |
| **Rust Files** | 14 | One per module (100–680 lines each) |
| **Error Factory Methods** | 74 | All ported, ready for test |
| **SAS Versions** | 4 | 2015-04-05, 2018-11-09, 2020-12-06, 2025-07-05 |
| **UDK Versions** | 4 | 2018-11-09, 2020-02-10, 2020-12-06, 2025-07-05 |
| **Authentication Types** | 5 | SharedKey, AccountSAS, BlobSAS (service+UDK), Token, PublicAccess |
| **Permission Enums** | 3 | BlobSASPermission (11), ContainerSASPermission (7+Any), BlobSASResourceType (3) |
| **Permission Tables** | 2 | OPERATION_BLOB_SAS (blob-level, container-level) |
| **Fidelity Risks** | 14 | 8 high confidence, 6 medium confidence |
| **Test Coverage Areas** | 50+ | Detailed in checklist section |

---

## Quick Start (5 Minutes)

1. **Read** PHASE6_7_PARITY_SUMMARY.md (focus on "What's Ported" and "Fidelity Risks" sections)
2. **Check** file locations in PHASE6_7_PARITY_MAP.md section 8 (File References)
3. **Review** PHASE6_7_TEST_SCAFFOLD.rs to understand test structure
4. **Create** new test modules:
   ```bash
   cp PHASE6_7_TEST_SCAFFOLD.rs rust/crates/azurite-blob/tests/blob/phase6_errors.rs
   cp PHASE6_7_TEST_SCAFFOLD.rs rust/crates/azurite-blob/tests/blob/phase7_authentication.rs
   ```
5. **Add** module declarations to `tests/blob/mod.rs`:
   ```rust
   mod phase6_errors;
   mod phase7_authentication;
   ```

---

## Implementation Roadmap

### Phase 6: Errors & Context (Highest Priority)
1. **StorageError** (storage_error.rs:110 lines)
   - Tests: 5 (XML escaping, headers, timestamp, extra fields, content-type)
   - Risk: Medium (timestamp RFC3339 format, XML escaping)

2. **StorageErrorFactory** (storage_error_factory.rs:680 lines)
   - Tests: 50+ (one per factory method or groups of similar methods)
   - Risk: High (error message snapshots, status code mapping, edge case defaults)

3. **NotImplementedError / NotImplementedinSQLError** (not_implemented_error.rs:50 lines)
   - Tests: 2 (status codes, request ID defaults)
   - Risk: Low

4. **StrictModelNotSupportedError** (strict_model_error.rs:40 lines)
   - Tests: 1 (message interpolation with feature name)
   - Risk: Low

5. **BlobStorageContext** (blob_storage_context.rs:137 lines)
   - Tests: 6 (getter/setter pairs, xMsRequestID alias, boolean fields, Deref)
   - Risk: Medium (interior mutability, Clone behavior)

### Phase 7: Authentication (Medium Priority)
1. **IAuthenticator** (i_authenticator.rs:14 lines)
   - Tests: 1 (trait structure, return types)
   - Risk: Low

2. **Permission Enums** (blob_sas_permissions.rs + container_sas_permissions.rs + blob_sas_resource_type.rs:~130 lines)
   - Tests: 5 (wire character mapping, sentinel handling, resource type dispatch)
   - Risk: Low

3. **IBlobSASSignatureValues** (i_blob_sas_signature_values.rs:479 lines)
   - Tests: 10 (version dispatch, canonical name format, string-to-sign layout per version)
   - Risk: High (8 version-specific generators, UDK vs service SAS asymmetries)

4. **OperationBlobSASPermission / OperationAccountSASPermission** (operation_*.rs:~280 lines)
   - Tests: 8 (permission table completeness, ANY-char matching, sentinel handling)
   - Risk: Medium (table correctness, ANY-char vs ALL-char semantics)

5. **Authenticators** (5 files, ~1200 lines total)
   - **BlobSharedKeyAuthenticator** (277 lines) → 10 tests
     - Risk: High (string-to-sign layout, Content-Length: 0 special case, secondary endpoint)
   
   - **AccountSASAuthenticator** (276 lines) → 8 tests
     - Risk: Medium (field requirement validation, strict mode, existing blob check)
   
   - **BlobSASAuthenticator** (449 lines) → 10 tests
     - Risk: High (UDK path vs service SAS, identifier ACL override scope, snapshot routing)
   
   - **BlobTokenAuthenticator** (194 lines) → 6 tests
     - Risk: Medium (Bearer extraction, HTTPS requirement, JWT parsing, time validation)
   
   - **PublicAccessAuthenticator** (125 lines) → 4 tests
     - Risk: Low (container requirement, metadata error handling, allowlist routing)

---

## Testing Priorities

### Must-Have Tests (Critical for Parity)
- [ ] StorageErrorFactory error message snapshots (all 74 methods)
- [ ] All 4 SAS version signature generators (4 layouts × 2 auth types = 8 tests)
- [ ] Shared Key string-to-sign construction (method + headers + resource)
- [ ] Permission ANY-character validation (not ALL-character)
- [ ] Identifier-based ACL policy override (only sp/st/se)
- [ ] Snapshot routing to CONTAINER permission table (not BLOB)
- [ ] UDK path validation (6 fields required + signedService="b")

### Should-Have Tests (High Fidelity)
- [ ] Timestamp RFC3339 format (millis + Z, not microseconds)
- [ ] XML escaping (< > & " ')
- [ ] Content-Length: 0 returns "" (not "0")
- [ ] Bearer token extraction ("Bearer TOKEN" → "TOKEN")
- [ ] Secondary endpoint "-secondary" suffix
- [ ] Empty permission string always fails validation
- [ ] Response override fields NOT decoded (rscc/rscd/rsce/rscl/rsct)

### Nice-to-Have Tests (Edge Cases)
- [ ] Non-standard SAS version strings (lexicographic comparison)
- [ ] Clone behavior of BlobStorageContext (shared vs independent)
- [ ] Metadata error swallowing in PublicAccessAuthenticator
- [ ] IP range and protocol validation stubs
- [ ] JWT signature verification intentionally skipped

---

## Fidelity Risk Mitigation

### Before Running Tests
1. **Read** the 14 fidelity risks in PHASE6_7_PARITY_SUMMARY.md
2. **Flag** high-confidence issues (items 1–8) in test names with `#[ignore]` if implementation uncertain
3. **Cross-reference** Rust implementation with TS source for ambiguous behaviors
4. **Use snapshot tests** for error messages and XML output (compare byte-for-byte)

### During Test Development
1. **Verify** each test against TS behavior (run TS tests in parallel)
2. **Document** discrepancies in test comments
3. **Use assertion libraries** that show diffs (pretty_assertions crate)
4. **Test edge cases** first (empty strings, special characters, boundary conditions)

### After Tests Pass
1. **Snapshot test outputs** (error XML, signatures, log messages)
2. **Compare snapshots** between TS and Rust runs
3. **Record any divergences** in a separate "Known Differences" document
4. **Plan fixes** for divergences if they're bugs, or document as intentional if behavioral differences are acceptable

---

## Integration with TS Tests

### Parallel Test Execution
```bash
# Terminal 1: Run TS tests (baseline)
cd /home/azureuser/Azurite
npm test -- tests/blob/authentication.test.ts
npm test -- tests/blob/sas.test.ts

# Terminal 2: Run Rust tests (comparison)
cd /home/azureuser/Azurite/rust
cargo test -p azurite-blob blob::phase6_errors
cargo test -p azurite-blob blob::phase7_authentication
```

### Snapshots to Compare
- StorageError XML body format
- All 74 error factory messages and status codes
- Shared Key string-to-sign (method + headers + resource)
- SAS signature output (4 service versions × 2 key types = 8 signatures)
- Permission validation results (ANY-char matching)
- Bearer token extraction logic

---

## File Locations (Absolute Paths)

### Porting Database
- `/home/azureuser/Azurite/rust/porting-db/src/blob/errors/*.md` (4 files)
- `/home/azureuser/Azurite/rust/porting-db/src/blob/context/*.md` (1 file)
- `/home/azureuser/Azurite/rust/porting-db/src/blob/authentication/*.md` (14 files)

### Rust Source
- `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/*.rs` (4 + mod.rs)
- `/home/azureuser/Azurite/rust/crates/azurite-blob/src/context/*.rs` (1 + mod.rs)
- `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/*.rs` (12 + mod.rs)

### Test Infrastructure
- `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob_parity.rs` (entry point)
- `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/mod.rs` (submodule declarations)
- `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/generated_framework.rs` (utilities)

### TS Source (Reference)
- `/home/azureuser/Azurite/src/blob/errors/*.ts` (4 files)
- `/home/azureuser/Azurite/src/blob/context/*.ts` (1 file)
- `/home/azureuser/Azurite/src/blob/authentication/*.ts` (12 files)

### TS Tests (Baseline)
- `/home/azureuser/Azurite/tests/blob/authentication.test.ts` (153 lines)
- `/home/azureuser/Azurite/tests/blob/sas.test.ts` (2398 lines)

---

## Success Metrics

| Metric | Target | How to Measure |
|---|---|---|
| **Compilation** | 0 warnings | `cargo build -p azurite-blob 2>&1 \| grep -i warn` |
| **Test count** | 100+ | `cargo test -p azurite-blob blob -- --list` |
| **Pass rate** | 100% | `cargo test -p azurite-blob blob --` |
| **Coverage** | All 14 modules | Grep test file for module names |
| **Fidelity** | Byte-for-byte match on signatures/errors | Snapshot comparison (TS vs RS) |
| **Unimplemented** | Zero | `cargo build -p azurite-blob 2>&1 \| grep -E "todo!|unimplemented!"` |

---

## Additional Resources

- **Porting Order**: `/home/azureuser/Azurite/PORTING-ORDER.md`
- **Phase 5 (Context/Request/Response)**: Reference for base types
- **Phase 4 (Shared Utilities)**: HMAC, date formatting
- **Phase 3 (Common Auth)**: Account SAS helpers, enums
- **TS SDK**: Azure Storage Blob SDK for reference behavior

---

**Last Updated**: 2024
**Maintained By**: Boromir (Rust Porting Lead)
**Status**: ✅ Complete. All modules ported and ready for testing.
