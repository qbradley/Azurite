# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/ExpressionNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/ExpressionNode.ts`
- Source lines: `17`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/expression_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::expression_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `ExpressionNode`
- Implements `IQueryNode`.
- Constructor: `new ExpressionNode(child: IQueryNode)`
- Members: `name`, `evaluate(context)`, `toString()`

## Dependencies
- `../IQueryContext`.
- `./IQueryNode`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| grouping wrapper | AST node with one boxed child | Represents parenthesized expressions from the parser. |
| delegating `evaluate()` | simple child-forwarding method | No additional semantics beyond grouping. |

## Special handling
- `ExpressionNode.ts:11-13` simply forwards evaluation to `child`.
- `ExpressionNode.ts:15-16` wraps the child `toString()` result in an extra pair of parentheses, which `countIdentifierReferences()` also treats as a recursive node boundary.

## Change propagation notes
- If TS adds real unary operators later, do not overload `ExpressionNode`; introduce a new node type.
- Keep the grouping-only behavior aligned with `QueryParser.visitExpressionGroup()`.
