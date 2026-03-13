# Porting Record — `src/blob/errors/NotImplementedError.ts`

## File info
- Source path: `src/blob/errors/NotImplementedError.ts`
- Source lines: `37`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/errors/not_implemented_error.rs`
- Crate: `azurite-blob`
- Module: `errors::not_implemented_error`
- Phase: `6.3`
- Status: `not_started`

## Exported API
### Default class `NotImplementedError`
- Extends `StorageError`.
- Constructor: `new NotImplementedError(requestID: string = "")`
- Fixed payload:
  - status `501`
  - code `APINotImplemented`
  - message `Current API is not implemented yet. Please vote your wanted features to https://github.com/azure/azurite/issues`

### Named class `NotImplementedinSQLError`
- Extends `StorageError`.
- Constructor: `new NotImplementedinSQLError(requestID: string = "")`
- Same status/code as the default export, but the message explicitly scopes the limitation to SQL-database-backed metadata storage.

## Dependencies
- Imports:
  - `./StorageError` — Phase `6.1`.
- Key consumers:
  - Blob handlers throw `NotImplementedError` for unsupported REST APIs.
  - SQL-backed metadata paths use `NotImplementedinSQLError` for feature gaps specific to SQL mode.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| two tiny subclasses | two constructor helpers over `StorageError` | In Rust, dedicated helper fns keep the messages distinct without adding inheritance machinery. |
| default `requestID = ""` | `Option<&str>` or `&str` with empty default | Preserve the empty-string default. |

## Special handling
- The named SQL variant preserves the odd casing `NotImplementedinSQLError` (`in` is lowercase after `Implemented`). Keep that asymmetry visible in the record even if the Rust symbol gets snake_case.
- Both classes share the same storage error code `APINotImplemented`; only the message text distinguishes generic and SQL-mode gaps.

## Change propagation notes
- If TS later routes these cases through `StorageErrorFactory`, keep the exact messages and 501 status values.
- If SQL-mode support expands, audit the call sites first; this file itself is intentionally trivial.
