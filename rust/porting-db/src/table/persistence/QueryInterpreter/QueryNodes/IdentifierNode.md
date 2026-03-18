# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/IdentifierNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/IdentifierNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/identifier_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::identifier_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Property name reference — resolves against IQueryContext to get entity property value.
