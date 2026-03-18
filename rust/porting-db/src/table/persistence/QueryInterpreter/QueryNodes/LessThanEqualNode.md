# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/LessThanEqualNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/LessThanEqualNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/less_than_equal_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::less_than_equal_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Comparison: `le` operator.
