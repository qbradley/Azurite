ist() && requires_write | Check after signature validation |
### **BlobTokenAuthenticator** (Phase 7.13)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Bearer prefix | "Bearer " + token | substr(prefix.length + 1) | Extract exactly after "Bearer " |
| HTTPS requirement | Throw AuthenticationFailed on HTTP | Check request protocol | Must reject non-HTTPS |
| JWT decode | jsonwebtoken.decode (no verify) | jwt crate, skip verify | Claims parsed as JSON |
| Required claims | nbf, exp, iat all checked | Check all three present | Missing any claim fails |
| Issuer regex | Match against VALID_ISSUE_PREFIXES | Regex match + capture validation | Account capture must match if group exists |
| Audience match | Exact string match | aud_regex match && m[0] == aud | Capture group must match current account |
| Time validation | Use context.startTime, not Date.now() | context.startTime().getTime() | Keeps auth time aligned with request start |
| OAuth level dispatch | Only BASIC currently implemented | match oauth_level | Return None (skip) for unknown levels |
### **PublicAccessAuthenticator** (Phase 7.14)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Container name requirement | Skip if container absent | Check containerName.is_some() | Return None if no container |
| Access type fetch | Async metadata lookup | async fn getContainerPublicAccessType | Errors swallowed, return None |
| Operation allowlist | Two separate Set<Operation> | Two HashSet statics | Container vs Blob read operations |
| Not-applicable behavior | Known access type but not allowlisted | Return None, not false | Allows next authenticator in chain |
| TODO comments | Uncertain entries (GetPageRanges, GetBlockList) | Preserve as shipped | Keep comments in tests |
---
## 4. LIKELY PARITY BUGS & API MISMATCHES IN RUST SOURCE
### ⚠️ HIGH CONFIDENCE ISSUES
#### 1. **BlobStorageContext Setter Mutation Issue** (blob_storage_context.rs:35–37)
- **Rust code**: `pub fn setAccount(&self, account: Option<String>)` takes `&self` but mutates via `context.insertExtra()`
- **Root cause**: Interior mutability via Context's internal Cell/RefCell (not visible in type signature)
- **Risk**: Tests that expect immutability may be surprised by mutations
- **Fix**: Document that "setters" are actually interior-mutable; confirm Context allows this pattern
- **TS comparison**: TS setters mutate `this.context` directly; Rust achieves same via interior mutability
- **Test focus**: Verify that setting and getting return same value in sequence
#### 2. **BlobStorageContext Clone Behavior** (blob_storage_context.rs:15)
- **Rust code**: `#[derive(Clone)]` on struct containing `Context`
- **Question**: Does `Context::new(context)` perform a deep clone or share interior-mutable state?
- **Risk**: Multiple clones may share the same underlying extras map
- **Test focus**: Mutate one BlobStorageContext clone, verify others are affected or unaffected
#### 3. **OperationBlobSASPermission Empty String Semantics** (operation_blob_sas_permission.rs:~40)
- **TS behavior**: `new OperationBlobSASPermission()` with no args → "" permission string
- **Rust behavior**: Unclear if empty string is preserved or treated as "no permission"
- **Test focus**: Verify `OperationBlobSASPermission::new("").validatePermissions("r")` returns false
#### 4. **BlobSharedKeyAuthenticator Canonicalized Headers Whitespace** (blob_shared_key_authenticator.rs:74)
- **TS code comment**: "Replace linear whitespace with a single space"
- **TS implementation**: Does NOT do this replacement
- **Rust code**: `trim_end()` on name, `trim_start()` on value
- **Risk**: If implementation adds normalization that TS doesn't, signatures will mismatch
- **Test focus**: Test header with internal whitespace; verify signature matches TS
#### 5. **BlobSASAuthenticator Request Field Extraction Asymmetry** (blob_sas_authenticator.rs:64–75)
- **Asymmetry**: Some fields decoded (sp, st, se, sv, spr, sip, si, snapshot, encryptionScope), others NOT (rscc, rscd, rsce, rscl, rsct)
- **TS matches this**: Response headers are NOT decoded
- **Rust code**: Missing `decodeURIComponent` for rscc/rscd/rsce/rscl/rsct
- **Test focus**: Verify rscc fields are NOT decoded when building signature
#### 6. **StorageError Body Timestamp Format** (storage_error.rs:87)
- **TS code**: `new Date().toISOString()` (always UTC Z suffix)
- **Rust code**: `chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)`
- **Risk**: Different timezone offset or precision
- **Test focus**: Parse XML body, verify timestamp ends with 'Z' and has exactly 3 decimal places
#### 7. **BlobTokenAuthenticator Bearer Token Extraction** (blob_token_authenticator.rs:~120)
- **TS code**: `authHeaderValue.substr(BEARER_TOKEN_PREFIX.length + 1)`
- **Expected**: "Bearer TOKEN" → extract "TOKEN" (skip 7 chars for "Bearer" + 1 for space)
- **Risk**: Off-by-one in string slicing
- **Test focus**: Provide "Bearer mytoken123" and verify token extracted is exactly "mytoken123"
#### 8. **AccountSASAuthenticator Missing IP Range Implementation** (account_sas_authenticator.rs:180)
- **TS code**: Stub that always returns true
- **Rust code**: Same stub
- **Risk**: If later IP validation added, tests may fail
- **Test focus**: Verify stub is called but result is ignored (always passes)
#### 9. **PublicAccessAuthenticator Error Swallowing** (public_access_authenticator.rs:~80)
- **TS behavior**: Metadata errors caught and return undefined
- **Rust behavior**: Catch errors and return None
- **Test focus**: Inject bad metadata store, verify returns None (continues auth chain)
### ⚠️ MEDIUM CONFIDENCE EDGE CASES
#### 10. **IBlobSASSignatureValues Version String Comparison** (i_blob_sas_signature_values.rs:62–78)
- **Comparison method**: `version.as_str() >= "2020-12-06"` (lexicographic)
- **Risk**: Non-standard version format (e.g., "2020-1-01") might not sort as expected
- **Test focus**: Test non-standard but valid versions (if any exist); verify fallback to older layout
#### 11. **BlobSharedKeyAuthenticator Secondary Endpoint Detection** (blob_shared_key_authenticator.rs:130)
- **TS behavior**: `authenticationPath?.indexOf(account) === 1`
- **Rust translation**: Check if path starts with "/" + account
- **Risk**: Off-by-one in string indexing
- **Test focus**: Pass authenticationPath="/account-secondary/container/blob", verify secondary flag used
#### 12. **StorageError Extra Fields XML Order** (storage_error.rs:98)
- **TS behavior**: JSON object iteration order (insertion order in modern JS)
- **Rust behavior**: BTreeMap iteration order (lexicographic key order)
- **Risk**: XML element order differs from TS
- **Test focus**: For helpers with multiple extra fields (getMd5Mismatch, getInvalidQuery*), verify XML element order matches TS alphabetically (BTreeMap is deterministic)
#### 13. **ContainerSASPermissions.Any Field Identification** (container_sas_permissions.rs:20)
- **TS definition**: Literal string "AnyPermission"
- **Rust definition**: Check if enum variant or string constant
- **Test focus**: Verify Any is NOT treated as a permission character; special-cased in validation
#### 14. **BlobSASAuthenticator Saved Policy Field Override Scope** (blob_sas_authenticator.rs:~300)
- **TS behavior**: ONLY startTime, expiryTime, permissions override from ACL; NOT protocol, IP, response headers, encryption scope
- **Rust code**: Confirm selectivity of override
- **Test focus**: Fetch ACL, verify protocol/IP/ses/rscc not overridden
---
## 5. TEST COVERAGE CHECKLIST
### Phase 6: Errors & Context
- [ ] StorageError
  - [ ] XML escaping of special characters in error code/message
  - [ ] Timestamp generation at construction (RFC3339 with 'Z' and millis)
  - [ ] Headers always set (x-ms-error-code, x-ms-request-id)
  - [ ] Extra XML elements appear as siblings
  - [ ] Content-Type is always "application/xml"
- [ ] StorageErrorFactory
  - [ ] Each of 74 factory methods returns correct status code
  - [ ] Error codes and messages match TS exactly (test snapshot)
  - [ ] Default context IDs match TS (mix of DEFAULT_ID and empty)
  - [ ] getMd5Mismatch() includes both MD5 values in XML
  - [ ] getInvalidPageRange2() injects Content-Range header
  - [ ] getInvalidTag() returns "DuplicateTagNames" (not "InvalidTag")
  - [ ] getInvalidQueryParameterValue() conditional extra fields
  - [ ] Snapshot-related helpers (3 variants) return correct codes
- [ ] NotImplementedError & NotImplementedinSQLError
  - [ ] Return 501 status
  - [ ] Request ID defaults to ""
  - [ ] Error code is "APINotImplemented"
  - [ ] Messages differ (generic vs SQL-scoped)
- [ ] StrictModelNotSupportedError
  - [ ] Returns 500 status
  - [ ] Error code is "FeatureNotSupported"
  - [ ] Feature name interpolated in message
  - [ ] Contains "--loose" and "Loose" guidance
- [ ] BlobStorageContext
  - [ ] All getters/setters work correctly (account, container, blob, authenticationPath, xMsRequestID, loose, isSecondary, disableProductStyleUrl)
  - [ ] xMsRequestID is alias for contextId
  - [ ] Setting to None inserts Null value
  - [ ] Can pass to functions expecting &Context (Deref)
  - [ ] Clone behavior (shared or independent mutations)
### Phase 7: Authentication
- [ ] IAuthenticator trait
  - [ ] Async method signature
  - [ ] Tri-state return (Some(true), Some(false), None)
- [ ] BlobSASPermissions & ContainerSASPermissions
  - [ ] All enum values and wire characters present
  - [ ] Casing preserved (execute/permanentDelete lowercase)
  - [ ] Any/AnyPermission sentinel not treated as permission char
- [ ] BlobSASResourceType
  - [ ] Container="c", Blob="b", BlobSnapshot="bs"
- [ ] IBlobSASSignatureValues & Signature Generators
  - [ ] All 24 struct fields present
  - [ ] Version dispatch (2025-07-05 → 2020-12-06 → 2018-11-09 → 2015-04-05)
  - [ ] Canonical name /blob/{account}/{container}[/{blob}] (no URL encoding)
  - [ ] String-to-sign layout matches TS for each version
  - [ ] Empty fields preserved in signature
  - [ ] Blob vs BlobSnapshot handling differs in UDK paths
  - [ ] Service SAS: both permissions and expiryTime optional if identifier present
  - [ ] UDK: permissions and expiryTime required even with identifier
- [ ] OperationBlobSASPermission & OperationAccountSASPermission
  - [ ] Permission tables complete and correct
  - [ ] ANY-character matching (not ALL-character)
  - [ ] Sentinel handling (Any = "has content")
  - [ ] Empty permission string always fails
  - [ ] Snapshot uses CONTAINER_PERMISSIONS, not BLOB_PERMISSIONS
- [ ] BlobSharedKeyAuthenticator
  - [ ] Authorization header check (must start with "SharedKey")
  - [ ] Account lookup (unknown → ResourceNotFound)
  - [ ] Reject GetUserDelegationKey
  - [ ] String-to-sign: method\n + standard headers\n + x-ms-headers\n + resource
  - [ ] Content-Length: 0 returns "" (not "0")
  - [ ] Canonical headers: lowercase name, trimLeft value, sorted
  - [ ] Canonical resource: lowercase query keys, decode values
  - [ ] Key1 and key2 both tested
  - [ ] Secondary endpoint "-secondary" suffix insertion
- [ ] AccountSASAuthenticator
  - [ ] All required fields (sv, se, sp, ss, srt, sig) enforced
  - [ ] Strict mode rejects ses with StrictModelNotSupportedError
  - [ ] Signature validation before other checks
  - [ ] Time, IP, protocol checks (stubs)
  - [ ] Operation lookup in OPERATION_ACCOUNT_SAS_PERMISSIONS
  - [ ] Existing blob Write requirement for create/copy
  - [ ] Blob existence check (uncommitted = nonexistent)
- [ ] BlobSASAuthenticator
  - [ ] Resource type validation (c/b/bs only)
  - [ ] Identifier-based ACL policy override (only sp/st/se)
  - [ ] UDK path: require six signed fields + signedService="b"
  - [ ] UDK signature immediate validation (no fallback)
  - [ ] Service SAS key1/key2 fallback
  - [ ] Response override fields NOT decoded (rscc, rscd, rsce, rscl, rsct)
  - [ ] Time, IP, protocol checks
  - [ ] Permission table routing (Blob → BLOB_PERMISSIONS, Container/BlobSnapshot → CONTAINER_PERMISSIONS)
  - [ ] Existing blob Write requirement
- [ ] BlobTokenAuthenticator
  - [ ] Bearer prefix extraction (exactly 7 + 1 chars)
  - [ ] HTTPS requirement (reject HTTP)
  - [ ] JWT decode without verification
  - [ ] Required claims: nbf, exp, iat
  - [ ] Issuer validation against VALID_ISSUE_PREFIXES
  - [ ] Audience regex match + capture validation
  - [ ] Time validation uses context.startTime
  - [ ] OAuth level dispatch (BASIC only)
- [ ] PublicAccessAuthenticator
  - [ ] Container name requirement (skip if absent)
  - [ ] Metadata lookup (errors swallowed)
  - [ ] Operation allowlist (CONTAINER_PUBLIC_READ_OPERATIONS, BLOB_PUBLIC_READ_OPERATIONS)
  - [ ] Not-applicable for known access type but non-allowlisted operation
---
## 6. TEST DATA & FIXTURES
### Account Keys
- Primary: `"MTAwCjE2NQoyMjUKMTAzCjIxOAoyNDEKNDAKNzgKMTkxCjE3OAoyMTQKMTY5CjIxMwo2MQoyNTIKMTQxCg=="` (devstoreaccount1)
- Secondary: `"testing_key"`
- Invalid: any other key
### Account Names
- Valid: `"devstoreaccount1"`, `"devstoreaccount2"`
- Invalid: `"invalid"`, `"unknown"`
### Containers & Blobs
- Test containers: `"test-container"`, `"public-container"`
- Test blobs: `"test-blob"`, `"test-snapshot"`
- Block blobs: `"block-blob"` (committed)
- Uncommitted block blobs: `"uncommitted-block"`
### SAS Versions
- Current: `"2020-12-06"`, `"2025-07-05"`
- Legacy: `"2018-11-09"`, `"2015-04-05"`
- Invalid: `"2020-01-01"` (should fall through to older layout)
### Error Response Examples
```xml
<Error>
  <Code>ContainerNotFound</Code>
  <Message>The specified container does not exist.
RequestId:test-id
Time:2024-01-15T10:30:45.123Z</Message>
</Error>
```
---
## 7. INTEGRATION POINTS WITH OTHER PHASES
- **Phase 5.7 (Context)**: BlobStorageContext wraps generated Context
- **Phase 5.6 (XML)**: StorageError uses jsonToXML (via quick_xml in Rust)
- **Phase 5.11 (Operation)**: Permission tables keyed by Operation enum
- **Phase 4.2 (Crypto)**: HMAC-SHA256 for all signatures
- **Phase 3.x (Common Auth)**: DateOrString, SASProtocol, AccountSAS helpers
- **Phase 2 (Persistence)**: IBlobMetadataStore for access policies and blob existence checks
---
## 8. REFERENCES TO SOURCE CODE
### Errors Module
- TS: `/home/azureuser/Azurite/src/blob/errors/StorageError.ts` (lines 1–65)
- TS: `/home/azureuser/Azurite/src/blob/errors/StorageErrorFactory.ts` (lines 1–854)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/storage_error.rs` (lines 1–110)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/storage_error_factory.rs` (lines 1–680)
### Context Module
- TS: `/home/azureuser/Azurite/src/blob/context/BlobStorageContext.ts` (lines 1–73)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/context/blob_storage_context.rs` (lines 1–137)
### Authentication Module
- TS: `/home/azureuser/Azurite/src/blob/authentication/IAuthenticator.ts` (lines 1–6)
- TS: `/home/azureuser/Azurite/src/blob/authentication/IBlobSASSignatureValues.ts` (lines 1–818)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/i_authenticator.rs` (lines 1–14)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/i_blob_sas_signature_values.rs` (lines 1–479) 
### All Authenticators
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_shared_key_authenticator.rs` (277 lines) )
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/account_sas_authenticator.rs` (276 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_sas_authenticator.rs` (449 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_token_authenticator.rs` (194 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/public_access_authenticator.rs` (125 lines)
### Existing Tests
- TS: `/home/azureuser/Azurite/tests/blob/authentication.test.ts` (153 lines)
- TS: `/home/azureuser/Azurite/tests/blob/sas.test.ts` (2398 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob_parity.rs` (empty scaffold)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/generated_framework.rs` (500+ lines, utilities)
EOF
cat /tmp/parity_map.md
                EC=$?
                echo "___BEGIN___COMMAND_DONE_MARKER___$EC"
            }
___BEGIN___COMMAND_OUTPUT_MARKER___
# Rust Parity Test Map for Blob Phase 6 & 7: Errors, Context & Authentication

## 1. EXISTING RUST TEST STRUCTURE & HOW TO WIRE NEW TESTS

### Test File Organization
- **Primary test entry**: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob_parity.rs` (currently empty scaffold: `#[ignore]`)
- **Test submodule**: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/mod.rs`
  - Contains three modules: `apis`, `generated_framework`, `unit`
- **Generated framework tests**: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/generated_framework.rs` (lines 1–500+)
  - Contains `RecordingLogger`, `CallRecorder`, `ServiceHandlerHarness` test utilities
  - Has framework for handler/middleware testing
- **Inline tests**: None yet in `/src/errors/`, `/src/context/`, or `/src/authentication/`

### How to Wire New Tests
1. **Create new test module** in `/rust/crates/azurite-blob/tests/blob/`:
   - `phase6_errors.rs` — for errors, context, and error factory tests
   - `phase7_authentication.rs` — for all authenticators, SAS signature values, and permissions
   
2. **Add module declarations** in `/tests/blob/mod.rs`:
   ```rust
   mod phase6_errors;
   mod phase7_authentication;
   ```

3. **Use existing utilities**:
   - `RecordingLogger` (from generated_framework.rs:55–94) for capturing log output
   - Test harnesses can leverage `GeneratedHttpRequest`, `Context`, and `BlobStorageContext`
   - Inject mock `IAccountDataStore`, `IBlobMetadataStore` via Arc<dyn Trait>

4. **Run tests**:
   ```bash
   cd /home/azureuser/Azurite/rust
   cargo test -p azurite-blob blob::phase6_errors
   cargo test -p azurite-blob blob::phase7_authentication
   ```

5. **Enable currently-disabled test**:
   - Remove `#[ignore]` from `/tests/blob/unit.rs:1` once infrastructure is ready
   - Remove `#[ignore]` from `/tests/blob_parity.rs:2` to activate scaffold

---

## 2. BEHAVIOR COVERAGE: WHAT'S IMPLEMENTED VS MISSING

### ✅ FULLY PORTED (Ready for Testing)

#### Phase 6.1: StorageError
- **File**: `src/errors/storage_error.rs` (110 lines)
- **Traits/Impl**: `std::error::Error`, `Display`, `Clone`, `Debug`
- **Fields**: statusCode, message, statusMessage, headers, body, contentType, storageErrorCode, storageErrorMessage, storageRequestID
- **Constructor**: `new()` with BTreeMap extra fields
- **XML body generation**: Uses `quick_xml::escape::escape` for proper XML escaping
- **Headers set**: `x-ms-error-code`, `x-ms-request-id` always populated
- **Timestamp format**: RFC3339 with millis precision via `chrono::Utc::now().to_rfc3339_opts()`
- **Status**: ✅ Fully functional, matches TS behavior including timestamp generation at construction

#### Phase 6.2: StorageErrorFactory
- **File**: `src/errors/storage_error_factory.rs` (680 lines)
- **Methods ported**: 74 static factory helpers (all major methods present)
- **Key edge cases handled**:
  - `getMd5Mismatch()`: Correctly injects `UserSpecifiedMd5` and `ServerCalculatedMd5` XML elements
  - `getInvalidPageRange2()`: Mutates response to inject `Content-Range` header
  - `getInvalidTag()`: Returns error code `DuplicateTagNames` (not `InvalidTag`)—quirk preserved
  - `getInvalidQueryParameterValue()` / `getOutOfRangeInput()`: Conditionally add QueryParameterName, QueryParameterValue, Reason
- **Context ID defaults**: Mixes `Some(DEFAULT_ID)` and `None`/empty string—preserved as TS does
- **Status**: ✅ Complete 1:1 mapping

#### Phase 6.3: NotImplementedError
- **File**: `src/errors/not_implemented_error.rs` (50 lines)
- **Exports**: Two helper functions:
  - `NotImplementedError(request_id)` → 501, `APINotImplemented`, generic message
  - `NotImplementedinSQLError(request_id)` → 501, `APINotImplemented`, SQL-scoped message
- **Status**: ✅ Fully ported

#### Phase 6.4: StrictModelNotSupportedError
- **File**: `src/errors/strict_model_error.rs` (40 lines)
- **Constructor**: Takes feature name, builds interpolated message with `--loose` and VS Code `Loose` guidance
- **Always returns**: HTTP 500, code `FeatureNotSupported`
- **Status**: ✅ Fully ported

#### Phase 6.5: BlobStorageContext
- **File**: `src/context/blob_storage_context.rs` (137 lines)
- **Trait impl**: `IAuthenticationContext`
- **Accessor pattern**: Wraps generated `Context`, provides convenience getters/setters using extras map
- **Storage**: All fields stored in `context.extras()` as `GeneratedValue` enum (String, Bool, Null)
- **Key accessors**: account, container, blob, authenticationPath, xMsRequestID (alias for contextId), loose, isSecondary, disableProductStyleUrl
- **Deref impl**: To unwrap the inner Context when needed
- **Status**: ✅ Fully ported, parity confirmed with TS getter/setter semantics

#### Phase 7.1: IAuthenticator
- **File**: `src/authentication/i_authenticator.rs` (14 lines)
- **Trait**: Single async method `validate(req: IRequest, content: Context) -> Result<Option<bool>, StorageError>`
- **Return semantics**: Some(true) = auth passed, Some(false) = auth failed, None = not applicable
- **Status**: ✅ Trait defined and ready

#### Phase 7.4: BlobSASPermissions
- **File**: `src/authentication/blob_sas_permissions.rs` (54 lines)
- **Enum values**: Read="r", Add="a", Create="c", Write="w", Delete="d", DeleteVersion="x", Tag="t", Move="m", execute="e", SetImmutabilityPolicy="i", permanentDelete="y"
- **Quirk preserved**: Lowercase execute and permanentDelete while rest are PascalCase
- **Status**: ✅ Complete

#### Phase 7.5: BlobSASResourceType
- **File**: `src/authentication/blob_sas_resource_type.rs` (29 lines)
- **Enum values**: Container="c", Blob="b", BlobSnapshot="bs"
- **Status**: ✅ Complete

#### Phase 7.6: ContainerSASPermissions
- **File**: `src/authentication/container_sas_permissions.rs` (44 lines)
- **Enum values**: Read="r", Add="a", Create="c", Write="w", Delete="d", List="l", Filter="f", Any="AnyPermission"
- **Sentinel**: Any field used only in batch validation
- **Status**: ✅ Complete

#### Phase 7.3: IBlobSASSignatureValues
- **File**: `src/authentication/i_blob_sas_signature_values.rs` (479 lines)
- **Struct**: All 24 fields ported (version through delegatedUserTenantId)
- **Exports**: 
  - `generateBlobSASSignature()` (service SAS)
  - `generateBlobSASSignatureWithUDK()` (user delegation key SAS)
  - `getCanonicalName()` helper
- **Version dispatch**:
  - `>= "2025-07-05"` → UDK 2025-07-05 layout
  - `>= "2020-12-06"` → service/UDK 2020-12-06 layout
  - `>= "2018-11-09"` → service/UDK 2018-11-09 layout
  - else → service 2015-04-05 layout
- **Special handling**:
  - Canonical names NOT URL-encoded
  - Signature generated over un-encoded values
  - Permission and expiryTime: both can be missing if identifier present (matches TS)
  - Blob vs BlobSnapshot treated differently in UDK paths
- **Status**: ✅ Fully ported with version-specific helpers

#### Phase 7.9: OperationBlobSASPermission
- **File**: `src/authentication/operation_blob_sas_permission.rs` (162 lines)
- **Exports**: Two static permission maps:
  - `OPERATION_BLOB_SAS_BLOB_PERMISSIONS` (HashMap<Operation, OperationBlobSasPermission>)
  - `OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS` (HashMap<Operation, OperationBlobSasPermission>)
- **Validation logic**: ANY-character matching (not ALL-character), handles ContainerSASPermission::Any sentinel
- **Status**: ✅ Complete with both maps and validation methods

#### Phase 7.8: OperationAccountSASPermission
- **File**: `src/authentication/operation_account_sas_permission.rs` (121 lines)
- **Exports**: Static `OPERATION_ACCOUNT_SAS_PERMISSIONS` map with all blob operations
- **Validation**: Checks service, resource type, and permission using substring/character-presence matching
- **Sentinels**: AccountSASPermission::Any and AccountSASResourceType::Any handled
- **Status**: ✅ Complete

#### Phase 7.10: BlobSharedKeyAuthenticator
- **File**: `src/authentication/blob_shared_key_authenticator.rs` (277 lines)
- **Constructor**: Takes IAccountDataStore + ILogger
- **Implements**: IAuthenticator trait
- **Validation flow**: 
  1. Check Authorization header starts with "SharedKey"
  2. Lookup account
  3. Reject GetUserDelegationKey
  4. Build string-to-sign from method + headers + canonicalized x-ms-* + resource
  5. Compare signature against key1, then key2
  6. Handle secondary endpoint `-secondary` suffix if applicable
- **Header signing**: Content-Length: 0 returns empty string (quirk preserved)
- **Canonical resource**: Lowercases query keys, decodes values (with +→%20 replacement)
- **Status**: ✅ Fully ported

#### Phase 7.11: AccountSASAuthenticator
- **File**: `src/authentication/account_sas_authenticator.rs` (276 lines)
- **Constructor**: Takes IAccountDataStore + IBlobMetadataStore + ILogger
- **Validation flow**:
  1. Extract all account-SAS query params (sv, ss, srt, spr, st, se, sip, sp, sig, ses)
  2. Reject ses in strict mode with StrictModelNotSupportedError
  3. Validate signature against key1/key2
  4. Check time, IP, protocol (stubs)
  5. Lookup operation in OPERATION_ACCOUNT_SAS_PERMISSIONS
  6. Special: existing blob requires Write for create/copy
- **IP validation**: Stub that always returns true
- **Protocol validation**: Permissive (any comma passes)
- **Status**: ✅ Fully ported with known limitations documented

#### Phase 7.12: BlobSASAuthenticator
- **File**: `src/authentication/blob_sas_authenticator.rs` (449 lines)
- **Constructor**: Takes IAccountDataStore + IBlobMetadataStore + ILogger
- **Validation flow**:
  1. Extract blob-SAS from query (sv, sr, snapshot, sp, si, st, se, sip, spr, ses, + UDK fields)
  2. Validate sr is valid (c/b/bs)
  3. UDK path: parse signed fields, validate signed timestamps, derive key, validate signature
  4. Service SAS path: validate signature against key1/key2
  5. Handle identifier (fetch ACL, override sp/st/se from saved policy)
  6. Time/IP/protocol checks (partial)
  7. Route blob vs container vs snapshot to correct permission table
  8. Enforce permission + existing-blob Write rule for create/copy
- **Not parsing**: delegatedUserObjectId, delegatedUserTenantId (generator knows them for 2025-07-05, extractor does not)
- **Snapshot asymmetry**: Uses CONTAINER permissions, not BLOB permissions
- **Response header overrides**: Validated in signature but not applied to response yet (TODO)
- **Status**: ✅ Fully ported with documented future work

#### Phase 7.13: BlobTokenAuthenticator
- **File**: `src/authentication/blob_token_authenticator.rs` (194 lines)
- **Constructor**: Takes IAccountDataStore + OAuthLevel + ILogger
- **Validation flow**:
  1. Wrap context as BlobStorageContext
  2. Lookup account
  3. Skip for Container_GetAccessPolicy / Container_SetAccessPolicy
  4. Require Bearer prefix
  5. Require HTTPS
  6. Dispatch by OAuth level (only BASIC implemented)
  7. In BASIC: decode JWT (no verification), check nbf/exp/iat, validate issuer, validate audience
- **JWT validation**: Signature intentionally NOT verified (per TS comment "skip in basic check")
- **Time validation**: Uses context.startTime, not current time
- **Status**: ✅ Fully ported

#### Phase 7.14: PublicAccessAuthenticator
- **File**: `src/authentication/public_access_authenticator.rs` (125 lines)
- **Constructor**: Takes IBlobMetadataStore + ILogger
- **Validation flow**:
  1. Only applies when containerName present
  2. Fetch container public access type
  3. If Container or Blob, check operation against allowlist
  4. If allowlisted, return true; if known public access but not allowlisted, return None
- **Allowlists**: Separate sets for container vs blob read operations
- **Metadata errors**: Swallowed and return None (permissive)
- **Status**: ✅ Fully ported

### ⚠️ PARTIALLY TESTED / KNOWN LIMITATIONS

#### Phase 7.11 & 7.12: IP Range & Protocol Validation
- **Files**: account_sas_authenticator.rs:180–185, blob_sas_authenticator.rs:350–355
- **Implementation**: Stubs that always return `true`
- **TS Parity**: ✅ Matches TS behavior (TS has same stubs with TODO comments)
- **Test requirement**: Tests should verify stub is called but ignore actual validation

#### Phase 7.13: JWT Signature Verification
- **File**: blob_token_authenticator.rs:120–160
- **Implementation**: Intentionally skipped (comment: "Validate signature, skip in basic check")
- **TS Parity**: ✅ Matches TS behavior
- **Test requirement**: Tests should verify JWTs are parsed without signature validation

#### Phase 7.12: Response Header Overrides
- **File**: blob_sas_authenticator.rs:340–345
- **Implementation**: Validated during signature but not applied to HTTP response
- **TS Parity**: ✅ Matches TS (has TODO for future application)
- **Test requirement**: Tests should verify SAS fields are parsed but not assert they appear in response

---

## 3. EXACT TS FIDELITY REQUIREMENTS BY TARGET MODULE

### **StorageError** (Phase 6.1)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| XML escaping | jsonToXML (auto-escapes) | quick_xml::escape | Special chars: `<>&"'` in error codes/messages |
| Message expansion | `${msg}\nRequestId:${id}\nTime:${iso}` | Format string with chrono | Exact newline + timestamp format |
| Header set | Always `x-ms-error-code` + `x-ms-request-id` | BTreeMap insertion | Both headers present and correct values |
| Extra XML elements | User-provided BTreeMap | BTreeMap iteration | Custom elements appear as sibling XML elements |
| Content-Type | Always `application/xml` | String field | Must be set |

### **StorageErrorFactory** (Phase 6.2)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Default context IDs | Mix of DEFAULT_ID and empty string | Some(DEFAULT_ID) vs Some("") | Each helper returns correct default |
| MD5 fields | User + Server in separate XML elements | BTreeMap entries | Both XML fields populated |
| Page range header | Injected post-construction in getInvalidPageRange2 | headers_mut().insert() | Content-Range header added when supplied |
| Error messages | Exact string templates | String literals | Snapshot helpers (3 variants), auth helpers, tag quirk |
| Status codes | 304 for NotModified, 416 for range, 400 for validation | u16 constants | Edge case status codes match exactly |

### **NotImplementedError & StrictModelNotSupportedError** (Phase 6.3, 6.4)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Request ID default | Optional, defaults to "" | Option<&str> with unwrap_or("") | Empty string when not provided |
| HTTP status | 501 (NotImplemented), 500 (Strict) | u16 constants | Exact status codes |
| Message interpolation | Feature string inserted verbatim | format! macro | `--loose` and VS Code `Loose` in message |

### **BlobStorageContext** (Phase 6.5)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Storage mechanism | Direct properties on this.context | GeneratedValue enum in extras map | getters read from extras map correctly |
| Getter/Setter pairs | Unified property access | get_string/set_string methods | account, container, blob, authenticationPath, loose, isSecondary, disableProductStyleUrl all functional |
| xMsRequestID alias | Reads/writes context.contextId | Delegation to context.contextId() | Alias points to correct underlying field |
| Null handling | undefined → undefined | GeneratedValue::Null | Setting to None inserts Null; getting Null returns None |
| None storage type compatibility | Direct property reads | Deref impl | Can pass to functions expecting &Context |

### **IAuthenticator** (Phase 7.1)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Tri-state return | boolean | undefined means "skip" | Three paths: Ok(Some(true)), Ok(Some(false)), Ok(None) |
| Async/await | Promise-based | async fn | All authenticators use #[async_trait] |
| Error propagation | Throws StorageError | Returns Err(StorageError) | Errors halt auth chain immediately |

### **SAS Permission Enums** (Phase 7.4, 7.5, 7.6)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Casing inconsistency | execute lowercase, others PascalCase | Match TS exactly | Wire character tests for "e" vs "E", "y" vs "Y" |
| Snapshot resource | "bs" two-character code | String comparison | Distinct from Blob="b" in routing |
| Sentinel (Any) | "AnyPermission" string | Separate variant | Should NOT be serialized as permission character |

### **IBlobSASSignatureValues** (Phase 7.3)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Version comparison | Lexicographic string >= | as_str() >= "YYYY-MM-DD" | Edge: "2020-1-01" should NOT match "2020-12-06" |
| Canonical name | /blob/{account}/{container}[/{blob}] | format! concatenation | NO URL encoding; raw slash assembly |
| String-to-sign layout | Version-specific array join("\n") | Version-specific helpers | Empty fields preserved (undefined → empty string) |
| UDK vs Service SAS | Different field sets and placeholders | Two separate functions | UDK rejects saved policies; service SAS uses identifier |
| Blob vs BlobSnapshot | Different canonical treatment by version | Conditional blob appending | UDK 2020+ does NOT append blob for snapshot |

### **OperationBlobSASPermission & OperationAccountSASPermission** (Phase 7.8, 7.9)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| ANY-char matching | Any required character present = pass | Loop with contains check | "wc" permission passes if either w or c present |
| Sentinel handling | Any/AnyPermission = "has any content" | Special case in validate | Empty string always fails; sentinel only for batch |
| Table structure | One entry per Operation | HashMap<Operation, ...> | No computed permissions; exact table lookup |
| Two blob/container tables | Separate maps for blob vs container scope | BLOB_PERMISSIONS vs CONTAINER_PERMISSIONS | snapshot routes through container table, not blob |

### **BlobSharedKeyAuthenticator** (Phase 7.10)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| String-to-sign layout | method\n + headers\n + x-ms\n + resource | concat! with newlines | Exact byte order matters for HMAC |
| Content-Length: 0 | Returns "" (not "0") | if value == "0" return "" | Special case for empty body |
| Canonical headers | Lowercase name, trimLeft value, sort by name | iter, sort, format! | Whitespace handling: trim_end name, trim_start value |
| Secondary suffix | Insert "-secondary" after account | format string | /account-secondary/container/... |
| Key fallback | Try key1, then key2 | Two HMAC comparisons | Both keys should match same signature |

### **AccountSASAuthenticator** (Phase 7.11)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Required fields | sv, se, sp, ss, srt, sig all required | All extracted, None check | Return false if any missing (not undefined) |
| Strict mode ses | Throw StrictModelNotSupportedError | Match with error code | "SAS Encryption Scope 'ses'" exact message |
| Signature order | Generate then validate | Call generateAccountSASSignature | Both keys checked before other validations |
| Blob existence rule | Write required if destination exists | blobExist() check after signature | Only for create/copy operations |
| Blob committed check | Uncommitted blocks = nonexistent | Check BlobType::BlockBlob && !committed | Matches TS behavior |

### **BlobSASAuthenticator** (Phase 7.12)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Resource type dispatch | sr=c/b/bs only valid values | match on value, None for invalid | Invalid sr returns None, not false |
| Identifier-based policy | si → fetch ACL, override sp/st/se | access_policy_field helper | Only these three fields override, not others |
| UDK validation | Check signed six fields + service=b | All UDK fields required | signedService must be "b" |
| UDK signature order | Derive key then validate | generateBlobSASSignatureWithUDK | Immediate false if mismatch, no fallback to service SAS |
| Response override fields | rscc, rscd, rsce, rscl, rsct | No decodeURIComponent | These are NOT URL-decoded (quirk) |
| Snapshot permission table | Routes to CONTAINER permissions | Use CONTAINER_PERMISSIONS | Only BLOB resource uses BLOB_PERMISSIONS |
| Existing blob Write | Required for upload/create/copy | blobExist() && requires_write | Check after signature validation |

### **BlobTokenAuthenticator** (Phase 7.13)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Bearer prefix | "Bearer " + token | substr(prefix.length + 1) | Extract exactly after "Bearer " |
| HTTPS requirement | Throw AuthenticationFailed on HTTP | Check request protocol | Must reject non-HTTPS |
| JWT decode | jsonwebtoken.decode (no verify) | jwt crate, skip verify | Claims parsed as JSON |
| Required claims | nbf, exp, iat all checked | Check all three present | Missing any claim fails |
| Issuer regex | Match against VALID_ISSUE_PREFIXES | Regex match + capture validation | Account capture must match if group exists |
| Audience match | Exact string match | aud_regex match && m[0] == aud | Capture group must match current account |
| Time validation | Use context.startTime, not Date.now() | context.startTime().getTime() | Keeps auth time aligned with request start |
| OAuth level dispatch | Only BASIC currently implemented | match oauth_level | Return None (skip) for unknown levels |

### **PublicAccessAuthenticator** (Phase 7.14)
| Aspect | TS Behavior | Rust Implementation | Test Focus |
|---|---|---|---|
| Container name requirement | Skip if container absent | Check containerName.is_some() | Return None if no container |
| Access type fetch | Async metadata lookup | async fn getContainerPublicAccessType | Errors swallowed, return None |
| Operation allowlist | Two separate Set<Operation> | Two HashSet statics | Container vs Blob read operations |
| Not-applicable behavior | Known access type but not allowlisted | Return None, not false | Allows next authenticator in chain |
| TODO comments | Uncertain entries (GetPageRanges, GetBlockList) | Preserve as shipped | Keep comments in tests |

---

## 4. LIKELY PARITY BUGS & API MISMATCHES IN RUST SOURCE

### ⚠️ HIGH CONFIDENCE ISSUES

#### 1. **BlobStorageContext Setter Mutation Issue** (blob_storage_context.rs:35–37)
- **Rust code**: `pub fn setAccount(&self, account: Option<String>)` takes `&self` but mutates via `context.insertExtra()`
- **Root cause**: Interior mutability via Context's internal Cell/RefCell (not visible in type signature)
- **Risk**: Tests that expect immutability may be surprised by mutations
- **Fix**: Document that "setters" are actually interior-mutable; confirm Context allows this pattern
- **TS comparison**: TS setters mutate `this.context` directly; Rust achieves same via interior mutability
- **Test focus**: Verify that setting and getting return same value in sequence

#### 2. **BlobStorageContext Clone Behavior** (blob_storage_context.rs:15)
- **Rust code**: `#[derive(Clone)]` on struct containing `Context`
- **Question**: Does `Context::new(context)` perform a deep clone or share interior-mutable state?
- **Risk**: Multiple clones may share the same underlying extras map
- **Test focus**: Mutate one BlobStorageContext clone, verify others are affected or unaffected

#### 3. **OperationBlobSASPermission Empty String Semantics** (operation_blob_sas_permission.rs:~40)
- **TS behavior**: `new OperationBlobSASPermission()` with no args → "" permission string
- **Rust behavior**: Unclear if empty string is preserved or treated as "no permission"
- **Test focus**: Verify `OperationBlobSASPermission::new("").validatePermissions("r")` returns false

#### 4. **BlobSharedKeyAuthenticator Canonicalized Headers Whitespace** (blob_shared_key_authenticator.rs:74)
- **TS code comment**: "Replace linear whitespace with a single space"
- **TS implementation**: Does NOT do this replacement
- **Rust code**: `trim_end()` on name, `trim_start()` on value
- **Risk**: If implementation adds normalization that TS doesn't, signatures will mismatch
- **Test focus**: Test header with internal whitespace; verify signature matches TS

#### 5. **BlobSASAuthenticator Request Field Extraction Asymmetry** (blob_sas_authenticator.rs:64–75)
- **Asymmetry**: Some fields decoded (sp, st, se, sv, spr, sip, si, snapshot, encryptionScope), others NOT (rscc, rscd, rsce, rscl, rsct)
- **TS matches this**: Response headers are NOT decoded
- **Rust code**: Missing `decodeURIComponent` for rscc/rscd/rsce/rscl/rsct
- **Test focus**: Verify rscc fields are NOT decoded when building signature

#### 6. **StorageError Body Timestamp Format** (storage_error.rs:87)
- **TS code**: `new Date().toISOString()` (always UTC Z suffix)
- **Rust code**: `chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)`
- **Risk**: Different timezone offset or precision
- **Test focus**: Parse XML body, verify timestamp ends with 'Z' and has exactly 3 decimal places

#### 7. **BlobTokenAuthenticator Bearer Token Extraction** (blob_token_authenticator.rs:~120)
- **TS code**: `authHeaderValue.substr(BEARER_TOKEN_PREFIX.length + 1)`
- **Expected**: "Bearer TOKEN" → extract "TOKEN" (skip 7 chars for "Bearer" + 1 for space)
- **Risk**: Off-by-one in string slicing
- **Test focus**: Provide "Bearer mytoken123" and verify token extracted is exactly "mytoken123"

#### 8. **AccountSASAuthenticator Missing IP Range Implementation** (account_sas_authenticator.rs:180)
- **TS code**: Stub that always returns true
- **Rust code**: Same stub
- **Risk**: If later IP validation added, tests may fail
- **Test focus**: Verify stub is called but result is ignored (always passes)

#### 9. **PublicAccessAuthenticator Error Swallowing** (public_access_authenticator.rs:~80)
- **TS behavior**: Metadata errors caught and return undefined
- **Rust behavior**: Catch errors and return None
- **Test focus**: Inject bad metadata store, verify returns None (continues auth chain)

### ⚠️ MEDIUM CONFIDENCE EDGE CASES

#### 10. **IBlobSASSignatureValues Version String Comparison** (i_blob_sas_signature_values.rs:62–78)
- **Comparison method**: `version.as_str() >= "2020-12-06"` (lexicographic)
- **Risk**: Non-standard version format (e.g., "2020-1-01") might not sort as expected
- **Test focus**: Test non-standard but valid versions (if any exist); verify fallback to older layout

#### 11. **BlobSharedKeyAuthenticator Secondary Endpoint Detection** (blob_shared_key_authenticator.rs:130)
- **TS behavior**: `authenticationPath?.indexOf(account) === 1`
- **Rust translation**: Check if path starts with "/" + account
- **Risk**: Off-by-one in string indexing
- **Test focus**: Pass authenticationPath="/account-secondary/container/blob", verify secondary flag used

#### 12. **StorageError Extra Fields XML Order** (storage_error.rs:98)
- **TS behavior**: JSON object iteration order (insertion order in modern JS)
- **Rust behavior**: BTreeMap iteration order (lexicographic key order)
- **Risk**: XML element order differs from TS
- **Test focus**: For helpers with multiple extra fields (getMd5Mismatch, getInvalidQuery*), verify XML element order matches TS alphabetically (BTreeMap is deterministic)

#### 13. **ContainerSASPermissions.Any Field Identification** (container_sas_permissions.rs:20)
- **TS definition**: Literal string "AnyPermission"
- **Rust definition**: Check if enum variant or string constant
- **Test focus**: Verify Any is NOT treated as a permission character; special-cased in validation

#### 14. **BlobSASAuthenticator Saved Policy Field Override Scope** (blob_sas_authenticator.rs:~300)
- **TS behavior**: ONLY startTime, expiryTime, permissions override from ACL; NOT protocol, IP, response headers, encryption scope
- **Rust code**: Confirm selectivity of override
- **Test focus**: Fetch ACL, verify protocol/IP/ses/rscc not overridden

---

## 5. TEST COVERAGE CHECKLIST

### Phase 6: Errors & Context

- [ ] StorageError
  - [ ] XML escaping of special characters in error code/message
  - [ ] Timestamp generation at construction (RFC3339 with 'Z' and millis)
  - [ ] Headers always set (x-ms-error-code, x-ms-request-id)
  - [ ] Extra XML elements appear as siblings
  - [ ] Content-Type is always "application/xml"

- [ ] StorageErrorFactory
  - [ ] Each of 74 factory methods returns correct status code
  - [ ] Error codes and messages match TS exactly (test snapshot)
  - [ ] Default context IDs match TS (mix of DEFAULT_ID and empty)
  - [ ] getMd5Mismatch() includes both MD5 values in XML
  - [ ] getInvalidPageRange2() injects Content-Range header
  - [ ] getInvalidTag() returns "DuplicateTagNames" (not "InvalidTag")
  - [ ] getInvalidQueryParameterValue() conditional extra fields
  - [ ] Snapshot-related helpers (3 variants) return correct codes

- [ ] NotImplementedError & NotImplementedinSQLError
  - [ ] Return 501 status
  - [ ] Request ID defaults to ""
  - [ ] Error code is "APINotImplemented"
  - [ ] Messages differ (generic vs SQL-scoped)

- [ ] StrictModelNotSupportedError
  - [ ] Returns 500 status
  - [ ] Error code is "FeatureNotSupported"
  - [ ] Feature name interpolated in message
  - [ ] Contains "--loose" and "Loose" guidance

- [ ] BlobStorageContext
  - [ ] All getters/setters work correctly (account, container, blob, authenticationPath, xMsRequestID, loose, isSecondary, disableProductStyleUrl)
  - [ ] xMsRequestID is alias for contextId
  - [ ] Setting to None inserts Null value
  - [ ] Can pass to functions expecting &Context (Deref)
  - [ ] Clone behavior (shared or independent mutations)

### Phase 7: Authentication

- [ ] IAuthenticator trait
  - [ ] Async method signature
  - [ ] Tri-state return (Some(true), Some(false), None)

- [ ] BlobSASPermissions & ContainerSASPermissions
  - [ ] All enum values and wire characters present
  - [ ] Casing preserved (execute/permanentDelete lowercase)
  - [ ] Any/AnyPermission sentinel not treated as permission char

- [ ] BlobSASResourceType
  - [ ] Container="c", Blob="b", BlobSnapshot="bs"

- [ ] IBlobSASSignatureValues & Signature Generators
  - [ ] All 24 struct fields present
  - [ ] Version dispatch (2025-07-05 → 2020-12-06 → 2018-11-09 → 2015-04-05)
  - [ ] Canonical name /blob/{account}/{container}[/{blob}] (no URL encoding)
  - [ ] String-to-sign layout matches TS for each version
  - [ ] Empty fields preserved in signature
  - [ ] Blob vs BlobSnapshot handling differs in UDK paths
  - [ ] Service SAS: both permissions and expiryTime optional if identifier present
  - [ ] UDK: permissions and expiryTime required even with identifier

- [ ] OperationBlobSASPermission & OperationAccountSASPermission
  - [ ] Permission tables complete and correct
  - [ ] ANY-character matching (not ALL-character)
  - [ ] Sentinel handling (Any = "has content")
  - [ ] Empty permission string always fails
  - [ ] Snapshot uses CONTAINER_PERMISSIONS, not BLOB_PERMISSIONS

- [ ] BlobSharedKeyAuthenticator
  - [ ] Authorization header check (must start with "SharedKey")
  - [ ] Account lookup (unknown → ResourceNotFound)
  - [ ] Reject GetUserDelegationKey
  - [ ] String-to-sign: method\n + standard headers\n + x-ms-headers\n + resource
  - [ ] Content-Length: 0 returns "" (not "0")
  - [ ] Canonical headers: lowercase name, trimLeft value, sorted
  - [ ] Canonical resource: lowercase query keys, decode values
  - [ ] Key1 and key2 both tested
  - [ ] Secondary endpoint "-secondary" suffix insertion

- [ ] AccountSASAuthenticator
  - [ ] All required fields (sv, se, sp, ss, srt, sig) enforced
  - [ ] Strict mode rejects ses with StrictModelNotSupportedError
  - [ ] Signature validation before other checks
  - [ ] Time, IP, protocol checks (stubs)
  - [ ] Operation lookup in OPERATION_ACCOUNT_SAS_PERMISSIONS
  - [ ] Existing blob Write requirement for create/copy
  - [ ] Blob existence check (uncommitted = nonexistent)

- [ ] BlobSASAuthenticator
  - [ ] Resource type validation (c/b/bs only)
  - [ ] Identifier-based ACL policy override (only sp/st/se)
  - [ ] UDK path: require six signed fields + signedService="b"
  - [ ] UDK signature immediate validation (no fallback)
  - [ ] Service SAS key1/key2 fallback
  - [ ] Response override fields NOT decoded (rscc, rscd, rsce, rscl, rsct)
  - [ ] Time, IP, protocol checks
  - [ ] Permission table routing (Blob → BLOB_PERMISSIONS, Container/BlobSnapshot → CONTAINER_PERMISSIONS)
  - [ ] Existing blob Write requirement

- [ ] BlobTokenAuthenticator
  - [ ] Bearer prefix extraction (exactly 7 + 1 chars)
  - [ ] HTTPS requirement (reject HTTP)
  - [ ] JWT decode without verification
  - [ ] Required claims: nbf, exp, iat
  - [ ] Issuer validation against VALID_ISSUE_PREFIXES
  - [ ] Audience regex match + capture validation
  - [ ] Time validation uses context.startTime
  - [ ] OAuth level dispatch (BASIC only)

- [ ] PublicAccessAuthenticator
  - [ ] Container name requirement (skip if absent)
  - [ ] Metadata lookup (errors swallowed)
  - [ ] Operation allowlist (CONTAINER_PUBLIC_READ_OPERATIONS, BLOB_PUBLIC_READ_OPERATIONS)
  - [ ] Not-applicable for known access type but non-allowlisted operation

---

## 6. TEST DATA & FIXTURES

### Account Keys
- Primary: `"MTAwCjE2NQoyMjUKMTAzCjIxOAoyNDEKNDAKNzgKMTkxCjE3OAoyMTQKMTY5CjIxMwo2MQoyNTIKMTQxCg=="` (devstoreaccount1)
- Secondary: `"testing_key"`
- Invalid: any other key

### Account Names
- Valid: `"devstoreaccount1"`, `"devstoreaccount2"`
- Invalid: `"invalid"`, `"unknown"`

### Containers & Blobs
- Test containers: `"test-container"`, `"public-container"`
- Test blobs: `"test-blob"`, `"test-snapshot"`
- Block blobs: `"block-blob"` (committed)
- Uncommitted block blobs: `"uncommitted-block"`

### SAS Versions
- Current: `"2020-12-06"`, `"2025-07-05"`
- Legacy: `"2018-11-09"`, `"2015-04-05"`
- Invalid: `"2020-01-01"` (should fall through to older layout)

### Error Response Examples
```xml
<Error>
  <Code>ContainerNotFound</Code>
  <Message>The specified container does not exist.
RequestId:test-id
Time:2024-01-15T10:30:45.123Z</Message>
</Error>
```

---

## 7. INTEGRATION POINTS WITH OTHER PHASES

- **Phase 5.7 (Context)**: BlobStorageContext wraps generated Context
- **Phase 5.6 (XML)**: StorageError uses jsonToXML (via quick_xml in Rust)
- **Phase 5.11 (Operation)**: Permission tables keyed by Operation enum
- **Phase 4.2 (Crypto)**: HMAC-SHA256 for all signatures
- **Phase 3.x (Common Auth)**: DateOrString, SASProtocol, AccountSAS helpers
- **Phase 2 (Persistence)**: IBlobMetadataStore for access policies and blob existence checks

---

## 8. REFERENCES TO SOURCE CODE

### Errors Module
- TS: `/home/azureuser/Azurite/src/blob/errors/StorageError.ts` (lines 1–65)
- TS: `/home/azureuser/Azurite/src/blob/errors/StorageErrorFactory.ts` (lines 1–854)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/storage_error.rs` (lines 1–110)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/errors/storage_error_factory.rs` (lines 1–680)

### Context Module
- TS: `/home/azureuser/Azurite/src/blob/context/BlobStorageContext.ts` (lines 1–73)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/context/blob_storage_context.rs` (lines 1–137)

### Authentication Module
- TS: `/home/azureuser/Azurite/src/blob/authentication/IAuthenticator.ts` (lines 1–6)
- TS: `/home/azureuser/Azurite/src/blob/authentication/IBlobSASSignatureValues.ts` (lines 1–818)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/i_authenticator.rs` (lines 1–14)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/i_blob_sas_signature_values.rs` (lines 1–479)

### All Authenticators
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_shared_key_authenticator.rs` (277 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/account_sas_authenticator.rs` (276 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_sas_authenticator.rs` (449 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_token_authenticator.rs` (194 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/public_access_authenticator.rs` (125 lines)

### Existing Tests
- TS: `/home/azureuser/Azurite/tests/blob/authentication.test.ts` (153 lines)
- TS: `/home/azureuser/Azurite/tests/blob/sas.test.ts` (2398 lines)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob_parity.rs` (empty scaffold)
- RS: `/home/azureuser/Azurite/rust/crates/azurite-blob/tests/blob/generated_framework.rs` (500+ lines, utilities)
___BEGIN___COMMAND_DONE_MARKER___0
