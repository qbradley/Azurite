# Porting Record — `src/table/persistence/QueryInterpreter/QueryNodes/IQueryNode.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryNodes/IQueryNode.ts`
- Source lines: `~10`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/i_query_node.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::i_query_node`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `interface IQueryNode` | `trait IQueryNode` | AST node trait |

## Special handling
Core AST node trait: `evaluate(context: IQueryContext) -> bool`. All query nodes implement this.
