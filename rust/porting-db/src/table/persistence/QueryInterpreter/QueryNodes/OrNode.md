# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/OrNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/OrNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/or_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::or_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Logical OR — evaluates both children, returns true if either true.
