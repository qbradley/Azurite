# Porting Record — `src/blob/generated/middleware/HandlerMiddlewareFactory.ts`

## File info
- Source path: `src/blob/generated/middleware/HandlerMiddlewareFactory.ts`
- Source lines: `90`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware/handler_middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware::handler_middleware_factory`
- Phase: `5.24`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class HandlerMiddlewareFactory {
  /**
   * Creates an instance of HandlerMiddlewareFactory.
   * Accept handlers and create handler middleware.
   *
   * @param {IHandlers} handlers Handlers implemented handler interfaces
   * @param {ILogger} logger A valid logger
   * @memberof HandlerMiddlewareFactory
   */
  constructor(
    private readonly handlers: IHandlers,
    private readonly logger: ILogger
  ) {}

  /**
   * Creates a handler middleware from input handlers.
   *
   * @memberof HandlerMiddlewareFactory
   */
  public createHandlerMiddleware(): (
    context: Context,
    next: NextFunction
  ) => void {
    return (context: Context, next: NextFunction) => {
      this.logger.info(
        `HandlerMiddleware: DeserializedParameters=${JSON.stringify(
          context.handlerParameters,
          (key, value) => {
            if (key === "body") {
              return "ReadableStream";
            }
            return value;
          }
        )}`,
        context.contextId
      );

      if (context.operation === undefined) {
        const handlerError = new OperationMismatchError();
        this.logger.error(
          `HandlerMiddleware: ${handlerError.message}`,
          context.contextId
        );
        return next(handlerError);
      }

      if (Specifications[context.operation] === undefined) {
        this.logger.warn(
          `HandlerMiddleware: cannot find handler for operation ${
            Operation[context.operation]
          }`
        );
      }

      // We assume handlerPath always exists for every generated operation in generated code
      const handlerPath = getHandlerByOperation(context.operation)!;

      const args = [];
      for (const arg of handlerPath.arguments) {
        args.push(context.handlerParameters![arg]);
      }
      args.push(context);

      const handler = (this.handlers as any)[handlerPath.handler];
      const handlerMethod = handler[handlerPath.method] as () => Promise<any>;
      handlerMethod
        .apply(handler, args as any)
        .then((response: any) => {
          context.handlerResponses = response;
        })
        .then(next)
        .catch(next);
    };
  }
}
```

## Dependencies
- Internal imports:
  - `../artifacts/operation` → `src/blob/generated/artifacts/operation.ts` — Phase 5 — analyzed in this pass
  - `../artifacts/specifications` → `src/blob/generated/artifacts/specifications.ts` — Phase 5 — analyzed in this pass
  - `../Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `../errors/OperationMismatchError` → `src/blob/generated/errors/OperationMismatchError.ts` — Phase 5 — analyzed in this pass
  - `../handlers/handlerMappers` → `src/blob/generated/handlers/handlerMappers.ts` — Phase 5 — analyzed in this pass
  - `../handlers/IHandlers` → `src/blob/generated/handlers/IHandlers.ts` — Phase 5 — analyzed in this pass
  - `../MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `../utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `(this.handlers as any)[handlerPath.handler]` | typed handler registry lookup or enum-indexed dispatch | Dynamic handler-family lookup is string-based today. |
| `handler[method] as () => Promise<any>` | typed async function pointer / trait-method adapter | Return value is stored back into `context.handlerResponses`. |
| `args as any` | runtime-built argument vector / tuple adapter | Argument order comes from `handlerMappers.ts`. |
## `any` hotspots
- `79: const handler = (this.handlers as any)[handlerPath.handler];`
- `80: const handlerMethod = handler[handlerPath.method] as () => Promise<any>;`
- `82: .apply(handler, args as any)`
- `83: .then((response: any) => {`

## Generated-code notes
- Logs deserialized parameters with a replacer that turns any `body` property into literal text `ReadableStream`; preserve this logging quirk if logger parity matters.
- Builds the handler call by iterating `handlerPath.arguments`, pulling each named value from `context.handlerParameters`, then appending `context` as the final argument.
- Treats missing handler paths as impossible (`getHandlerByOperation(...)!`).

## Middleware chain ordering
- Stage 3 of 6: bridges deserialized parameter bags into typed handler trait calls and stores the result in `context.handlerResponses`.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If argument names or order change anywhere in generated artifacts, revisit this factory first because it is the dynamic pivot between deserializer output and handler trait signatures.
