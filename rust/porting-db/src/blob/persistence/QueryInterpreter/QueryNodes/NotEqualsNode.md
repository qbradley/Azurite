# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/NotEqualsNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/NotEqualsNode.ts`
- Source lines: `27`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/not_equals_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::not_equals_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `NotEqualsNode`
- Extends `BinaryOperatorNode`.
- Member: `name -> "ne"`
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
- `NotEqualsNode.ts` evaluates both child nodes and only succeeds when both witness values are defined and not strictly equal.
- On success it returns whichever side carried a `key`, so identifier-vs-constant comparisons preserve the identifier witness for later tag filtering.
- `NotEqualsNode.ts:14-26` treats `undefined` on either side as a failed comparison rather than as a separate ordering case.

## Change propagation notes
- If TS changes witness selection (left vs right key), update every comparison node and `toBlobTags()` integration together.
- Do not replace these operators with boolean returns unless the upstream TS contracts do so too.
