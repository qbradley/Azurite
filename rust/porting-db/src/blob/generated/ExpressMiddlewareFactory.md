# Porting Record — `src/blob/generated/ExpressMiddlewareFactory.ts`

## File info
- Source path: `src/blob/generated/ExpressMiddlewareFactory.ts`
- Source lines: `162`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/express_middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `generated::express_middleware_factory`
- Phase: `5.18`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export default class ExpressMiddlewareFactory extends MiddlewareFactory {
  /**
   * Creates an instance of MiddlewareFactory.
   *
   * @param {ILogger} logger A valid logger
   * @param {string} [contextPath="default_context"] Optional. res.locals[contextPath] will be used to hold context
   * @memberof MiddlewareFactory
   */
  public constructor(
    logger: ILogger,
    private readonly contextPath: string = "default_context"
  ) {
    super(logger);
  }

  /**
   * DispatchMiddleware is the 1s middleware should be used among other generated middleware.
   *
   * @returns {RequestHandler}
   * @memberof MiddlewareFactory
   */
  public createDispatchMiddleware(): RequestHandler {
    return (req: Request, res: Response, next: NextFunction) => {
      req.baseUrl
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      dispatchMiddleware(
        new Context(res.locals, this.contextPath, request, response),
        request,
        next,
        this.logger
      );
    };
  }

  /**
   * DeserializerMiddleware is the 2nd middleware should be used among other generated middleware.
   *
   * @returns {RequestHandler}
   * @memberof MiddlewareFactory
   */
  public createDeserializerMiddleware(): RequestHandler {
    return (req: Request, res: Response, next: NextFunction) => {
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      deserializerMiddleware(
        new Context(res.locals, this.contextPath, request, response),
        request,
        next,
        this.logger
      );
    };
  }

  /**
   * HandlerMiddleware is the 3rd middleware should be used among other generated middleware.
   *
   * @param {IHandlers} handlers
   * @returns {RequestHandler}
   * @memberof MiddlewareFactory
   */
  public createHandlerMiddleware(handlers: IHandlers): RequestHandler {
    const handlerMiddlewareFactory = new HandlerMiddlewareFactory(
      handlers,
      this.logger
    );
    return (req: Request, res: Response, next: NextFunction) => {
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      handlerMiddlewareFactory.createHandlerMiddleware()(
        new Context(res.locals, this.contextPath, request, response),
        next
      );
    };
  }

  /**
   * SerializerMiddleware is the 4st middleware should be used among other generated middleware.
   *
   * @returns {RequestHandler}
   * @memberof MiddlewareFactory
   */
  public createSerializerMiddleware(): RequestHandler {
    return (req: Request, res: Response, next: NextFunction) => {
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      serializerMiddleware(
        new Context(res.locals, this.contextPath, request, response),
        new ExpressResponseAdapter(res),
        next,
        this.logger
      );
    };
  }

  /**
   * ErrorMiddleware is the 5st middleware should be used among other generated middleware.
   *
   * @returns {ErrorRequestHandler}
   * @memberof MiddlewareFactory
   */
  public createErrorMiddleware(): ErrorRequestHandler {
    return (err: Error, req: Request, res: Response, next: NextFunction) => {
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      errorMiddleware(
        new Context(res.locals, this.contextPath, request, response),
        err,
        new ExpressRequestAdapter(req),
        new ExpressResponseAdapter(res),
        next,
        this.logger
      );
    };
  }

  /**
   * EndMiddleware is the 6st middleware should be used among other generated middleware.
   *
   * @returns {RequestHandler}
   * @memberof MiddlewareFactory
   */
  public createEndMiddleware(): RequestHandler {
    return (req: Request, res: Response) => {
      const request = new ExpressRequestAdapter(req);
      const response = new ExpressResponseAdapter(res);
      endMiddleware(
        new Context(res.locals, this.contextPath, request, response),
        new ExpressResponseAdapter(res),
        this.logger
      );
    };
  }
}
```

## Dependencies
- External packages:
  - `express` — imported as `{ ErrorRequestHandler, NextFunction, Request, RequestHandler, Response }`
- Internal imports:
  - `./Context` → `src/blob/generated/Context.ts` — Phase 5 — analyzed in this pass
  - `./ExpressRequestAdapter` → `src/blob/generated/ExpressRequestAdapter.ts` — Phase 5 — analyzed in this pass
  - `./ExpressResponseAdapter` → `src/blob/generated/ExpressResponseAdapter.ts` — Phase 5 — analyzed in this pass
  - `./handlers/IHandlers` → `src/blob/generated/handlers/IHandlers.ts` — Phase 5 — analyzed in this pass
  - `./middleware/deserializer.middleware` → `src/blob/generated/middleware/deserializer.middleware.ts` — Phase 5 — analyzed in this pass
  - `./middleware/dispatch.middleware` → `src/blob/generated/middleware/dispatch.middleware.ts` — Phase 5 — analyzed in this pass
  - `./middleware/end.middleware` → `src/blob/generated/middleware/end.middleware.ts` — Phase 5 — analyzed in this pass
  - `./middleware/error.middleware` → `src/blob/generated/middleware/error.middleware.ts` — Phase 5 — analyzed in this pass
  - `./middleware/HandlerMiddlewareFactory` → `src/blob/generated/middleware/HandlerMiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `./middleware/serializer.middleware` → `src/blob/generated/middleware/serializer.middleware.ts` — Phase 5 — analyzed in this pass
  - `./MiddlewareFactory` → `src/blob/generated/MiddlewareFactory.ts` — Phase 5 — analyzed in this pass
  - `./utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `express.RequestHandler` / `ErrorRequestHandler` | `axum/tower middleware layer` or equivalent closures | Need separate normal-error stage signatures. |
| `res.locals[contextPath]` context storage | `request/response extensions map keyed by context path` | This is how per-request generated state survives across recreated adapters. |
| `new ExpressRequestAdapter(req)` / `new ExpressResponseAdapter(res)` per call | fresh wrapper structs over shared underlying request/response state | Do not accidentally clone request bodies or response state. |
## `any` hotspots
- No runtime `any` usages in this file beyond literal message text/comments.

## Generated-code notes
- Concrete Express implementation of `MiddlewareFactory`. Each factory method re-wraps Express `req`/`res`, constructs a new generated `Context`, and delegates to the corresponding generated middleware/helper.
- Required stage order is repeated here verbatim: dispatch, deserializer, handler, serializer, error, end.
- Error stage constructs both adapters twice (`const request/response` plus fresh adapters in the call) — preserve the observable behavior even if Rust later avoids redundant wrapper allocation internally.

## Middleware chain ordering
- Stage 1 `createDispatchMiddleware()`
- Stage 2 `createDeserializerMiddleware()`
- Stage 3 `createHandlerMiddleware(handlers)`
- Stage 4 `createSerializerMiddleware()`
- Stage 5 `createErrorMiddleware()`
- Stage 6 `createEndMiddleware()`

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If context construction or adapter wiring changes here, audit every middleware stage because they all assume the same `Context` holder path and wrapper behavior.

## Rust port notes
- Ported in `rust/crates/azurite-blob/src/generated/express_middleware_factory.rs` using generated request/response adapters plus a shared `ContextHolder`.
- Forced deviation: until the surrounding blob listener wiring lands, the Rust port exposes direct stage methods (`dispatch`, `deserialize`, `handle`, `serialize`, `error`, `end`) while the abstract `create*Middleware` slots remain marker methods. The six-stage order and per-request context reconstruction still match the generated TypeScript flow.
