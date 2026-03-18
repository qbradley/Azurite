# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/BinaryDataNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/BinaryDataNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/binary_data_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::binary_data_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
binary'...' or X'...' literal. Base64/hex encoded binary data for comparison.
