# Porting Record — `src/table/persistence/QueryInterpreter/IQueryContext.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/IQueryContext.ts`
- Source lines: `~20`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/i_query_context.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::i_query_context`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `trait IQueryNode` | AST node interface |
| `IQueryContext` | `trait IQueryContext` | Entity property accessor |

## Special handling
Query evaluation context interface — provides entity property access for filter evaluation.
