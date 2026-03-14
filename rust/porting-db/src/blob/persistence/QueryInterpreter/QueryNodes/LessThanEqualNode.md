# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/LessThanEqualNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/LessThanEqualNode.ts`
- Source lines: `26`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/less_than_equal_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::less_than_equal_node`
- Phase: `10.4`
- Status: `ported`

## Exported API
### Default class `LessThanEqualNode`
- Extends `BinaryOperatorNode`.
- Member: `name -> "lte"`
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
- `LessThanEqualNode.ts` evaluates both child nodes and only succeeds when both witness values are defined and the left value is lexicographically less than or equal to the right.
- On success it returns whichever side carried a `key`, so identifier-vs-constant comparisons preserve the identifier witness for later tag filtering.
- `LessThanEqualNode.ts:9-25` mirrors `LessThanNode` with the same loose `any` return annotation.

## Change propagation notes
- If TS changes witness selection (left vs right key), update every comparison node and `toBlobTags()` integration together.
- Do not replace these operators with boolean returns unless the upstream TS contracts do so too.
