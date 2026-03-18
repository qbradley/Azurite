# Porting Record — `src/table/errors/StorageErrorFactory.ts`

## File info
- Source path: `src/table/errors/StorageErrorFactory.ts`
- Source lines: `445`
- Source type: `handwritten`
- Rust target: `azurite-table/src/errors/storage_error_factory.rs`
- Crate: `azurite-table`
- Module: `errors::storage_error_factory`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `static method` | `pub fn` | Factory methods |
| `additionalMessages?: object` | `Option<HashMap<String, String>>` | Optional error details |

## Special handling
Table-specific error factory with ~35 static methods. Includes table-unique errors:
- `getTableAlreadyExists()` (409), `getTableNotFound()` (404)
- `getEntityNotFound()` (404), `getEntityAlreadyExists()` (409)
- `getPropertiesNeedValue()` — entity property validation
- `getDuplicatePropertiesSpecified()` — duplicate property names in entity
- `getAtomicityFailure()` — batch transaction failure
- `getEntityTooLarge()` — entity exceeds 1MB limit
- `getTooManyProperties()` — entity exceeds 255 properties
- `getInvalidInput()` — OData query parse errors

Method naming is inconsistent (get-prefix vs no-prefix) — must preserve exactly. Follows same pattern as blob equivalent. See `porting-db/src/blob/errors/StorageErrorFactory.md` for detailed fidelity notes.

## Change propagation notes
Adding table-specific errors follows factory pattern. Error messages are part of public API.
