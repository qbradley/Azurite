# Porting Record — `src/blob/generated/middleware/deserializer.middleware.ts`

## File info
- Source path: `src/blob/generated/middleware/deserializer.middleware.ts`
- Source lines: `59`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/deserializer.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::deserializer`
- Phase: `5.22`
- Status: `analyzed`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default function deserializerMiddleware(
  context: Context,
  req: IRequest,
  next: NextFunction,
  logger: ILogger
): void {
  logger.verbose(
    `DeserializerMiddleware: Start deserializing...`,
    context.contextId
  );

  if (context.operation === undefined) {
    const handlerError = new OperationMismatchError();
    logger.error(
      `DeserializerMiddleware: ${handlerError.message}`,
      context.contextId
    );
    return next(handlerError);
  }

  if (Specifications[context.operation] === undefined) {
    logger.warn(
      `DeserializerMiddleware: Cannot find deserializer for operation ${
        Operation[context.operation]
      }`
    );
  }

  deserialize(context, req, Specifications[context.operation], logger)
    .then(parameters => {
      context.handlerParameters = parameters;
    })
    .then(next)
    .catch(err => {
      const deserializationError = new DeserializationError(err.message);
      deserializationError.stack = err.stack;
      next(deserializationError);
    });
}
```

## Dependencies
- Internal imports:
  - `../artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `../artifacts/specifications` → `src/blob/generated/artifacts/specifications.ts` — Phase 5 — analyzed in this pass
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../errors/DeserializationError` → `src/blob/generated/errors/DeserializationError.ts` — Phase 5 — analyzed in this pass
  - `../errors/OperationMismatchError` → `src/blob/generated/errors/OperationMismatchError.ts` — Phase 5 — analyzed in this pass
  - `../IRequest` → `src/blob/generated/IRequest.ts` — Phase 5 — analyzed in this pass
  - `../MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass
  - `../utils/serializer` → `src/blob/generated/utils/serializer.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `Promise<IHandlerParameters>` | `async fn -> HandlerParametersMap` | The map remains dynamically keyed by parameter names. |
| `OperationMismatchError` / `DeserializationError` | `typed middleware error variants` | Operation guard is a 500; body/query/header failures become 400. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Guards that `context.operation` exists, warns when no spec is found, then calls `deserialize(...)` and stores the result in `context.handlerParameters`.
- Any thrown error is wrapped as `DeserializationError(err.message)` while preserving the original stack.

## Middleware chain ordering
- Stage 2 of 6: runs after dispatch and before handler invocation.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If deserializer return shape changes, keep `Context.handlerParameters` and `HandlerMiddlewareFactory` argument extraction in sync.
