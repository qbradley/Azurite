# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/BinaryOperatorNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/BinaryOperatorNode.ts`
- Source lines: `13`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/binary_operator_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::binary_operator_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default abstract class `BinaryOperatorNode`
- Constructor: `new BinaryOperatorNode(left: IQueryNode, right: IQueryNode)`
- Abstract members:
  - `evaluate(context: IQueryContext): any`
  - `get name(): string`
- Concrete helper:
  - `toString(): string`

## Dependencies
- `../IQueryContext`.
- `./IQueryNode`.
- Base class for `AndNode`, `OrNode`, and every comparison operator node.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| abstract binary AST node | enum variant or boxed trait object with `left/right` children | Keep the two-child structure visible for recursive evaluation. |
| `evaluate(...): any` | `Vec<TagContent>` return in practice | TS leaves the base method loose even though concrete nodes all return witness arrays. |

## Special handling
- `BinaryOperatorNode.ts:4-5` stores `left` and `right` as public fields, and `QueryInterpreter.countIdentifierReferences()` later recurses through them directly.
- `BinaryOperatorNode.ts:11-12` parenthesizes `toString()` as `(<left> <name> <right>)`, which is useful parity output when debugging parser trees.

## Change propagation notes
- If TS adds unary or n-ary operator base classes later, do not overload this one to fake the new shape.
- Any change to child visibility affects the query interpreter recursion helper immediately.
