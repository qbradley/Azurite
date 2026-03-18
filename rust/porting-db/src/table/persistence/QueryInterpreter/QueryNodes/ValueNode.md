# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/ValueNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/ValueNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/value_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::value_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Base value node for typed literals.
