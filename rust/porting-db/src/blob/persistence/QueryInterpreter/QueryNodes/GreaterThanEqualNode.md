# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/GreaterThanEqualNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/GreaterThanEqualNode.ts`
- Source lines: `26`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/greater_than_equal_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::greater_than_equal_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `GreaterThanEqualNode`
- Extends `BinaryOperatorNode`.
- Member: `name -> "gte"`
- Method: `evaluate(context)`

## Dependencies
- `../IQueryContext`.
- `./BinaryOperatorNode`.
- `./IQueryNode.TagContent` where explicitly imported by the TS file.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| binary comparison node | AST variant with `left` and `right` children | Evaluation depends on witness arrays from both child nodes. |
| JS string comparison | Rust string comparison preserving lexicographic semantics | TS compares string values directly; it does not coerce to numbers or dates. |

## Special handling
- `GreaterThanEqualNode.ts` evaluates both child nodes and only succeeds when both witness values are defined and the left value is lexicographically greater than or equal to the right.
- On success it returns whichever side carried a `key`, so identifier-vs-constant comparisons preserve the identifier witness for later tag filtering.
- `GreaterThanEqualNode.ts:9-25` types `evaluate()` as `any` instead of `TagContent[]`, a small but real TS looseness to keep visible.

## Change propagation notes
- If TS changes witness selection (left vs right key), update every comparison node and `toBlobTags()` integration together.
- Do not replace these operators with boolean returns unless the upstream TS contracts do so too.
