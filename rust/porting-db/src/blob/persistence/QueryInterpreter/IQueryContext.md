# Porting Record — `src/blob/persistence/QueryInterpreter/IQueryContext.ts`

## File info
- Source path: `src/blob/persistence/QueryInterpreter/IQueryContext.ts`
- Source lines: `0`
- Source type: `handwritten`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/persistence/query_interpreter/i_query_context.rs`
- Crate: `azurite-blob`
- Module: `persistence::query_interpreter::i_query_context`
- Phase: `10.2`
- Status: `not_started`

## Exported API
### Type alias `IQueryContext = any`

## Dependencies
- Consumed by every query node in `QueryInterpreter/QueryNodes/*`.
- Populated by `executeQuery()` in `QueryInterpreter.ts` with blob tags plus the synthetic `@container` identifier.

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `any` | string-keyed map or thin wrapper struct | Actual callers treat the context as `tagName -> value` plus `@container`. |
| dynamic property lookup | `HashMap<String, String>` access | Keep runtime key lookup visible rather than over-constraining the type. |

## Special handling
- The file is literally a one-line `type any` alias with no trailing newline, so `wc -l` reports `0` even though the source has content.
- This intentionally untyped boundary is what lets `KeyNode` read arbitrary tag names and `@container` from the same context object.

## Change propagation notes
- If TS ever gives `IQueryContext` a real shape, update every query node and `executeQuery()` together.
- Avoid hiding the dynamic-map behavior behind a rigid Rust struct unless the upstream TS source does so first.
