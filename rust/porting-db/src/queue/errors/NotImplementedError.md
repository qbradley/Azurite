# Porting Record — `src/queue/errors/NotImplementedError.ts`

## File info
- Source path: `src/queue/errors/NotImplementedError.ts`
- Source lines: `19`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/errors/not_implemented_error.rs`
- Crate: `azurite-queue`
- Module: `errors::not_implemented_error`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `Error` | `custom error struct` | Extends JS Error |

## Special handling
Follows same pattern as blob equivalent. See `porting-db/src/blob/errors/NotImplementedError.md` for detailed fidelity notes.
