# Porting Record — `src/queue/utils/constants.ts`

## File info
- Source path: `src/queue/utils/constants.ts`
- Source lines: `160`
- Source type: `handwritten`
- Rust target: `azurite-queue/src/utils/constants.rs`
- Crate: `azurite-queue`
- Module: `utils::constants`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `const` | `pub const` | String/numeric constants |

## Special handling
Queue service constants: API version strings, default ports, header names, limits (max message size 64KB, visibility timeout max 7 days, max messages per peek/dequeue 32). Follows same pattern as blob equivalent. See `porting-db/src/blob/utils/constants.md` for detailed fidelity notes.
