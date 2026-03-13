# Porting Record — `src/blob/generated/middleware/dispatch.middleware.ts`

## File info
- Source path: `src/blob/generated/middleware/dispatch.middleware.ts`
- Source lines: `189`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/dispatch.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::dispatch`
- Phase: `5.21`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default function dispatchMiddleware(
  context: Context,
  req: IRequest,
  next: NextFunction,
  logger: ILogger
): void {
  logger.verbose(
    `DispatchMiddleware: Dispatching request...`,
    context.contextId
  );

  // Sometimes, more than one operations specifications are all valid against current request
  // Such as a SetContainerMetadata request will fit both CreateContainer and SetContainerMetadata specifications
  // We need to avoid this kind of situation when define swagger
  // However, following code will try to find most suitable operation by selecting operation which
  // have most required conditions met
  let conditionsMet: number = -1;

  for (const key in Operation) {
    if (Operation.hasOwnProperty(key)) {
      const operation = parseInt(key, 10);
      const res = isRequestAgainstOperation(
        req,
        Specifications[operation],
        context.dispatchPattern
      );
      if (res[0] && res[1] > conditionsMet) {
        context.operation = operation;
        conditionsMet = res[1];
      }
    }
  }

  if (context.operation === undefined) {
    const handlerError = new UnsupportedRequestError();
    logger.error(
      `DispatchMiddleware: ${handlerError.message}`,
      context.contextId
    );
    return next(handlerError);
  }

  logger.info(
    `DispatchMiddleware: Operation=${Operation[context.operation]}`,
    context.contextId
  );

  next();
}
```

## Dependencies
- External packages:
  - `@azure/ms-rest-js` — imported as `* as msRest`
- Internal imports:
  - `../artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `../artifacts/specifications` → `src/blob/generated/artifacts/specifications.ts` — Phase 5 — analyzed in this pass
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../errors/UnsupportedRequestError` → `src/blob/generated/errors/UnsupportedRequestError.ts` — Phase 5 — analyzed in this pass
  - `../IRequest` → `src/blob/generated/IRequest.ts` — Phase 5 — analyzed in this pass
  - `../MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass
  - `../utils/utils` → `src/blob/generated/utils/utils.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `NextFunction` callback | next-step continuation / result propagation closure | Dispatch passes `UnsupportedRequestError` into `next(err)` on failure. |
| `[boolean, number]` scoring tuple | `(bool, usize)` | Second element counts matched required conditions for tie-breaking. |
| `msRest.OperationSpec` | `OperationSpec` descriptor | Uses HTTP method/path/query/header metadata from generated specs. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Iterates every `Operation` enum member, tests the request against `Specifications[operation]`, and chooses the match with the highest required-condition score. Do not short-circuit on first match.
- Honors `X-HTTP-Method` override for `GET`, `MERGE`, `PATCH`, and `DELETE` only.
- Uses `context.dispatchPattern` in preference to `req.getPath()` when present, making upstream path rewriting observable.

## Middleware chain ordering
- Stage 1 of 6: must run before deserialization so `context.operation` is available.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If spec constants or parameter defaults change, re-check dispatch ambiguity because the scoring algorithm depends on required query/header constraints.
