# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/NotNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/NotNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/not_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::not_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Logical NOT — inverts child result.
