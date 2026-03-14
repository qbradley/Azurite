# Porting Record — `src/blob/persistence/QueryInterpreter/QueryNodes/KeyNode.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/QueryNodes/KeyNode.ts`
- Source lines: `20`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/query_nodes/key_node.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::query_nodes::key_node`
- Phase: `10.4`
- Status: `not_started`

## Exported API
### Default class `KeyNode`
- Implements `IQueryNode`.
- Constructor: `new KeyNode(identifier: string)`
- Members: `name`, `evaluate(context)`, `toString()`

## Dependencies
- `../IQueryContext`.
- `./IQueryNode`.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| identifier node | leaf AST variant carrying a key name | Represents blob tag identifiers plus the special `@container` identifier. |
| dynamic `context[this.identifier]` lookup | map access by string key | Keep runtime lookup semantics; tags are not compile-time fields. |

## Special handling
- `KeyNode.ts:7-9` returns the literal node name `"id"`, not the identifier text itself.
- `KeyNode.ts:11-15` evaluates to `[{ key: identifier, value: context[identifier] }]`, which is how comparison nodes later know which side supplied the identifier.
- `KeyNode.ts:18-19` returns the raw identifier in `toString()` without quoting.

## Change propagation notes
- If TS later distinguishes tag identifiers from `@container` with separate node types, this generic key node should not absorb the change silently.
- Any change to the witness shape must be reflected in comparison nodes and `QueryInterpreter.executeQuery()`.
