# Porting Record — `(Rust-specific — no direct TS equivalent)`

## File info
- Source path: `(Rust-specific — no direct TS equivalent)`
- Source lines: `~50`
- Source type: `handwritten (Rust addition)`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_value.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_value`
- Status: `ported`

## Special handling
Rust-specific typed value enum for query evaluation. TS uses dynamic `any` typing; Rust needs explicit value variants (String, Int32, Int64, Double, Bool, DateTime, Guid, Binary, Null).
