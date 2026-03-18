# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/GuidNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/GuidNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/guid_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::guid_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
guid'...' literal. Parses UUID format.
