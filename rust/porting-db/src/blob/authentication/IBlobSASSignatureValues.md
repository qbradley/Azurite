# Porting Record — `src/blob/authentication/IBlobSASSignatureValues.ts`

## File info
- Source path: `src/blob/authentication/IBlobSASSignatureValues.ts`
- Source lines: `818`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/i_blob_sas_signature_values.rs`
- Crate: `azurite-blob`
- Module: `authentication::i_blob_sas_signature_values`
- Phase: `7.3`
- Status: `ported`

## Exported API
### Interface `IBlobSASSignatureValues`
- Core SAS fields:
  - `version: string`
  - `protocol?: SASProtocol | string`
  - `startTime?: Date | string`
  - `expiryTime?: Date | string`
  - `permissions?: string`
  - `ipRange?: IIPRange | string`
  - `containerName: string`
  - `blobName?: string`
  - `identifier?: string`
- Response-header overrides:
  - `cacheControl?`, `contentDisposition?`, `contentEncoding?`, `contentLanguage?`, `contentType?`
- Resource / snapshot / encryption:
  - `signedResource?`, `snapshot?`, `encryptionScope?`
- User-delegation-key fields:
  - `signedObjectId?`, `signedTenantId?`, `signedService?`, `signedVersion?`, `signedStartsOn?`, `signedExpiresOn?`
- Future delegated-user fields:
  - `delegatedUserObjectId?`
  - `delegatedUserTenantId?`

### Exported functions
- `generateBlobSASSignature(values, resource, accountName, sharedKey): [string, string]`
- `generateBlobSASSignatureWithUDK(values, resource, accountName, udkValue): [string, string]`

### Internal helpers that shape the Rust port
- `generateBlobSASSignature20201206()`
- `generateBlobSASSignature20181109()`
- `generateBlobSASSignature20150405()`
- `generateBlobSASSignatureUDK20181109()`
- `generateBlobSASSignatureWithUDK20200210()`
- `generateBlobSASBlobSASSignatureWithUDK20201206()`
- `generateBlobSASBlobSASSignatureWithUDK20250705()`
- `getCanonicalName(accountName, containerName, blobName?)`

## Dependencies
- Imports:
  - `../../common/utils/utils.computeHMACSHA256` — Phase `4.2` shared HMAC helper.
  - `../../common/utils/utils.truncatedISO8061Date` — Phase `4.2` timestamp formatter.
  - `./BlobSASResourceType` — Phase `7.5` blob SAS resource enum.
  - `../../common/authentication/IAccountSASSignatureValues.SASProtocol` — Phase `3.5` shared protocol enum.
  - `../../common/authentication/IIPRange` / `ipRangeToString` — Phase `3.1` shared IP range helper.
- Key consumers:
  - `BlobSASAuthenticator.ts` calls both exported generators.
  - Later SAS emission code depends on these exact string-to-sign layouts.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `interface IBlobSASSignatureValues` | `pub struct IBlobSasSignatureValues` | Keep field names close to TS for future propagation. |
| `Date | string` | explicit enum like `DateOrString` | The generator accepts preformatted strings and native dates interchangeably. |
| `IIPRange | string` | explicit enum like `IpRangeOrString` | Preserve passthrough-string behavior. |
| `[string, string]` | `(String, String)` | First element is signature, second is string-to-sign. |
| version dispatch by string comparison | plain string comparison on `YYYY-MM-DD` literals | Do not replace with semantic-version parsing. |

## Version dispatch and canonical layouts
### `generateBlobSASSignature()`
- `version >= "2020-12-06"` → 2020-12-06 layout.
- else if `version >= "2018-11-09"` → 2018-11-09 layout.
- else → 2015-04-05 layout.

### `generateBlobSASSignatureWithUDK()`
- `version >= "2025-07-05"` → 2025-07-05 UDK layout.
- else if `version >= "2020-12-06"` → 2020-12-06 UDK layout.
- else if `version >= "2020-02-10"` → 2020-02-10 UDK layout.
- else if `version >= "2018-11-09"` → 2018-11-09 UDK layout.
- else throws `RangeError("SAS token version is not valid")`.

### Canonical-field differences that matter
- `2020-12-06` service SAS includes `signedResource`, `snapshot`, and `encryptionScope`.
- `2018-11-09` service SAS includes `signedResource` and `snapshot` but **not** `encryptionScope`.
- `2015-04-05` service SAS includes neither `signedResource` nor `snapshot`, and only appends blob name when `resource === Blob`.
- `2018-11-09` UDK layout adds signed object/tenant/service/version/timing fields plus an explicit `undefined` blob-version-timestamp slot.
- `2020-02-10` UDK layout inserts three `undefined` placeholder slots for preauthorized agent object ID, agent object ID, and correlation ID.
- `2020-12-06` UDK layout keeps those placeholders and adds `encryptionScope`.
- `2025-07-05` UDK layout keeps the placeholders and inserts `delegatedUserTenantId` before `delegatedUserObjectId`.

## Special handling
- Every generator signs the **un-url-encoded** values.
- `getCanonicalName()` concatenates raw path fragments as `/blob/{account}/{container}` plus `/{blob}` if `blobName` is truthy. It does not URL-encode or decode.
- All service-SAS generators use the condition `if (!identifier && (!permissions && !expiryTime))`. That means they only throw when **both** `permissions` and `expiryTime` are missing; if one is missing and the other is present, generation still proceeds. Preserve this exact quirk.
- By contrast, `BlobSASAuthenticator.getBlobSASSignatureValuesFromRequest()` later requires both fields when `identifier` is absent. Document the mismatch; do not silently reconcile it in Rust.
- `identifier` is inserted into the string-to-sign directly. The inline TODO comment asks whether it should be blanked conditionally, but current behavior is passthrough.
- Optional fields are frequently emitted as empty strings, not omitted from the array. Those blank fields are part of the signed byte layout.
- The helper names `generateBlobSASBlobSASSignatureWithUDK20201206` and `generateBlobSASBlobSASSignatureWithUDK20250705` contain duplicated `BlobSAS` in the function name. Keep that oddity visible in the record.
- The UDK `2020-12-06` and `2025-07-05` helpers only append `blobName` to the canonical name when `resource === Blob`, **not** when `resource === BlobSnapshot`. Earlier non-UDK helpers treat `BlobSnapshot` like `Blob`. Preserve that version/resource asymmetry.
- The UDK layouts intentionally include literal `undefined` placeholders when `.join("\n")` builds the string-to-sign. Rust must emit empty fields in the same positions.

## Change propagation notes
- Any new SAS version should become a new dedicated helper plus a dispatcher update; do not mutate historical layouts in place.
- If TS adds or renames a field in `IBlobSASSignatureValues`, audit every version-specific helper because field order is the actual compatibility surface here.
- If common helpers like `truncatedISO8061Date()` or `ipRangeToString()` change, re-run parity checks for both blob SAS and account SAS because they share formatting primitives.
