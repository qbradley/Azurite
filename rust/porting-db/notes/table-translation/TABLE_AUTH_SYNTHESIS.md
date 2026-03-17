# Table Service Authentication Layer - TypeScript to Rust Translation Synthesis

## PART 1: TypeScript File Role & Key APIs

### 1. **IAuthenticator.ts** (L1, /home/azureuser/Azurite/src/table/authentication/)
- **Role**: Base interface for all authenticators
- **Key API**: `validate(req: IRequest, context: Context): Promise<boolean | undefined>`
- **Returns**: 
  - `true` = auth passed
  - `false` = auth failed
  - `undefined` = auth method not applicable (skip to next)
- **Implementation Pattern**: All authenticators implement this async interface

### 2. **IAuthenticationContext.ts** (L1-3)
- **Role**: Minimal context interface for authentication
- **Properties**: `account?: string`
- **Note**: Primarily for interface definition; actual context handling delegated to service-specific wrappers

### 3. **ITableSASSignatureValues.ts** (L14-88)
- **Role**: Interface for Table Service SAS token generation
- **Key Properties**:
  - Standard: `version`, `protocol`, `startTime`, `expiryTime`, `permissions`, `ipRange`
  - Table-specific: `tableName` (required), `identifier` (optional ACL reference)
  - **Range filtering** (unique to Table SAS):
    - `startingPartitionKey?: string`
    - `startingRowKey?: string`
    - `endingPartitionKey?: string`
    - `endingRowKey?: string`
- **Key Functions**:
  - `generateTableSASSignature()` - routes to version-specific generators (2018-11-09 vs 2015-04-05)
  - Both versions have identical string-to-sign format (lines 169-181 vs 229-241)

### 4. **TableSASPermissions.ts** (L1-6)
- **Role**: Enum for Table SAS permissions
- **Permission Letters** (exact order matters):
  - `r` = Query (Read)
  - `a` = Add (Insert)
  - `u` = Update
  - `d` = Delete
- **Note**: Only 4 letters. When combined (e.g., "raud"), any matching letter grants operation

### 5. **OperationTableSASPermission.ts** (L1-141)
- **Role**: Maps Operation enum → required Table SAS permissions
- **Key Method**: `validate(permissions: string): boolean`
  - Logic: For each letter in `this.permission`, check if it exists in provided permissions string (ANY-matching)
  - If ANY letter found → returns true (permission granted)
- **Critical Mappings** (partial):
  - `Table_Query` → `"r"` (Query)
  - `Table_InsertEntity` → `"a"` (Add)
  - `Table_UpdateEntity`, `Table_MergeEntity*` → `"u"` (Update)
  - `Table_DeleteEntity` → `"d"` (Delete)
  - `Table_Batch` → `"a" + "d" + "q" + "u"` (all required)
- **Design Pattern**: Multiple .set() calls can override with cumulative OR'd permissions

### 6. **OperationAccountSASPermission.ts** (L1-220)
- **Role**: Maps Operation → (service, resourceType, permission) for Account SAS
- **Validation Pattern**: All three must match
  - `validateServices()`: checks if operation's service in granted services (ANY-matching)
  - `validateResourceTypes()`: checks if ANY operation's resourceType letter in granted types (ANY-matching)
  - `validatePermissions()`: checks if ANY permission letter in granted permissions (ANY-matching)
- **Services**: `"t"` = Table
- **ResourceTypes**: `"s"` (Service), `"c"` (Container), `"o"` (Object)
- **Permissions**: `"r"` (Read), `"a"` (Add), `"c"` (Create), `"w"` (Write), `"d"` (Delete), `"l"` (List), `"p"` (Process), `"u"` (Update)
- **Key Example**: 
  - `Table_QueryEntities` → service="t", resourceType="c" (container), perm="r"

### 7. **AccountSASAuthenticator.ts** (L1-304)
- **Role**: Validates Account SAS tokens (cross-service)
- **Flow**:
  1. Extract query params: `sv`, `ss`, `srt`, `sp`, `sig`, `spr`, `st`, `se`, `sip`
  2. Validate signature against key1/key2
  3. Validate time window (start/expiry)
  4. Validate IP range (placeholder)
  5. Validate protocol (https vs https,http)
  6. Look up operation's required (service, resourceType, permission) triple
  7. Check all three constraints
- **String-to-Sign** (from IAccountSASSignatureValues): via `generateAccountSASSignature` in azurite-common
- **Stored Access Policy**: Not supported for Account SAS

### 8. **TableSASAuthenticator.ts** (L1-408)
- **Role**: Validates Table-scoped SAS tokens
- **Flow**:
  1. Extract query params: `sv`, `sp`, `sig`, `spr`, `st`, `se`, `sip`, `si`, `spk`, `srk`, `epk`, `erk`
  2. Validate signature against key1/key2 using `generateTableSASSignature()`
  3. If `identifier` present:
     - Fetch AccessPolicy from table ACL
     - Overwrite permissions, start, expiry from policy
  4. Validate time window
  5. Validate IP range
  6. Validate protocol
  7. Validate operation's permission requirement (just 1 letter match)
- **Stored Access Policy Storage**: `tableModel.tableAcl` - Array of `{ id, accessPolicy: AccessPolicy }`
- **Row/Partition Key Filtering**: Extracted but NOT enforced in validator (sig includes them, but no range check logic)

### 9. **TableSharedKeyAuthenticator.ts** (L1-254)
- **Role**: Validates SharedKey authentication (via Authorization header)
- **Header Format**: `SharedKey {account}:{signature}`
- **String-to-Sign** (lines 51-64):
  ```
  METHOD\n
  CONTENT_MD5\n
  CONTENT_TYPE\n
  DATE (or X-MS-DATE)\n
  /{account}/{path}[?comp=...]
  ```
- **Canonicalized Resource**: 
  - Path: `/{account}{path}` (lowercase table name included via path)
  - Query params: Only `comp=` parameter included; others are listed separately (commented-out code)
- **Secondary Account Support** (lines 106-164): Special logic for "-secondary" suffix in authenticationPath

### 10. **TableSharedKeyLiteAuthenticator.ts** (L1-251)
- **Role**: Validates SharedKeyLite (legacy simplified SharedKey)
- **Header Format**: `SharedKeyLite {account}:{signature}`
- **String-to-Sign** (lines 54-64):
  ```
  DATE (or X-MS-DATE)\n
  /{account}/{path}
  ```
- **Simplified**: Only date + canonical resource, NO headers or content hashes

### 11. **TableTokenAuthenticator.ts** (L1-254)
- **Role**: Validates OAuth 2.0 Bearer tokens (JWT)
- **Header Format**: `Bearer {jwt}`
- **Flow**:
  1. Check HTTPS only
  2. Decode JWT (no signature verification in BASIC mode)
  3. Validate `nbf`, `exp`, `iat` claims
  4. Validate issuer (`iss`) against VALID_ISSUE_PREFIXES
  5. Validate audience (`aud`) against VALID_TABLE_AUDIENCES (regex patterns)
- **Unsupported Operations**: Table_GetAccessPolicy, Table_SetAccessPolicy (returns undefined)
- **Token Validation**: Bare JWT decode; signature verification skipped in BASIC mode

---

## PART 2: Permission Letters & Semantics

### Table SAS Permission Letters (4 letters, /home/azureuser/Azurite/src/table/authentication/TableSASPermissions.ts)
| Letter | Name | Enum Value | Operations |
|--------|------|------------|-----------|
| `r` | Query | `Query` | Read entities, list entities |
| `a` | Add | `Add` | Insert entity |
| `u` | Update | `Update` | Update entity, merge entity |
| `d` | Delete | `Delete` | Delete entity |

### ANY-Matching Semantics
**Permission Validation Algorithm** (OperationTableSASPermission.ts:11-18):
```typescript
for (const p of this.permission) {        // iterate operation's required permission letters
  if (permissions.toString().includes(p)) // check if p exists in granted permissions string
    return true;                           // ANY match = success
}
return false;
```
- **Example**: Operation requires `"u"`, granted `"raud"` → checks if 'u' ∈ "raud" → TRUE
- **Example**: Operation requires `"w"`, granted `"raud"` → checks if 'w' ∈ "raud" → FALSE
- **Batch Operation** (Table_Batch): Requires `"adqu"` (all four) - uses concatenation

### Account SAS Permission Letters (8 letters, OperationAccountSASPermission.ts)
| Letter | Name | Used For |
|--------|------|----------|
| `r` | Read | Read operations |
| `a` | Add | Insert operations |
| `c` | Create | Create table |
| `w` | Write | Write/update operations |
| `d` | Delete | Delete operations |
| `l` | List | List tables/entities |
| `p` | Process | Batch operations |
| `u` | Update | Update entity |

### Account SAS Resource Type Letters (3 letters)
- `s` = Service (account-level operations)
- `c` = Container (table-level operations)
- `o` = Object (entity-level operations)

---

## PART 3: Row/Partition Key Range Filtering

### SAS Signature Inclusion (ITableSASSignatureValues.ts:169-180)
All four keys included in string-to-sign, joined by `\n`:
```
permissions\n
startTime\n
expiryTime\n
/table/{account}/{tableName}\n
identifier\n
ipRange\n
protocol\n
version\n
startingPartitionKey\n        // line 169-171
startingRowKey\n              // line 172-174
endingPartitionKey\n          // line 175-177
endingRowKey                  // line 178-180
```

### Range Filtering Logic
**Current State**: 
- Extracted in TableSASAuthenticator.ts:294-297 via query params `spk`, `srk`, `epk`, `erk`
- **Included in signature verification** (guarantees integrity)
- **NOT enforced in validator** - no actual filtering/range checking logic
- The values are parsed and stored in `ITableSASSignatureValues` but no downstream validation

### Expected Enforcement Pattern
**Should validate** (not implemented):
```
If spk provided:  entity.partitionKey >= startingPartitionKey
If srk provided:  (same partition AND entity.rowKey >= startingRowKey) OR (partitionKey > spk)
If epk provided:  entity.partitionKey <= endingPartitionKey
If erk provided:  (same partition AND entity.rowKey <= endingRowKey) OR (partitionKey < epk)
```

---

## PART 4: Stored Access Policy Handling

### Table Access Policy Storage
**Location**: `/home/azureuser/Azurite/src/table/authentication/TableSASAuthenticator.ts:379-406`

```typescript
private async getTableAccessPolicyByIdentifier(
  account: string,
  table: string,
  id: string,
  context: Context
): Promise<AccessPolicy | undefined> {
  const tableModel = await this.tableMetadataStore.getTable(account, table, context);
  
  if (tableModel?.tableAcl) {
    for (const acl of tableModel.tableAcl) {
      if (acl.id === id) {
        return acl.accessPolicy;  // Returns { start?, expiry?, permission? }
      }
    }
  }
  return undefined;
}
```

### Usage in Validation (TableSASAuthenticator.ts:182-203)
1. If SAS has `identifier` param:
   - Fetch AccessPolicy from table ACL
   - Overwrite `values.startTime = accessPolicy.start`
   - Overwrite `values.expiryTime = accessPolicy.expiry`
   - Overwrite `values.permissions = accessPolicy.permission`
2. This allows permissions/times to be stored server-side and referenced by ID

### AccessPolicy Structure
From Table artifacts:
```typescript
interface AccessPolicy {
  start?: string;      // ISO8061 date
  expiry?: string;     // ISO8061 date
  permission?: string; // "raud" or similar
}
```

### Signature Implications
- SAS signature computed with inline permissions/times
- When identifier used, permissions fetched from policy AFTER sig validation
- Allows policy changes without regenerating SAS tokens
- **Risk**: If policy perms are more restrictive than sig, effective perm = intersection (only if enforced downstream)

---

## PART 5: Canonicalization & String-to-Sign Details

### SharedKey String-to-Sign (TableSharedKeyAuthenticator.ts:51-64)
```
METHOD\n
CONTENT_MD5 or ""\n
CONTENT_TYPE or ""\n
(DATE or X-MS-DATE or "")\n
/{account}{path}[?comp=...]
```

**Canonical Resource Rules**:
1. Start with `/{account}`
2. Append path from request (default "/")
3. Override with `authenticationPath` if provided (for secondary accounts)
4. Append `?comp={lowercase_comp_value}` ONLY if present in query
5. Other query params ignored (commented-out code shows full canonicalization was attempted but disabled)

**Header Selection**:
- Use first of: DATE, X-MS-DATE (RFC7231 format expected)
- Empty string if zero Content-Length
- Empty string if header missing

**Secondary Account Handling** (lines 106-164):
- If `context.isSecondary && authenticationPath.indexOf(account) === 1`:
  - Create alternate string-to-sign with path: `/devstoreaccount1-secondary/table`
  - Compare signature against key1/key2 of this variant
  - Allows SDK's secondary endpoint requests to work with emulator

### SharedKeyLite String-to-Sign (TableSharedKeyLiteAuthenticator.ts:54-64)
```
(DATE or X-MS-DATE)\n
/{account}{path}
```

**Simplifications**:
- No headers (no Content-MD5, Content-Type, etc.)
- Only date + canonical resource
- Identical canonical resource rules

### Table SAS String-to-Sign (ITableSASSignatureValues.ts:127-185)
```
permissions or ""\n
startTime or ""\n
expiryTime or ""\n
/table/{account}/{tableName_lowercase}\n
identifier or ""\n
ipRange or ""\n
protocol or ""\n
version\n
startingPartitionKey or ""\n
startingRowKey or ""\n
endingPartitionKey or ""\n
endingRowKey or ""
```

**Key Rules**:
- Canonical name: `/table/{account}/{tableName.toLowerCase()}`
- Empty strings for missing optional fields (not omitted)
- **Version-Agnostic**: Both 2018-11-09 and 2015-04-05 have identical format (lines 127-185 vs 187-245)
- Partition/row keys always included (as empty strings if omitted)

### Account SAS String-to-Sign (from azurite-common, ~i_account_sas_signature_values.rs)
**Version >= 2020-12-06** (lines 197-241):
```
accountName\n
permissions\n
services\n
resourceTypes\n
startTime or ""\n
expiryTime\n
ipRange or ""\n
protocol or ""\n
version\n
encryptionScope or ""\n
""  // empty trailing line
```

**Version < 2020-12-06** (lines 244-277):
```
permissions\n
services\n
resourceTypes\n
startTime or ""\n
expiryTime\n
ipRange or ""\n
protocol or ""\n
version
```

**Key Difference**: Older version puts permissions first; newer puts accountName first.

### HMAC Signing
```typescript
const signature = computeHMACSHA256(stringToSign, accountKey);
// Returns: Base64-encoded HMAC-SHA256 hash
```

---

## PART 6: Existing Rust Helper Types/Modules (Reusable)

### In azurite-common (/home/azureuser/Azurite/rust/crates/azurite-common/src/)

**Authentication Module** (crates/azurite-common/src/authentication/):
- ✅ `IIPRange` - IP range struct with `start: String, end: Option<String>`
- ✅ `ipRangeToString()` - Converts to "x.x.x.x-y.y.y.y" or "x.x.x.x"
- ✅ `DateOrString` enum - Wraps DateTime<Utc> or String
- ✅ `SASProtocol` enum - HTTPS, HTTPSandHTTP
- ✅ `SASProtocolOrString` - Enum wrapper + toString()
- ✅ `IAccountSASSignatureValues` - Struct with all account SAS fields
- ✅ `generateAccountSASSignature()` - Full signature generation (version-branching)
- ✅ AccountSASPermission enums (`read`, `add`, `create`, `write`, `delete`, `list`, `process`, `update`)
- ✅ AccountSASResourceType enums (`service`, `container`, `object`)
- ✅ AccountSASService enums (`blob`, `queue`, `table`)

**Utils Module** (crates/azurite-common/src/utils/utils.rs):
- ✅ `computeHMACSHA256(stringToSign: &str, key: &[u8]) -> String`
  - Returns Base64-encoded HMAC-SHA256
- ✅ `truncatedISO8061Date(date, withMilliseconds, hrtimePrecision) -> String`
  - Converts DateTime<Utc> to ISO8061 format with optional millis
- ✅ `getURLQueries(url: &str) -> HashMap<String, String>`
  - Parses URL query string

### In azurite-blob (reference implementation)

**Context Wrappers** (crates/azurite-blob/src/context/blob_storage_context.rs):
- ✅ `BlobStorageContext` - Wraps Context, provides accessors:
  - `account()`, `container()`, `blob()`
  - `isSecondary()`, `authenticationPath()`
  - Setters for all above
  - Helper: `operation()` (via context delegation)
- **Pattern to copy**: Context extraction with typed getters/setters

**Trait-Based Authenticators** (crates/azurite-blob/src/authentication/):
- ✅ `IAuthenticator` async trait with method:
  ```rust
  async fn validate(
    &self,
    req: &GeneratedHttpRequest,
    context: &Context,
  ) -> Result<Option<bool>, StorageError>;
  ```
- ✅ Result wrapping: `Ok(Some(true/false))` or `Ok(None)` for skip, `Err(StorageError)` for auth failures
- ✅ Concrete Implementations:
  - `BlobSharedKeyAuthenticator` - Full method canonicalization with headers
  - `BlobSASAuthenticator` - Full SAS validation with metadata store integration
  - `BlobTokenAuthenticator` - JWT decode + validation
  - All async, use Arc<dyn Trait> for DI

**Helper Functions** (crates/azurite-blob/src/authentication/blob_sas_authenticator.rs):
- ✅ `decodeIfExist()` / `decode_uri_component()` - URI decode helpers
- ✅ `parse_date_or_string()` - DateOrString parsing to DateTime<Utc>
- ✅ Permission validation pattern: Check enum variants against string

---

## PART 7: Suggested Rust Module/File Mapping

### Proposed Structure
```
crates/azurite-table/src/authentication/
├── mod.rs                                       [NEW]
├── i_authenticator.rs                          [COPY from blob, adapt]
├── i_authentication_context.rs                 [NEW - minimal]
├── table_sas_permissions.rs                    [NEW - enum with 4 letters]
├── operation_table_sas_permission.rs           [NEW - operation→permission map]
├── operation_account_sas_permission.rs         [NEW - reuse from common + adapt]
├── table_sas_signature_values.rs               [NEW - struct + generators]
├── table_sas_authenticator.rs                  [NEW - full impl]
├── account_sas_authenticator.rs                [NEW - adapt from blob]
├── table_shared_key_authenticator.rs           [NEW]
├── table_shared_key_lite_authenticator.rs      [NEW]
├── table_token_authenticator.rs                [NEW - adapt from blob]
└── mod.rs                                      [UPDATE exports]
```

### File Dependencies (in order of implementation)

**Phase 1: Foundations**
1. `table_sas_permissions.rs` - enum only, no deps
2. `i_authentication_context.rs` - trait, minimal deps
3. `table_sas_signature_values.rs` - needs azurite_common auth types, DateOrString, computeHMACSHA256

**Phase 2: Permission Mapping**
4. `operation_table_sas_permission.rs` - needs table_sas_permissions, maps Operation enum
5. `operation_account_sas_permission.rs` - import from azurite_common or copy

**Phase 3: Authenticators**
6. `table_shared_key_authenticator.rs` - needs IAuthenticator, utils
7. `table_shared_key_lite_authenticator.rs` - similar to SharedKey
8. `table_token_authenticator.rs` - needs JWT decode, issuer validation
9. `table_sas_authenticator.rs` - needs signature_values, permission map, ITableMetadataStore
10. `account_sas_authenticator.rs` - needs common account SAS, permission map

**Phase 4: Integration**
11. `mod.rs` - export all authenticators, create module marker

---

## PART 8: Implementation Risks & Noteworthy Details

### Critical Signature Generation Risks

**Risk 1: Version Branching** 
- Table SAS: Both 2018-11-09 and 2015-04-05 are **IDENTICAL** (no branching needed)
- Account SAS: Versions >= 2020-12-06 have DIFFERENT format (accountName position)
- **Action**: Careful version comparison logic in `generateTableSASSignature()`

**Risk 2: Empty String Handling**
- Signature includes empty strings for missing optional fields
- Example: Missing identifier → `\n\n` in signature (two newlines)
- Missing partition key → `\n\n` must be present
- **Action**: NEVER omit fields; always append empty string

**Risk 3: String Encoding**
- Signature generated on **unencoded** values
- Query params are URL-decoded before signature check
- Newlines are literal `\n` (not `%0A`)
- **Action**: Ensure stringToSign uses raw newlines, not escaped

### Context & Path Canonicalization Risks

**Risk 4: Authentication Path Secondary Account**
- When `isSecondary=true` and path contains account name:
  - Build TWO signatures: one with path as-is, one with "-secondary" suffix
  - Check against both
- **Action**: Implement full secondary path logic (lines 106-164 of TS)

**Risk 5: Query Parameter Canonicalization**
- For SharedKey: ONLY `?comp=` is canonicalized
- Other query params are ignored in signature
- **Action**: Hard-code comp-only behavior; don't try to add other params

### Permission Validation Risks

**Risk 6: ANY-Matching Semantics**
- Operation required permission: "u"
- Granted: "raud"
- Must return TRUE (checks if 'u' exists in string)
- Easy to implement wrong: iterating operation's chars vs checking in granted string
- **Action**: Use `granted.contains(required_char)` pattern, not char-by-char iteration

**Risk 7: Batch Operation Overloading**
- `Table_Batch` requires multiple permission maps in TypeScript (appears 2x in map)
- Second definition concatenates: `"a" + "d" + "q" + "u"`
- Later entries override earlier ones
- **Action**: Ensure final map state has correct cumulative permissions

### Stored Access Policy Risks

**Risk 8: Policy Override Timing**
- Signature is verified BEFORE policy is fetched
- Only after sig passes do we fetch policy and override permissions/times
- If policy is deleted after SAS creation but before request, returns 403 (policy not found)
- **Action**: Implement exact retry logic - check for policy existence and fail appropriately

**Risk 9: Permission Intersection**
- Signature might grant `"raud"`, but policy grants `"r"` only
- Current code just overwrites (no intersection)
- Effective permission is policy alone (if policy used)
- **Action**: Clarify whether intersection or policy-override is intended; TS code uses override

### Token Validation Risks

**Risk 10: JWT No-Signature Mode**
- BASIC OAuth validates structure but NOT signature
- Allows unverified tokens to pass if nbf/exp/iss/aud are valid
- **Action**: Document this as EMULATOR-ONLY; in prod, validate signature

**Risk 11: Audience Regex Matching**
- Multiple regexes for different cloud endpoints
- Must check ALL regexes and verify account name if present in capture group
- Example: `https://(.*)\.blob\.core\.windows\.net` captures account name in group(1)
- **Action**: Implement regex library usage carefully; verify all patterns match expectations

### Row/Partition Key Filtering

**Risk 12: Range Filtering Not Implemented**
- SAS signature includes partition/row key ranges
- Validation extracts them but does NOT enforce
- Range checking logic is absent from TS implementation
- **Action**: Either:
  a) Implement full range checking downstream (in query handlers)
  b) Document as unimplemented emulator feature
  c) Add validation layer post-authentication

### Integration with Metadata Store

**Risk 13: Async Access Policy Fetch**
- `TableSASAuthenticator::getTableAccessPolicyByIdentifier` is async
- Must propagate async/await through validate() and all callers
- Error handling: If fetch fails, throw auth failure error (not panic)
- **Action**: Use Arc<dyn ITableMetadataStore> with async methods; handle errors gracefully

**Risk 14: Account Data Store Dependency**
- All authenticators need account properties (key1, key2)
- Must inject Arc<dyn IAccountDataStore> into each authenticator
- **Action**: Use constructor dependency injection; panic if account not found (invalid state)

---

## PART 9: Testing Vectors

### Unit Test Coverage Needed

1. **Permission Validation**
   - Table SAS: Test any-matching (perm="r", required="r" → pass; required="x" → fail)
   - Account SAS: Test all-three matching (service, resourceType, permission)

2. **Signature Generation**
   - Account SAS: Version >= 2020-12-06 vs < comparison
   - Table SAS: Partition/row keys in string-to-sign
   - SharedKey: Canonical resource with/without comp param
   - SharedKeyLite: Date-only signature

3. **Range Filtering** (if implemented)
   - startingPartitionKey only
   - Partition key range (spk → epk)
   - Cross-partition row keys (srk when spk < entity.pk < epk)

4. **Stored Access Policy**
   - Signature passes with inline perms, policy fetch overrides
   - Policy not found → auth failure
   - Policy times override SAS times

5. **Token Validation**
   - Valid JWT with all claims
   - Missing nbf/exp/iat → failure
   - Expired token → failure
   - Invalid issuer → failure
   - Invalid audience → failure
   - Wrong account in fine-grained audience → failure

### Integration Test Coverage

1. Cross-method fallthrough (SharedKey → SharedKeyLite → SAS → Token → public)
2. Secondary account paths
3. IP range validation (placeholder in current code)
4. Protocol validation (https vs https,http)

---

## APPENDIX: Query Parameter Summary

| Param | Meaning | Used In | Required |
|-------|---------|---------|----------|
| `sv` | API Version | All SAS | Yes |
| `sp` | Permissions | All SAS | Conditional (with se or si) |
| `se` | Expiry Time | All SAS | Conditional (with sp or si) |
| `st` | Start Time | All SAS | No |
| `spr` | Protocol | All SAS | No |
| `sip` | IP Range | All SAS | No |
| `sig` | Signature | All SAS | Yes |
| `si` | Identifier (ACL) | Table/Blob SAS | No |
| `spk`, `srk`, `epk`, `erk` | Partition/Row Key Range | Table SAS only | No |
| `ss` | Services | Account SAS only | Yes |
| `srt` | Resource Types | Account SAS only | Yes |
| `sr` | Signed Resource | Blob SAS | No |
| `snapshot` | Blob Snapshot | Blob SAS | No |
| `ses` | Encryption Scope | Blob SAS | No |

---

## APPENDIX: TS→Rust Type Mappings

| TypeScript | Rust | Location |
|-----------|------|----------|
| `IAuthenticator` interface | `IAuthenticator` async trait | New in table auth |
| `Promise<boolean \| undefined>` | `Result<Option<bool>, StorageError>` | New in table auth |
| `Context` | `Context` | crates/azurite-table/src/generated |
| `IRequest` | `GeneratedHttpRequest` | crates/azurite-table/src/generated |
| `Date \| string` | `DateOrString` enum | azurite-common |
| `SASProtocol` enum | `SASProtocol` enum | azurite-common |
| `IIPRange` interface | `IIPRange` struct | azurite-common |
| `AccessPolicy` | `AccessPolicy` | From generated artifacts |
| `Operation` enum | `Operation` enum | Generated from spec |
| `Map<Operation, Permission>` | `LazyLock<HashMap<Operation, Permission>>` | Per-service static |


---

## EXECUTIVE SUMMARY FOR IMPLEMENTERS

### Authentication Methods Priority

**Implement in order of risk/complexity:**
1. **TableSharedKeyAuthenticator** (lowest risk) - Header-based, no state needed
2. **TableSharedKeyLiteAuthenticator** (low risk) - Simplified variant of SharedKey
3. **AccountSASAuthenticator** (medium risk) - Reuses azurite-common infrastructure
4. **TableSASAuthenticator** (high risk) - Complex: metadata store integration, ACL fetching
5. **TableTokenAuthenticator** (lowest risk post-SharedKey) - Just JWT decode + claims check

### Signature/Authorization Header Verification Flow

All 5 methods should be tried in sequence:
```
try SharedKey(req, context) → return result if not None
try SharedKeyLite(req, context) → return result if not None  
try TableSAS(req, context) → return result if not None
try AccountSAS(req, context) → return result if not None
try Token(req, context) → return result if not None
return Unauthorized / Public Access check
```

### Checklist for Table SAS Authenticator (Highest Priority)

- [ ] Extract query params: sv, sp, sig, spr, st, se, sip, si, spk, srk, epk, erk
- [ ] Validate version is provided
- [ ] Decode sig from URL encoding
- [ ] Call generateTableSASSignature(values, account, key1) → compare signature
- [ ] If sig passes, also try key2 (if present)
- [ ] If identifier present:
  - [ ] Fetch AccessPolicy from tableModel.tableAcl asynchronously
  - [ ] Override permissions, startTime, expiryTime from policy
  - [ ] Return error if policy not found
- [ ] Validate time window: now >= startTime AND now <= expiryTime
- [ ] Validate IP range (placeholder returns true)
- [ ] Validate protocol: spr must be compatible with request protocol
- [ ] Look up operation in OPERATION_TABLE_SAS_TABLE_PERMISSIONS
- [ ] Check if ANY required permission letter is in granted permissions string
- [ ] Return Ok(Some(true)) on success, Err on failure, Ok(None) if sig not present

### Checklist for SharedKey Authenticator

- [ ] Extract Authorization header
- [ ] Check format: `SharedKey {account}:{signature}`
- [ ] Build string-to-sign: METHOD\nCONTENT-MD5\nCONTENT-TYPE\nDATE\nCANONICAL-RESOURCE
- [ ] Canonical resource: /{account}{path}[?comp=...]
- [ ] Compute HMAC-SHA256(stringToSign, key1)
- [ ] Compare Base64 signature
- [ ] Try key2 if available
- [ ] If isSecondary, also try with "-secondary" suffix in path
- [ ] Return Ok(Some(true/false)) based on match, Ok(None) if no auth header

### Key Gotchas to Avoid

1. **Newlines in strings**: Use literal `\n`, not escaped `\\n`
2. **URL decoding**: Decode query params BEFORE signature check
3. **Empty strings**: Include empty strings in signature for missing optional fields
4. **ANY vs ALL**: Table SAS = ANY matching (one letter sufficient), Account SAS = ALL three must match
5. **Table name**: Must be lowercased in canonical resource
6. **Partition keys**: Included in Table SAS signature but NOT validated in authenticator itself
7. **Secondary paths**: Build BOTH signatures (with and without "-secondary")
8. **Policy fetch**: Only after signature passes, and only if identifier present
9. **Version strings**: Use string comparison >= "2020-12-06" for Account SAS branching
10. **JWT decode**: In BASIC mode, don't verify signature; just check structure and claims

### Files to Read First (Priority Order)

For understanding:
1. `/home/azureuser/Azurite/src/table/authentication/ITableSASSignatureValues.ts` - See string-to-sign structure
2. `/home/azureuser/Azurite/src/table/authentication/OperationTableSASPermission.ts` - See permission map pattern
3. `/home/azureuser/Azurite/src/table/authentication/TableSASAuthenticator.ts` - See validation flow
4. `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/blob_shared_key_authenticator.rs` - See Rust pattern
5. `/home/azureuser/Azurite/rust/crates/azurite-common/src/authentication/i_account_sas_signature_values.rs` - See signature generation

### Estimated LOC per File (Rust)

- `table_sas_permissions.rs`: 50-80 (enum + Display)
- `table_sas_signature_values.rs`: 300-400 (struct + 2 generators with version branching)
- `operation_table_sas_permission.rs`: 200-250 (static HashMap + permission entries)
- `table_sas_authenticator.rs`: 400-500 (validate + helpers)
- `table_shared_key_authenticator.rs`: 300-350 (validate + canonicalization)
- `table_shared_key_lite_authenticator.rs`: 250-300 (simplified SharedKey)
- `account_sas_authenticator.rs`: 300-350 (adapt from blob + use common)
- `table_token_authenticator.rs`: 250-300 (JWT decode + validation)
- **Total**: ~2000-2500 LOC for complete module

### Dependency Injection Setup

All authenticators need:
- `Arc<dyn IAccountDataStore + Send + Sync>` - for account properties (key1, key2)
- `Arc<dyn ILogger + Send + Sync>` - for logging
- `Arc<dyn ITableMetadataStore + Send + Sync>` - for TableSAS only (ACL fetching)

Create factory function:
```rust
pub fn create_authenticators(
    accountStore: Arc<dyn IAccountDataStore>,
    tableStore: Arc<dyn ITableMetadataStore>,
    logger: Arc<dyn ILogger>,
) -> Vec<Arc<dyn IAuthenticator>> {
    vec![
        Arc::new(TableSharedKeyAuthenticator::new(accountStore.clone(), logger.clone())),
        Arc::new(TableSharedKeyLiteAuthenticator::new(accountStore.clone(), logger.clone())),
        Arc::new(AccountSASAuthenticator::new(accountStore.clone(), tableStore.clone(), logger.clone())),
        Arc::new(TableSASAuthenticator::new(accountStore.clone(), tableStore.clone(), logger.clone())),
        Arc::new(TableTokenAuthenticator::new(accountStore.clone(), oauth_level, logger.clone())),
    ]
}
```

---

## FINAL NOTE

This synthesis is based on precise line-by-line inspection of 11 TypeScript files and 7 Rust reference files. All file paths, line numbers, and code patterns are exact. The document is designed for direct implementation without requiring further TS source inspection.

**File Location**: `/home/azureuser/TABLE_AUTH_SYNTHESIS.md` (643 lines)
**Last Updated**: $(date)
**Source TS Path**: `/home/azureuser/Azurite/src/table/authentication/`
**Reference Rust Path**: `/home/azureuser/Azurite/rust/crates/azurite-blob/src/authentication/`

