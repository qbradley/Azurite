# Porting Record — `(Rust-specific — no direct TS equivalent)`

## File info
- Source path: `(Rust-specific — no direct TS equivalent)`
- Source lines: `~30`
- Source type: `handwritten (Rust addition)`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_error.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_error`
- Status: `ported`

## Special handling
Rust-specific error types for query parsing/evaluation failures. TS uses thrown exceptions; Rust uses typed Result errors.
