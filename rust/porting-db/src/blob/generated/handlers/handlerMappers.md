# Porting Record — `src/blob/generated/handlers/handlerMappers.ts`

## File info
- Source path: `src/blob/generated/handlers/handlerMappers.ts`
- Source lines: `563`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/handlers/handler_mappers.rs`
- Crate: `azurite-blob`
- Module: `generated::handlers::handler_mappers`
- Phase: `5.16`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export interface IHandlerPath {
  handler: string;
  method: string;
  arguments: string[];
}

export default getHandlerByOperation;
```

## Dependencies
- Internal imports:
  - `../artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `{ [key: number]: IHandlerPath }` | `static map keyed by Operation repr value` | Handler dispatch depends on enum numeric identity. |
| `handler: string` / `method: string` | enum/const identifiers or `&'static str` | Dynamic string lookup is the current generated contract. |
| `arguments: string[]` | `&'static [&'static str]` | Argument order drives how middleware builds the handler call. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Contains one `operationHandlerMapping` entry per generated operation plus the exported `getHandlerByOperation()` lookup helper.
- Several GET/HEAD pairs intentionally alias to the same handler method (for example both `Service_GetAccountInfo` and `Service_GetAccountInfoWithHead` map to `serviceHandler.getAccountInfo`). Preserve those aliases instead of inventing separate methods.
- The `arguments` arrays must stay in lockstep with deserializer `parameterPath` names and handler interface signatures.

## Middleware chain ordering
- Stage 3 handler middleware calls `getHandlerByOperation(context.operation)` to determine which handler object, method name, and positional parameter list to invoke.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- Any regenerated change to this map requires synchronized updates to handler interface method names and to the middleware’s dynamic invocation path.
