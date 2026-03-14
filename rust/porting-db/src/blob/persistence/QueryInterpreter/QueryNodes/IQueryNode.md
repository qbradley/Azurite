# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/IQueryNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/IQueryNode.ts`
- Source lines: `13`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/i_query_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::i_query_node`
- Phase: `10.3`
- Status: `ported`

## Exported API
### Interface `TagContent`
- `key?: string`
- `value?: string`

### Default interface `IQueryNode`
- `get name(): string`
- `evaluate(context: IQueryContext): TagContent[]`
- `toString(): string`

## Dependencies
- `../IQueryContext`.
- Implemented by every node under `QueryNodes/` and consumed by `QueryParser` / `QueryInterpreter`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `TagContent[]` | `Vec<TagContent>` | Nodes return witness values, not booleans; non-empty means the predicate matched. |
| `key?: string`, `value?: string` | `Option<String>` fields | Comparison nodes decide which side “won” by checking which witness carries a key. |
| `IQueryNode` | trait object or enum AST node | Every node needs `evaluate()` plus a stable `to_string()` form for debugging/parity. |

## Special handling
- `IQueryNode.ts:3-6` makes both `key` and `value` optional, which is why constant nodes can return value-only witnesses and comparison nodes can return whichever side contained an identifier.
- `IQueryNode.ts:8-13` defines evaluation in terms of arrays rather than booleans; `AndNode` / `OrNode` later use `.length` checks as truthiness.

## Change propagation notes
- If TS changes `TagContent` shape, revisit every comparison node and `toBlobTags()` conversion in the persistence layer.
- Keep `toString()` behavior stable if query strings or diagnostics are compared in tests later.
