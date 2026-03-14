# Porting Record — `src/blob/handlers/BaseHandler.ts`

## File info
- Source path: `src/blob/handlers/BaseHandler.ts`
- Source lines: `20`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/handlers/base_handler.rs`
- Crate: `azurite-blob`
- Module: `handlers::base_handler`
- Phase: `11.1`
- Status: `not_started`

## Exported API
### Default class `BaseHandler`
- Constructor: `(metadataStore: IBlobMetadataStore, extentStore: IExtentStore, logger: ILogger, loose: boolean)`
- Protected fields shared by all blob handlers: `metadataStore`, `extentStore`, `logger`, `loose`.

## Dependencies
- `../persistence/IBlobMetadataStore`
- `../../common/persistence/IExtentStore`
- `../generated/utils/ILogger`
- Extended by every handwritten Phase 11 handler.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| abstract-ish base class with protected readonly fields | embedded struct held by concrete handlers | Matches D-016 composition-over-inheritance without hiding the shared dependencies. |
| `loose: boolean` flag | plain `bool` field | This flag changes observable request validation behavior; keep it on every handler. |

## Special handling
- `BaseHandler.ts:13-19` is intentionally tiny. The important behavior is structural: every concrete handler receives the same store/logger/loose bundle through inheritance.
- This file does not define helper methods, so Rust should not invent generic behavior here that could obscure future TS diffs.

## Change propagation notes
- If future TS adds shared helper methods here, Aragorn should port them into the embedded base struct rather than scattering copies through concrete handlers.
- Keep constructor parameter order aligned with TypeScript so later mechanical change propagation stays straightforward.
