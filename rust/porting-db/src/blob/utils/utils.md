# Porting Record — `src/blob/utils/utils.ts`

## File info
- Source path: `src/blob/utils/utils.ts`
- Source lines: `259`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/utils/utils.rs`
- Crate: `azurite-blob`
- Module: `utils::utils`
- Phase: `12.2`
- Status: `not_started`

## Exported API
### Utility functions
- `checkApiVersion()`
- `streamToLocalFile()`
- `deserializeRangeHeader()`
- `deserializePageBlobRangeHeader()`
- `removeQuotationFromListBlobEtag()`
- `validateContainerName()`
- `getUserDelegationKeyValue()`
- `getBlobTagsCount()`
- `getTagsFromString()`
- `validateBlobTag()`
- `toBlobTags()`

## Dependencies
- `crypto.createHmac`, `fs.createWriteStream`
- `StorageErrorFactory`
- `USERDELEGATIONKEY_BASIC_KEY`
- `BlobTag`, `BlobTags` from `@azure/storage-blob`
- `TagContent` from the Phase 10 query interpreter

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| header/range parsing helpers | plain functions returning tuples/results | Preserve current `Infinity` / `undefined` conventions explicitly. |
| blob tag helpers using SDK models | local Rust structs mirroring Azure tag payloads | Tag parsing and query conversion are compatibility-sensitive. |
| HMAC helper | deterministic SHA-256 helper | Used by both auth and service handler UDK flows. |

## Special handling
- `utils.ts:8-16` performs API-version validation by exact membership lookup, not prefix or date parsing.
- `utils.ts:18-31` resolves `streamToLocalFile()` on the write stream's `close` event, not on input-stream `end`.
- `utils.ts:41-78` parses standard byte ranges, returning `undefined` when no range header exists and `Infinity` for open-ended ranges.
- `utils.ts:91-117` adds optional 512-byte boundary validation for page blobs on top of standard range parsing.
- `utils.ts:125-133` strips surrounding quotes from listed ETags only when both first and last characters are `"`.
- `utils.ts:135-148` enforces container naming with a single regex and length bounds; system containers are exempted earlier in middleware, not here.
- `utils.ts:151-168` derives user delegation keys from a fixed HMAC seed and the literal string-to-sign `[oid, tid, start, expiry, "b", version]`.
- `utils.ts:176-199` parses header-encoded blob tags, first replacing `+` with `%20` before `decodeURIComponent()`. It also uses `split("=")`, so additional `=` characters in tag values are not preserved.
- `utils.ts:203-223` enforces the current tag limits: max 10 tags, key length 1..128, value length <= 256, and a whitelist of ASCII-ish characters.
- `utils.ts:245-258` converts query-interpreter witness arrays back to tags by dropping the `@container` sentinel and letting later duplicate keys overwrite earlier ones via a `Record<string,string>`.

## Change propagation notes
- Range parsing changes here affect `BlobHandler`, `PageBlobHandler`, and batch parsing behavior indirectly; update those call sites as a unit.
- Tag-query behavior spans this file, Phase 10 query nodes, and handler/tag APIs. Keep them synchronized.
