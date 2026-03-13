# Porting Record — `src/common/utils/utils.ts`

## File info
- Source path: `src/common/utils/utils.ts`
- Source lines: `171`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/utils/utils.rs`
- Crate: `azurite-common`
- Module: `utils::utils`
- Phase: `4.2`
- Status: `analyzed`

## Exported API
### Constant `lfsa`
- `lfsa: any = require("lokijs/src/loki-fs-structured-adapter.js")`

### Constant `rimrafAsync`
- `rimrafAsync: (...args: any[]) => Promise<any> = promisify(rimraf)`

### Function `minDate`
- `minDate(date1: Date, date2: Date): Date`

### Function `convertDateTimeStringMsTo7Digital`
- `convertDateTimeStringMsTo7Digital(dateTimeString: string): string`

### Function `convertRawHeadersToMetadata`
- `convertRawHeadersToMetadata(rawHeaders: string[] = [], contextId: string = ""): { [propertyName: string]: string } | undefined`

### Function `newEtag`
- `newEtag(): string`

### Function `computeHMACSHA256`
- `computeHMACSHA256(stringToSign: string, key: Buffer): string`

### Function `truncatedISO8061Date`
- `truncatedISO8061Date(date: Date, withMilliseconds: boolean = true, hrtimePrecision: boolean = false): string`

### Function `getURLQueries`
- `getURLQueries(url: string): { [key: string]: string }`

### Function `getMD5FromString`
- `getMD5FromString(text: string): Promise<Uint8Array>`

### Function `getMD5FromStream`
- `getMD5FromStream(stream: NodeJS.ReadableStream): Promise<Uint8Array>`

## Dependencies
- Node built-ins: `crypto`, `url.parse`, `util.promisify`.
- External packages: `rimraf`, LokiJS structured adapter via dynamic `require()`.
- Internal imports:
  - `../../blob/errors/StorageErrorFactory` — not yet ported (`PORTING-ORDER` Phase `6.2`).
  - `./constants` — Phase `4.1`, analyzed in this pass.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Date` | `chrono::DateTime<Utc>` / `std::time::SystemTime` | Keep UTC ISO formatting exact. |
| `{ [key: string]: string }` | `HashMap<String, String>` / `BTreeMap<String, String>` | `HashMap` is behaviorally closest; no ordering guarantee in TS either. |
| `Buffer` | `&[u8]` / `Vec<u8>` | HMAC and MD5 inputs are byte-oriented. |
| `NodeJS.ReadableStream` | `impl AsyncRead` / boxed async reader | Needed for streaming MD5. |
| `Promise<Uint8Array>` | `async fn -> Vec<u8>` | TS resolves to raw digest bytes, not hex/base64 text. |
| `any` dynamic module export | adapter trait or placeholder module type | `lfsa` exists only because TS still talks to LokiJS. |

## Recommended Rust translation
- Keep most helpers as free functions in a shared `utils` module.
- Use `hmac` + `sha2` for HMAC-SHA256 and `md5`/`md-5` for MD5 digests.
- Keep the Loki adapter export explicit in the record even if the Rust port removes LokiJS, because future TS changes may still touch this file.

## Function / method mapping notes
| TS export | Rust recommendation | Fidelity notes |
|---|---|---|
| `lfsa` | omit runtime equivalent, document as `NotApplicable` helper or feature-gated placeholder | There is no LokiJS adapter in Rust; keep the coupling visible in the record. |
| `rimrafAsync` | `async fn rimraf_async(path: impl AsRef<Path>) -> io::Result<()>` | Recursive delete helper only; exact package API need not be reproduced, but call sites should remain obvious. |
| `minDate()` | `fn min_date(date1, date2) -> DateTime<Utc>` | Direct `min` semantics. |
| `convertDateTimeStringMsTo7Digital()` | `fn convert_datetime_string_ms_to_7_digit(input: &str) -> String` | Literal string rewrite: replace trailing `Z` with `0000Z`. |
| `convertRawHeadersToMetadata()` | `fn convert_raw_headers_to_metadata(raw_headers: &[String], context_id: &str) -> Result<Option<HashMap<String, String>>, StorageError>` | Preserve pairwise header walk, case-insensitive prefix check, and duplicate-value comma joining. |
| `newEtag()` | `fn new_etag(now: DateTime<Utc>, rng: &mut impl Rng) -> String` | Keep the random multiplier formula if fidelity matters more than stronger entropy. |
| `computeHMACSHA256()` | `fn compute_hmac_sha256(string_to_sign: &str, key: &[u8]) -> String` | Output is base64 text, not raw bytes. |
| `truncatedISO8061Date()` | `fn truncated_iso8061_date(date: DateTime<Utc>, with_milliseconds: bool, hrtime_precision: bool) -> String` | Preserve the 7-digit fractional-seconds formatting quirk and the alternate no-millis path. |
| `getURLQueries()` | `fn get_url_queries(url: &str) -> HashMap<String, String>` | Preserve current behavior: no URL-decoding and silently drop malformed pairs. |
| `getMD5FromString()` | `async fn get_md5_from_string(text: &str) -> Vec<u8>` | TS marks this async even though it is CPU-only. |
| `getMD5FromStream()` | `async fn get_md5_from_stream(reader: impl AsyncRead + Unpin) -> io::Result<Vec<u8>>` | Preserve streaming hash accumulation and error propagation. |

## Special handling
- `convertRawHeadersToMetadata()` walks `rawHeaders` two elements at a time and assumes the Node raw-header layout `[name, value, name, value, ...]`; do not normalize this into a pre-parsed map too early if future TS changes still operate on the raw array.
- Metadata keys are validated against `VALID_CSHARP_IDENTIFIER_REGEX`; invalid keys throw `StorageErrorFactory.getInvalidMetadata(contextId)`. Keep the failure path aligned with blob error translation rather than changing to a generic common error.
- `newEtag()` is intentionally quirky: it multiplies `Date.now()` by a random integer between roughly 70000 and 100000 so the resulting hex string reaches the expected Azure-style length. Preserve the format (`"0x..."`, uppercase hex) even if the Rust RNG implementation differs.
- `truncatedISO8061Date()` uses `process.hrtime()[1]` only to append four more digits, not to derive a true timestamp. This is an approximation, not real nanosecond time.
- `getURLQueries()` does not decode percent-encoding and only keeps pairs with exactly one `=` and a non-empty key.

## Change propagation notes
- If TS changes metadata validation or duplicate-header handling here, re-check blob metadata parsing and every test that expects comma-joined duplicate metadata values.
- If the project removes LokiJS entirely, this record is the place to note that `lfsa` became dead code in TS or that the Rust port intentionally leaves it unsupported.
- Any change to `truncatedISO8061Date()` can impact SAS signing and timestamp comparisons across blob/queue/table code, so propagate carefully.

## Fidelity risks and edge cases
- `convertDateTimeStringMsTo7Digital()` is a literal string rewrite and assumes the input ends with `Z`; do not silently broaden it unless TS does.
- `convertRawHeadersToMetadata()` preserves the first-seen key casing after `x-ms-meta-` and concatenates duplicate values with commas. Both behaviors can affect tests.
- `truncatedISO8061Date()` names ISO 8061, but it is really shaping ISO 8601 text; preserve the actual emitted strings rather than correcting the naming.
- `getMD5FromString()`/`getMD5FromStream()` return raw bytes. Converting to hex in Rust would be a fidelity break.
