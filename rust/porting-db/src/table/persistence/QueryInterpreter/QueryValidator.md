# Porting Record — `src/table/persistence/QueryInterpreter/QueryValidator.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryValidator.ts`
- Source lines: `36`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_validator.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_validator`
- Status: `ported`

## Special handling
Validates parsed query AST for semantic correctness before evaluation.
