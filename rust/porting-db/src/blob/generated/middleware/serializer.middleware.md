# Porting Record — `src/blob/generated/middleware/serializer.middleware.ts`

## File info
- Source path: `src/blob/generated/middleware/serializer.middleware.ts`
- Source lines: `57`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/serializer.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::serializer`
- Phase: `5.23`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default function serializerMiddleware(
  context: Context,
  res: IResponse,
  next: NextFunction,
  logger: ILogger
): void {
  logger.verbose(
    `SerializerMiddleware: Start serializing...`,
    context.contextId
  );

  if (context.operation === undefined) {
    const handlerError = new OperationMismatchError();
    logger.error(
      `SerializerMiddleware: ${handlerError.message}`,
      context.contextId
    );
    return next(handlerError);
  }

  if (Specifications[context.operation] === undefined) {
    logger.warn(
      `SerializerMiddleware: Cannot find serializer for operation ${
        Operation[context.operation]
      }`,
      context.contextId
    );
  }

  serialize(
    context,
    res,
    Specifications[context.operation],
    context.handlerResponses,
    logger
  )
    .then(next)
    .catch(next);
}
```

## Dependencies
- Internal imports:
  - `../artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `../artifacts/specifications` → `src/blob/generated/artifacts/specifications.ts` — Phase 5 — analyzed in this pass
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../errors/OperationMismatchError` → `src/blob/generated/errors/OperationMismatchError.ts` — Phase 5 — analyzed in this pass
  - `../IResponse` → `src/blob/generated/IResponse.ts` — Phase 5 — analyzed in this pass
  - `../MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass
  - `../utils/serializer` → `src/blob/generated/utils/serializer.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `context.handlerResponses: any` | `response enum / boxed response object` | Whatever handlers return must still expose `statusCode` and any body/header fields expected by the spec. |
| `Promise<void>` serialization | `async fn -> Result<()>` | The actual work is delegated to `utils/serializer.ts`. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Guards that `context.operation` exists, warns when no spec is found, then calls `serialize(context, res, spec, context.handlerResponses, logger)`.
- Successful completion calls `next()`; any error bubbles via `next(err)` to generated error middleware.

## Middleware chain ordering
- Stage 4 of 6: runs after handler middleware has produced `context.handlerResponses` and before error/end finalization.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If handler response wrappers or spec status-code maps change, update this stage and `utils/serializer.ts` together.
