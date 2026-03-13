# Porting Record — `src/blob/generated/MiddlewareFactory.ts`

## File info
- Source path: `src/blob/generated/MiddlewareFactory.ts`
- Source lines: `91`
- Source type: `autorest-generated`
- Rust target (per `PORTING-ORDER.md`): `azurite-blob/src/generated/middleware_factory.rs`
- Crate: `azurite-blob`
- Module: `generated::middleware_factory`
- Phase: `5.17`
- Status: `ported`

## Exported API
- Full exported declarations copied below for fidelity reference.

```ts
export type Callback = (...args: any[]) => any;

export type MiddlewareTypes = Callback;

export type NextFunction = Callback;

export default abstract class MiddlewareFactory {
  /**
   * Creates an instance of MiddlewareFactory.
   *
   * @param {ILogger} logger A valid logger
   * @memberof MiddlewareFactory
   */
  public constructor(protected readonly logger: ILogger) {}

  /**
   * DispatchMiddleware is the 1s middleware should be used among other generated middleware.
   *
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createDispatchMiddleware(): MiddlewareTypes;

  /**
   * DeserializerMiddleware is the 2nd middleware should be used among other generated middleware.
   *
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createDeserializerMiddleware(): MiddlewareTypes;

  /**
   * HandlerMiddleware is the 3rd middleware should be used among other generated middleware.
   *
   * @param {IHandlers} handlers
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createHandlerMiddleware(handlers: IHandlers): MiddlewareTypes;

  /**
   * SerializerMiddleware is the 4st middleware should be used among other generated middleware.
   *
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createSerializerMiddleware(): MiddlewareTypes;

  /**
   * ErrorMiddleware is the 5st middleware should be used among other generated middleware.
   *
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createErrorMiddleware(): MiddlewareTypes;

  /**
   * EndMiddleware is the 6st middleware should be used among other generated middleware.
   *
   * @returns {MiddlewareTypes}
   * @memberof MiddlewareFactory
   */
  public abstract createEndMiddleware(): MiddlewareTypes;
}
```

## Dependencies
- Internal imports:
  - `./handlers/IHandlers` → `src/blob/generated/handlers/IHandlers.ts` — Phase 5 — analyzed in this pass
  - `./utils/ILogger` → `src/blob/generated/utils/ILogger.ts` — Phase 5 — analyzed in this pass

## Type mappings
| TypeScript | Recommended Rust | Notes |
|---|---|---|
| `type Callback = (...args: any[]) => any` | typed middleware closure signatures / trait objects | Autorest uses a very loose callback abstraction to stay framework-agnostic. |
| `abstract class MiddlewareFactory` | `trait MiddlewareFactory` plus shared logger state | Need explicit factory methods for six ordered middleware stages. |
## `any` hotspots
- `4: export type Callback = (...args: any[]) => any;`

## Generated-code notes
- This is the abstract contract for the six-stage generated middleware chain: dispatch → deserializer → handler → serializer → error → end.
- The callback types are intentionally `any`-based to accommodate Express/Koa-style wrappers; Rust should narrow each stage to a concrete signature but keep the six distinct factory methods.

## Middleware chain ordering
- Declares the canonical order and stage boundaries. The order is contract-critical and called out in both strategy docs and file comments.

## Change propagation notes
- Because this file is autorest-generated, treat TS regen diffs as contract updates rather than hand edits.
- If the corresponding swagger changes, diff the regenerated TS file first, then update the Rust port and this record together.
- If autorest inserts a new middleware stage, update `ExpressMiddlewareFactory.ts` and every later service listener factory in lockstep.

## Rust port notes
- Ported in `rust/crates/azurite-blob/src/generated/middleware_factory.rs` with the canonical six-stage order preserved as `GENERATED_MIDDLEWARE_ORDER`.
- Forced deviation: the Rust factory narrows the TypeScript callback-style `MiddlewareTypes` contract into explicit stage slots and direct stage helpers instead of untyped Node callback values. This keeps the middleware order visible without inventing faux Express closures before the blob listener pipeline is fully wired.
