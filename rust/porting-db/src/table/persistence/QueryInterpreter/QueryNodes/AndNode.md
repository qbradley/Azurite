# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/AndNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/AndNode.ts`
- Source lines: `~30`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/and_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::and_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `impl IQueryNode` | AST node impl |

## Special handling
Logical AND — evaluates both children, returns true only if both true.
