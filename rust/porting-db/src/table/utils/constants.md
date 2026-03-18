# Porting Record — `src/table/utils/constants.ts`

## File info
- Source path: `src/table/utils/constants.ts`
- Source lines: `147`
- Source type: `handwritten`
- Rust target: `azurite-table/src/utils/constants.rs`
- Crate: `azurite-table`
- Module: `utils::constants`
- Status: `ported`

## Special handling
Table service constants: API version strings, default ports (11002), header names, entity size limits (1MB), property count limits (255), batch size limits. Follows same pattern as blob equivalent. See `porting-db/src/blob/utils/constants.md` for detailed fidelity notes.
