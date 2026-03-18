# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/GreaterThanNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/GreaterThanNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/greater_than_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::greater_than_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Comparison: `gt` operator.
