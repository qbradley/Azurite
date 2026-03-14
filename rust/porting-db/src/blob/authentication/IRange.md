# Porting Record — `src/blob/authentication/IRange.ts`

## File info
- Source path: `src/blob/authentication/IRange.ts`
- Source lines: `48`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/authentication/i_range.rs`
- Crate: `azurite-blob`
- Module: `authentication::i_range`
- Phase: `7.7`
- Status: `ported`

## Exported API
### Interface `IRange`
- `offset: number`
- `count?: number`

### Function `rangeToString(iRange: IRange): string`
- Returns `bytes=<offset>-<end>` when `count` is provided.
- Returns `bytes=<offset>-` when `count` is absent.

## Dependencies
- Imports: none.
- Key consumers:
  - Used by later blob-range and SAS-related code that needs canonical `Range` header formatting.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| interface | `struct IRange` | Keep explicit `offset` and optional `count` rather than compressing into a Rust range type. |
| `RangeError` | dedicated Rust error type | Two validation failures currently exist: negative offset and non-positive count. |

## Special handling
- `offset < 0` throws `RangeError("IRange.offset cannot be smaller than 0.")`.
- The count validation is written as `if (iRange.count && iRange.count <= 0)`. Because `0` is falsy in JS, `count = 0` bypasses the error path entirely.
- The formatter also uses `iRange.count ? ... : ...`, so `count = 0` produces the open-ended form `bytes=<offset>-` instead of an error. Preserve this observable quirk unless the team explicitly decides to fix it.
- When `count` is positive, the end byte is inclusive: `offset + count - 1`.

## Change propagation notes
- If TS tightens validation around zero counts, record that as a real behavior change; today zero is treated like `undefined`.
- If later code starts depending on open-ended zero-count behavior, Rust should mirror it rather than switching to idiomatic range validation silently.
