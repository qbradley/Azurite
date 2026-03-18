# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/NotEqualsNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/NotEqualsNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/not_equals_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::not_equals_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Comparison: `ne` operator.
