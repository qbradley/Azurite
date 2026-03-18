# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/ConstantNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/ConstantNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/constant_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::constant_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Literal value node (string, number, boolean).
