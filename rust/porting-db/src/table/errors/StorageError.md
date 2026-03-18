# Porting Record — `src/table/errors/StorageError.ts`

## File info
- Source path: `src/table/errors/StorageError.ts`
- Source lines: `~60`
- Source type: `handwritten`
- Rust target: `azurite-table/src/errors/storage_error.rs`
- Crate: `azurite-table`
- Module: `errors::storage_error`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `StorageError class` | `StorageError struct` | Error with HTTP status + code + message |

## Special handling
Follows same pattern as blob equivalent. See `porting-db/src/blob/errors/StorageError.md` for detailed fidelity notes.
