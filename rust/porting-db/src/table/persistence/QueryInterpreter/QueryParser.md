# Porting Record — `src/table/persistence/QueryInterpreter/QueryParser.ts`

## File info
- Source path: `src/table/persistence/QueryInterpreter/QueryParser.ts`
- Source lines: `266`
- Source type: `handwritten`
- Rust target: `azurite-table/src/persistence/query_interpreter/query_parser.rs`
- Crate: `azurite-table`
- Module: `persistence::query_interpreter::query_parser`
- Status: `ported`

## Type mappings
| TypeScript | Rust | Notes |
|---|---|---|
| `IQueryNode` | `trait IQueryNode` | AST node interface |
| `IQueryContext` | `trait IQueryContext` | Entity property accessor |

## Special handling
Recursive-descent parser building AST from lexer tokens. Produces IQueryNode tree.

Key fidelity concerns:
1. Operator precedence: `not` > `and` > `or` (standard OData)
2. Comparison operators: eq, ne, gt, ge, lt, le
3. Typed literals: datetime'...', guid'...', binary'...', X'...'
4. Documents unary `not` in grammar but TS implementation may not fully consume it
5. Parenthesized sub-expressions supported
