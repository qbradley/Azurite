# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/OrNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/OrNode.ts`
- Source lines: `19`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/or_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::or_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `OrNode`
- Extends `BinaryOperatorNode`.
- Member: `name -> "or"`
- Method: `evaluate(context)`

## Dependencies
- `../IQueryContext`.
- `./BinaryOperatorNode`.
- `./IQueryNode.TagContent`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| logical disjunction node | binary AST variant | Uses witness-array emptiness as truthiness rather than booleans. |
| `left.concat(right)` | concatenated witness vector | Even when only one side matched, TS still concatenates both arrays and returns the non-empty side unchanged. |

## Special handling
- `OrNode.ts:10-18` returns `left.concat(right)` when either side is non-empty.
- There is no deduplication or special casing for two matching branches; both witness arrays are preserved in order.

## Change propagation notes
- If TS introduces short-circuit evaluation, revisit both semantic behavior and witness ordering.
- Any later change to how OR combines witness arrays should also be reflected in tag-result materialization.
