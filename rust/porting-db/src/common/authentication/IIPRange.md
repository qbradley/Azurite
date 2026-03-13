# Porting Record — `src/common/authentication/IIPRange.ts`

## File info
- Source path: `src/common/authentication/IIPRange.ts`
- Source lines: `37`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-common/src/authentication/i_ip_range.rs`
- Crate: `azurite-common`
- Module: `authentication::i_ip_range`
- Phase: `3.1`
- Status: `ported`

## Exported API
### Interface `IIPRange`
- `start: string`
- `end?: string`

### Function `ipRangeToString(ipRange: IIPRange): string`
- Serializes the range as `start-end` when `end` is present, otherwise returns `start` unchanged.

## Dependencies
- Imports: none.
- Porting status:
  - No direct imports.
- Key consumers:
  - `src/common/authentication/IAccountSASSignatureValues.ts` (Phase 3.5; analyzed in this pass) imports `ipRangeToString()` but uses external `SasIPRange` for the interface type.
  - `src/blob/authentication/IBlobSASSignatureValues.ts`, `src/queue/authentication/IQueueSASSignatureValues.ts`, and `src/table/authentication/ITableSASSignatureValues.ts` (later phases) import both `IIPRange` and `ipRangeToString()`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `interface IIPRange` | `pub struct IIPRange { start: String, end: Option<String> }` | Preserve the TS data shape and optional end bound. |
| `string` | `String` / `&str` | Owned in structs, borrowed in helpers. |
| `end?: string` | `Option<String>` | Missing `end` means a single allowed IP. |
| `ipRangeToString(...)` return | `String` | Exact formatting helper; no validation layer in TS. |

## Function and method mappings
- `ipRangeToString(ipRange: IIPRange): string` → `pub fn ip_range_to_string(ip_range: &IIPRange) -> String`
  - Preserve exact TS behavior: no IP validation, no whitespace trimming, no normalization.

## Recommended Rust translation
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IIPRange {
    pub start: String,
    pub end: Option<String>,
}

pub fn ip_range_to_string(ip_range: &IIPRange) -> String {
    match &ip_range.end {
        Some(end) => format!("{}-{}", ip_range.start, end),
        None => ip_range.start.clone(),
    }
}
```

## Special handling
- TS performs pure string formatting only. Invalid IPv4/IPv6 text is not rejected here, so Rust should not silently add validation inside this helper.
- Account SAS generation passes external `SasIPRange` values into this serializer via structural compatibility. Rust should keep an explicit adapter or compatible struct conversion rather than assuming this helper is used only with local `IIPRange`.
- Preserve the exact `start-end` formatting with no added spaces and no special-case collapsing.

## Change propagation notes
- If TS adds validation, IPv6 normalization, or extra range fields, audit every SAS signature generator that reuses `ipRangeToString()`.
- If local `IIPRange` and external `SasIPRange` diverge in TS, revisit the Rust adapter layer immediately.
- If later phases start depending on ordering or canonicalization beyond simple concatenation, treat that as a behavioral change rather than a cosmetic refactor.

## Rust port notes
- Ported in `rust/crates/azurite-common/src/authentication/i_ip_range.rs` with the TS-facing `ipRangeToString()` export plus a snake_case alias.
- Derived `Serialize`/`Deserialize` on `IIPRange` and kept the formatter as pure string concatenation with no validation.
- Reused by the Phase 3 account-SAS signature module through an explicit `SasIPRange` → `IIPRange` adapter.
