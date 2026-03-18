# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/DateTimeNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/DateTimeNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/date_time_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::date_time_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
datetime'...' literal. Parses ISO 8601 format.
