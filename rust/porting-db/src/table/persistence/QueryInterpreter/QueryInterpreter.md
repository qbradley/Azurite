# Porting Record — `src/table/persistence/QueryInterpreter/QueryInterpreter.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryInterpreter.ts`
- Source lines: `16`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_interpreter.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_interpreter`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `trait IQueryNode` | AST node interface |
| `IQueryContext` | `trait IQueryContext` | Entity property accessor |

## Special handling
Entry point: parses OData $filter string and evaluates against entities. Delegates to QueryParser for AST construction and QueryNode tree for evaluation.
