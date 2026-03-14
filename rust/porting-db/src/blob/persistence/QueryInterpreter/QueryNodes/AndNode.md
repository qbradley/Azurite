# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/AndNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/AndNode.ts`
- Source lines: `19`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/and_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::and_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `AndNode`
- Extends `BinaryOperatorNode`.
- Member: `name -> "and"`
- Method: `evaluate(context)`

## Dependencies
- `../IQueryContext`.
- `./BinaryOperatorNode`.
- `./IQueryNode.TagContent`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| logical conjunction node | binary AST variant | Uses witness-array emptiness as truthiness rather than booleans. |
| `left.concat(right)` | `Vec::extend`/concatenate` | Preserve order and avoid deduping matched witnesses. |

## Special handling
- `AndNode.ts:10-18` evaluates both children and returns `left.concat(right)` only when both sides are non-empty.
- If either side is empty, the whole node returns `[]`; there is no short-circuiting optimization or deduplication.

## Change propagation notes
- If later TS adds witness deduping, update both `AndNode` and `OrNode` together.
- Keep concatenation order stable because downstream tag materialization currently sees the left-side witnesses first.
