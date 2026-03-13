# Porting Record — `src/common/authentication/IAccountSASSignatureValues.ts`

## File info
- Source path: `src/common/authentication/IAccountSASSignatureValues.ts`
- Source lines: `247`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/authentication/i_account_sas_signature_values.rs`
- Crate: `azurite-common`
- Module: `authentication::i_account_sas_signature_values`
- Phase: `3.5`
- Status: `analyzed`

## Exported API
### Enum `SASProtocol`
- `HTTPS = "https"`
- `HTTPSandHTTP = "https,http"`

### Interface `IAccountSASSignatureValues`
- `version: string`
- `protocol?: SASProtocol | string`
- `startTime?: Date | string`
- `expiryTime: Date | string`
- `permissions: AccountSASPermissions | string`
- `ipRange?: SasIPRange | string`
- `services: AccountSASServices | string`
- `resourceTypes: AccountSASResourceTypes | string`
- `encryptionScope?: string`

### Function `generateAccountSASSignature(accountSASSignatureValues: IAccountSASSignatureValues, accountName: string, sharedKey: Buffer): [string, string]`
- Returns `[signature, stringToSign]`.
- Dispatches by `version` to one of two internal helpers.

## Internal helpers that shape the Rust port
- `generateAccountSASSignature20201206(accountSASSignatureValues: IAccountSASSignatureValues, accountName: string, sharedKey: Buffer): [string, string]`
- `generateAccountSASSignature20150405(accountSASSignatureValues: IAccountSASSignatureValues, accountName: string, sharedKey: Buffer): [string, string]`

## Dependencies
- Imports:
  - `@azure/storage-blob.SasIPRange` — external SDK type; no local porting-db record. Structurally compatible with local `IIPRange` today, but the account-SAS interface intentionally names the external type.
  - `../utils/utils.computeHMACSHA256` — Phase `4.2`, not yet analyzed/ported.
  - `../utils/utils.truncatedISO8061Date` — Phase `4.2`, not yet analyzed/ported.
  - `./AccountSASPermissions` — Phase `3.2`, analyzed in this pass.
  - `./AccountSASResourceTypes` — Phase `3.4`, analyzed in this pass.
  - `./AccountSASServices` — Phase `3.3`, analyzed in this pass.
  - `./IIPRange.ipRangeToString` — Phase `3.1`, analyzed in this pass.
- Key consumers:
  - `src/blob/authentication/AccountSASAuthenticator.ts` (later phase) calls `generateAccountSASSignature()` for blob requests.
  - `src/queue/authentication/AccountSASAuthenticator.ts` (later phase) calls `generateAccountSASSignature()` for queue requests.
  - `src/table/authentication/AccountSASAuthenticator.ts` (later phase) calls `generateAccountSASSignature()` for table requests.
  - `src/blob/authentication/IBlobSASSignatureValues.ts`, `src/queue/authentication/IQueueSASSignatureValues.ts`, and `src/table/authentication/ITableSASSignatureValues.ts` (later phases) import `SASProtocol`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| string enum `SASProtocol` | `enum SASProtocol` with `as_str() -> &'static str` | Preserve exact wire values `https` and `https,http`. |
| `interface IAccountSASSignatureValues` | `pub struct IAccountSASSignatureValues` | Keep field names/shape close to TS for propagation. |
| `Date | string` | enum like `DateOrString` | TS accepts either preformatted strings or `Date` objects. |
| `AccountSASPermissions | string` | enum like `PermissionsOrString` | Rust must preserve the ability to bypass helper objects and pass raw canonical strings. |
| `AccountSASServices | string` | enum like `ServicesOrString` | Same union semantics as TS. |
| `AccountSASResourceTypes | string` | enum like `ResourceTypesOrString` | Same union semantics as TS. |
| `SasIPRange | string` | enum like `IpRangeOrString` | Preserve the external-type-vs-local-serializer asymmetry explicitly. |
| `Buffer` parameter | `&[u8]` | Borrowed HMAC key input is sufficient. |
| `[string, string]` | `(String, String)` | Signature and string-to-sign tuple. |
| optional `string` fields | `Option<String>` | `undefined` serializes to an empty field in the string-to-sign. |

## Function and method mappings
- `generateAccountSASSignature(accountSASSignatureValues: IAccountSASSignatureValues, accountName: string, sharedKey: Buffer): [string, string]`
  - Rust: `pub fn generate_account_sas_signature(values: &IAccountSASSignatureValues, account_name: &str, shared_key: &[u8]) -> (String, String)`
  - Behavior: lexicographically dispatch on `values.version >= "2020-12-06"`.
- `generateAccountSASSignature20201206(...)`
  - Rust: private helper `fn generate_account_sas_signature_20201206(...) -> (String, String)`
  - Behavior: include `encryptionScope` as its own field before the final blank line.
- `generateAccountSASSignature20150405(...)`
  - Rust: private helper `fn generate_account_sas_signature_20150405(...) -> (String, String)`
  - Behavior: omit the `encryptionScope` field entirely.

## Recommended Rust translation
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SASProtocol {
    HTTPS,
    HTTPSandHTTP,
}

impl SASProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HTTPS => "https",
            Self::HTTPSandHTTP => "https,http",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DateOrString {
    Date(chrono::DateTime<chrono::Utc>),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionsOrString {
    Permissions(AccountSASPermissions),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IAccountSASSignatureValues {
    pub version: String,
    pub protocol: Option<String>,
    pub start_time: Option<DateOrString>,
    pub expiry_time: DateOrString,
    pub permissions: PermissionsOrString,
    pub ip_range: Option<IpRangeOrString>,
    pub services: ServicesOrString,
    pub resource_types: ResourceTypesOrString,
    pub encryption_scope: Option<String>,
}
```

## Special handling
- Version dispatch is a plain string comparison: `version >= "2020-12-06"`. That works because TS uses zero-padded `YYYY-MM-DD` version strings. Rust should preserve the same cutoff behavior rather than introducing a different notion of semantic version ordering.
- The helper trusts `.toString()` on `permissions`, `services`, and `resourceTypes`. Raw strings pass through unchanged because JS strings also implement `toString()`. Rust should preserve this dual-input behavior explicitly.
- `ipRange` is typed as external `SasIPRange | string`, but non-string values are serialized with local `ipRangeToString()`. Preserve this asymmetry with an explicit compatibility layer instead of silently collapsing everything to one local type.
- `startTime` and `expiryTime` use `truncatedISO8061Date(..., false)`, so the generated string omits fractional seconds. Do not accidentally emit milliseconds in Rust.
- The final `""` entry in the `stringToSign` array is required: it forces a trailing newline in the joined string. Preserve that exact byte layout.
- In the `2020-12-06` branch, `encryptionScope` is inserted as its own field. When it is `undefined`, JS `Array.join()` emits an empty field; Rust should use `unwrap_or_default()` or equivalent.
- The function performs no validation beyond formatting. If callers pass raw strings with non-canonical ordering, those strings are signed as-is.
- `computeHMACSHA256()` returns a base64-encoded HMAC-SHA256 digest of the UTF-8 string-to-sign. That output format is part of the wire contract.

## Change propagation notes
- Any change to `AccountSASPermissions`, `AccountSASServices`, `AccountSASResourceTypes`, `ipRangeToString()`, or `truncatedISO8061Date()` changes signature bytes and must trigger parity checks in all blob/queue/table account-SAS authenticators.
- If TS adds a new account SAS service version, prefer adding a new version-specific helper and updating the dispatcher threshold rather than mutating the existing 2015 or 2020 layouts in place.
- If TS changes the account-SAS `ipRange` type from external `SasIPRange` to local `IIPRange` (or vice versa), update the Rust compatibility layer and the later service-SAS records together.
- If `encryptionScope` rules evolve, confirm whether the change belongs only to newer versions or whether historical string-to-sign layouts also change.
