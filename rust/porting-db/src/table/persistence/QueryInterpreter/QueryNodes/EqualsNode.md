# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/EqualsNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/EqualsNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/equals_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::equals_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Comparison: `eq` operator. Type-aware comparison (string, number, datetime, guid, binary).
